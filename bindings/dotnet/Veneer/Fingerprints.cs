using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Veneer;

/// <summary>Raised for invalid input or unsatisfiable constraints.</summary>
public sealed class VeneerException : Exception
{
    /// <summary>Creates the exception with the core's message.</summary>
    public VeneerException(string message) : base(message) { }
}

/// <summary>One consistency finding.</summary>
public sealed record Issue(string Rule, string Severity, string Message);

/// <summary>Result of <see cref="Fingerprints.Validate"/> and <see cref="Fingerprints.CompareObserved"/>.</summary>
public sealed record Report(bool Valid, int Score, IReadOnlyList<Issue> Issues);

/// <summary>One request header.</summary>
public sealed record Header(string Name, string Value);

/// <summary>
/// Veneer profiles through the C ABI. Profiles are <see cref="JsonObject"/>s exactly as the core
/// produced them, so no field is lost when they are passed back. The native library is loaded from
/// <c>VENEER_FFI_LIB</c> if set, otherwise from the application directory or the system search path.
/// </summary>
public static class Fingerprints
{
    private static readonly JsonSerializerOptions Json = new(JsonSerializerDefaults.Web);

    static Fingerprints()
    {
        NativeLibrary.SetDllImportResolver(typeof(Fingerprints).Assembly, Resolve);
    }

    private static IntPtr Resolve(string name, Assembly assembly, DllImportSearchPath? path)
    {
        var explicitPath = Environment.GetEnvironmentVariable("VENEER_FFI_LIB");
        if (name == Native.Library && !string.IsNullOrEmpty(explicitPath))
        {
            return NativeLibrary.Load(explicitPath);
        }
        return IntPtr.Zero;
    }

    private static JsonNode? Call(IntPtr result, string function)
    {
        if (result == IntPtr.Zero)
        {
            throw new VeneerException($"{function} could not allocate its result");
        }
        string text;
        try
        {
            text = Marshal.PtrToStringUTF8(result) ?? throw new VeneerException($"{function} returned no text");
        }
        finally
        {
            Native.veneer_free(result);
        }
        var envelope = JsonNode.Parse(text)?.AsObject() ?? throw new VeneerException("malformed result");
        if (envelope.TryGetPropertyValue("error", out var error))
        {
            throw new VeneerException(error?.GetValue<string>() ?? "unknown error");
        }
        return envelope["ok"];
    }

    // NUL-terminated UTF-8. JSON escapes U+0000, so the text never contains an interior NUL.
    private static byte[] Encode(JsonNode node, string what)
    {
        ArgumentNullException.ThrowIfNull(node, what);
        var text = node.ToJsonString();
        var bytes = new byte[Encoding.UTF8.GetByteCount(text) + 1];
        Encoding.UTF8.GetBytes(text, 0, text.Length, bytes, 0);
        return bytes;
    }

    private static T To<T>(JsonNode? node) =>
        node.Deserialize<T>(Json) ?? throw new VeneerException($"unexpected null {typeof(T).Name}");

    /// <summary>
    /// Generates a coherent profile. Keys use the core's camelCase names, e.g.
    /// <c>{"os":"windows","browser":"chrome","seed":"s1"}</c>. Without a seed a random one is stored in <c>profile["seed"]</c>.
    /// </summary>
    public static JsonObject Generate(JsonObject? constraints = null)
    {
        var merged = constraints is null ? new JsonObject() : constraints.DeepClone().AsObject();
        if (merged["seed"] is null)
        {
            merged["seed"] = Guid.NewGuid().ToString();
        }
        return Call(Native.veneer_generate(Encode(merged, nameof(constraints))), "generate")?.AsObject()
            ?? throw new VeneerException("generate returned null");
    }

    /// <summary>Checks a profile against every consistency rule.</summary>
    public static Report Validate(JsonObject profile) =>
        To<Report>(Call(Native.veneer_validate(Encode(profile, nameof(profile))), "validate"));

    /// <summary>Init script to run before any page script.</summary>
    public static string InitScript(JsonObject profile) =>
        To<string>(Call(Native.veneer_init_script(Encode(profile, nameof(profile))), "init script"));

