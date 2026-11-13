using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using Microsoft.Playwright;

namespace Veneer.Playwright;

/// <summary>
/// Playwright for .NET helpers, mirroring <c>newContext</c> / <c>newPage</c> / <c>launchWithProfile</c> /
/// <c>verifyPage</c> of the npm package.
/// </summary>
public static class VeneerPlaywright
{
    /// <summary>Playwright browser type whose engine matches the profile: chromium, firefox or webkit.</summary>
    public static IBrowserType BrowserType(IPlaywright playwright, JsonObject profile)
    {
        ArgumentNullException.ThrowIfNull(playwright);
        ArgumentNullException.ThrowIfNull(profile);
        return profile["browser"]?["engine"]?.GetValue<string>() switch
        {
            "chromium" => playwright.Chromium,
            "gecko" => playwright.Firefox,
            "webkit" => playwright.Webkit,
            var other => throw new VeneerException($"unknown engine {other}"),
        };
    }

    /// <summary>Options for <see cref="IBrowser.NewContextAsync"/>.</summary>
    public static BrowserNewContextOptions ContextOptions(JsonObject profile)
    {
        ArgumentNullException.ThrowIfNull(profile);
        var screen = profile["screen"] ?? throw new VeneerException("profile has no screen");
        var viewport = profile["viewport"] ?? throw new VeneerException("profile has no viewport");
        var options = new BrowserNewContextOptions
        {
            UserAgent = profile["navigator"]?["userAgent"]?.GetValue<string>(),
            Locale = profile["locale"]?.GetValue<string>(),
            TimezoneId = profile["timezone"]?.GetValue<string>(),
            ViewportSize = new ViewportSize { Width = Int(viewport["width"]), Height = Int(viewport["height"]) },
            ScreenSize = new ScreenSize { Width = Int(screen["width"]), Height = Int(screen["height"]) },
            DeviceScaleFactor = (float)(screen["devicePixelRatio"]?.GetValue<double>() ?? 1),
        };
        // Playwright rejects isMobile on Firefox.
        if (Fingerprints.IsHandheld(profile) && profile["browser"]?["engine"]?.GetValue<string>() != "gecko")
        {
            options.IsMobile = true;
            options.HasTouch = true;
        }
        return options;
    }

    private static int Int(JsonNode? node) => node?.GetValue<int>() ?? throw new VeneerException("missing size");

    /// <summary>
    /// A context configured for the profile. On Chromium, popups also get matching request headers via
    /// CDP; the override is applied asynchronously, so a popup's very first request may race it.
    /// </summary>
    public static async Task<IBrowserContext> NewContextAsync(IBrowser browser, JsonObject profile)
    {
        ArgumentNullException.ThrowIfNull(browser);
        var context = await browser.NewContextAsync(ContextOptions(profile)).ConfigureAwait(false);
        await context.AddInitScriptAsync(Fingerprints.InitScript(profile)).ConfigureAwait(false);
        if (browser.BrowserType.Name == "chromium")
        {
            var overrideArgs = OverrideArgs(profile);
            context.Page += (_, page) => _ = ApplyQuietlyAsync(context, page, overrideArgs);
        }
        return context;
    }

    /// <summary>A page whose request headers are overridden before its first navigation.</summary>
    public static async Task<IPage> NewPageAsync(IBrowserContext context, JsonObject profile)
    {
        ArgumentNullException.ThrowIfNull(context);
        var page = await context.NewPageAsync().ConfigureAwait(false);
        if (context.Browser?.BrowserType.Name == "chromium")
        {
            await ApplyAsync(context, page, OverrideArgs(profile)).ConfigureAwait(false);
        }
        return page;
    }

    /// <summary>Launches the engine that matches the profile and returns the browser, context and page.</summary>
    public static async Task<(IBrowser Browser, IBrowserContext Context, IPage Page)> LaunchWithProfileAsync(
        IPlaywright playwright, JsonObject profile, BrowserTypeLaunchOptions? launchOptions = null)
    {
        var browser = await BrowserType(playwright, profile).LaunchAsync(launchOptions).ConfigureAwait(false);
        try
        {
            var context = await NewContextAsync(browser, profile).ConfigureAwait(false);
            return (browser, context, await NewPageAsync(context, profile).ConfigureAwait(false));
        }
        catch
        {
            await browser.CloseAsync().ConfigureAwait(false);
            throw;
        }
    }

    /// <summary>Runs the collector in the live page and compares it with the profile.</summary>
    public static async Task<Report> VerifyPageAsync(IPage page, JsonObject profile)
    {
        ArgumentNullException.ThrowIfNull(page);
        // Serialized in the page: EvaluateAsync<JsonElement> adds "$id" reference markers to objects.
        var expression = $"(async () => JSON.stringify(await ({Fingerprints.CollectorScript()})))()";
        var observed = await page.EvaluateAsync<string>(expression).ConfigureAwait(false);
        var node = JsonNode.Parse(observed) ?? throw new VeneerException("collector returned null");
        return Fingerprints.CompareObserved(profile, node);
    }

    private static Dictionary<string, object> OverrideArgs(JsonObject profile) =>
        (Dictionary<string, object>)Plain(Fingerprints.UserAgentOverride(profile))!;

    private static async Task ApplyAsync(IBrowserContext context, IPage page, Dictionary<string, object> args)
    {
        var session = await context.NewCDPSessionAsync(page).ConfigureAwait(false);
        await session.SendAsync("Emulation.setUserAgentOverride", args).ConfigureAwait(false);
    }

    private static async Task ApplyQuietlyAsync(IBrowserContext context, IPage page, Dictionary<string, object> args)
    {
        try
        {
            await ApplyAsync(context, page, args).ConfigureAwait(false);
        }
        catch (PlaywrightException)
        {
            // Page closed before the override could be applied.
        }
    }

    // CDP arguments as plain dictionaries, lists and primitives, which Playwright serializes as-is.
    private static object? Plain(JsonNode? node) => node switch
    {
        null => null,
        JsonObject o => o.Where(kv => kv.Value is not null).ToDictionary(kv => kv.Key, kv => Plain(kv.Value)!),
        JsonArray a => a.Select(Plain).ToList(),
        JsonValue v when v.TryGetValue<bool>(out var b) => b,
        JsonValue v when v.TryGetValue<string>(out var s) => s,
        JsonValue v when v.TryGetValue<long>(out var l) => l,
        JsonValue v => v.GetValue<double>(),
        _ => throw new VeneerException("unsupported JSON node"),
    };
}