    /// <summary>Navigation request headers in the order the profiled browser sends them.</summary>
    public static IReadOnlyList<Header> NavigationHeaders(JsonObject profile) =>
        To<List<Header>>(Call(Native.veneer_navigation_headers(Encode(profile, nameof(profile))), "headers"));

    /// <summary>Every OS, browser, device and locale the generator knows.</summary>
    public static JsonObject Catalog() =>
        Call(Native.veneer_catalog(), "catalog")?.AsObject() ?? throw new VeneerException("catalog returned null");

    /// <summary>Expression that resolves (as a promise) to what the page can observe.</summary>
    public static string CollectorScript() => To<string>(Call(Native.veneer_collector_script(), "collector"));

    /// <summary>Compares collector output from a real browser against the profile.</summary>
    public static Report CompareObserved(JsonObject profile, JsonNode observed) =>
        To<Report>(Call(
            Native.veneer_compare_observed(Encode(profile, nameof(profile)), Encode(observed, nameof(observed))),
            "compare"));

    /// <summary>
    /// CDP <c>Emulation.setUserAgentOverride</c> parameters: user agent, Accept-Language, platform and,
    /// for browsers with client hints, <c>userAgentMetadata</c>.
    /// </summary>
    public static JsonObject UserAgentOverride(JsonObject profile)
    {
        ArgumentNullException.ThrowIfNull(profile);
        var nav = profile["navigator"] as JsonObject ?? throw new VeneerException("profile has no navigator");
        var languages = (nav["languages"] as JsonArray)?.Select(l => l?.GetValue<string>() ?? string.Empty)
            ?? Enumerable.Empty<string>();
        var result = new JsonObject
        {
            ["userAgent"] = nav["userAgent"]?.DeepClone(),
            // Chromium appends the q-values itself; passing the finished header would double them.
            ["acceptLanguage"] = string.Join(",", languages),
            ["platform"] = nav["platform"]?.DeepClone(),
        };
        if (nav["userAgentData"] is JsonObject u)
        {
            result["userAgentMetadata"] = new JsonObject
            {
                ["brands"] = u["brands"]?.DeepClone(),
                ["fullVersionList"] = u["fullVersionList"]?.DeepClone(),
                ["fullVersion"] = u["uaFullVersion"]?.DeepClone(),
                ["platform"] = u["platform"]?.DeepClone(),
                ["platformVersion"] = u["platformVersion"]?.DeepClone(),
                ["architecture"] = u["architecture"]?.DeepClone(),
                ["model"] = u["model"]?.DeepClone(),
                ["mobile"] = u["mobile"]?.DeepClone(),
                ["bitness"] = u["bitness"]?.DeepClone(),
                ["wow64"] = u["wow64"]?.DeepClone(),
                ["formFactors"] = u["formFactors"]?.DeepClone(),
            };
        }
        return result;
    }

    /// <summary><c>true</c> for phone and tablet profiles.</summary>
    public static bool IsHandheld(JsonObject profile)
    {
        ArgumentNullException.ThrowIfNull(profile);
        var kind = profile["device"]?["kind"]?.GetValue<string>();
        return kind is "phone" or "tablet";
    }

    private static class Native
    {
        public const string Library = "veneer_ffi";

        [DllImport(Library, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
        public static extern IntPtr veneer_generate(byte[] constraintsJson);

        [DllImport(Library, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
        public static extern IntPtr veneer_validate(byte[] profileJson);

        [DllImport(Library, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
        public static extern IntPtr veneer_init_script(byte[] profileJson);

        [DllImport(Library, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
        public static extern IntPtr veneer_navigation_headers(byte[] profileJson);

        [DllImport(Library, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
        public static extern IntPtr veneer_catalog();

        [DllImport(Library, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
        public static extern IntPtr veneer_collector_script();

        [DllImport(Library, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
        public static extern IntPtr veneer_compare_observed(
            byte[] profileJson,
            byte[] observedJson);

        [DllImport(Library, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
        public static extern void veneer_free(IntPtr ptr);
    }
}
