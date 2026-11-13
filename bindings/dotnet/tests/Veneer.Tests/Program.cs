using System;
using System.Linq;
using System.Text.Json.Nodes;
using Veneer;

var profile = Fingerprints.Generate(new JsonObject { ["os"] = "windows", ["browser"] = "chrome", ["seed"] = "dotnet" });
Check(profile["seed"]!.GetValue<string>() == "dotnet", "seed is kept");
var report = Fingerprints.Validate(profile);
Check(report.Valid && report.Issues.Count == 0 && report.Score == 100, "generated profile validates");
var ua = profile["navigator"]!["userAgent"]!.GetValue<string>();
Check(Fingerprints.InitScript(profile).Contains(ua, StringComparison.Ordinal), "init script embeds the user agent");
Check(Fingerprints.NavigationHeaders(profile).Any(h => h.Name == "sec-ch-ua"), "headers include client hints");
Check(Fingerprints.Catalog()["os"] is JsonArray, "catalog lists OSes");
Check(Fingerprints.CollectorScript().Length > 0, "collector script");
Check(!Fingerprints.CompareObserved(profile, new JsonObject { ["userAgent"] = "other" }).Valid, "compare detects mismatch");

var a = Fingerprints.Generate(new JsonObject { ["seed"] = "same" });
var b = Fingerprints.Generate(new JsonObject { ["seed"] = "same" });
Check(JsonNode.DeepEquals(a, b), "same seed, same profile");
Check(Fingerprints.Generate()["seed"] is not null, "random seed");

var phone = Fingerprints.Generate(new JsonObject { ["os"] = "android", ["browser"] = "chrome", ["seed"] = "p" });
var o = Fingerprints.UserAgentOverride(phone);
Check(Fingerprints.IsHandheld(phone) && o["userAgentMetadata"]!["mobile"]!.GetValue<bool>(), "phone override is mobile");
Check(!o["acceptLanguage"]!.GetValue<string>().Contains(";q=", StringComparison.Ordinal), "acceptLanguage is a plain list");
var fx = Fingerprints.Generate(new JsonObject { ["browser"] = "firefox", ["seed"] = "f" });
Check(Fingerprints.UserAgentOverride(fx)["userAgentMetadata"] is null, "no client hints on Firefox");

Check(Throws<VeneerException>(() => Fingerprints.Generate(new JsonObject { ["os"] = "amiga" })), "unknown OS is rejected");
Check(Throws<VeneerException>(() => Fingerprints.Generate(new JsonObject { ["oss"] = "x" })), "unknown key is rejected");
Check(Throws<VeneerException>(() => Fingerprints.Validate(new JsonObject { ["nope"] = 1 })), "malformed profile is rejected");
Check(Throws<ArgumentNullException>(() => Fingerprints.Validate(null!)), "null profile is rejected");
Console.WriteLine("all checks passed");

static void Check(bool condition, string what)
{
    if (!condition)
    {
        throw new InvalidOperationException("FAILED: " + what);
    }
    Console.WriteLine("ok   " + what);
}

static bool Throws<T>(Action action) where T : Exception
{
    try
    {
        action();
        return false;
    }
    catch (T)
    {
        return true;
    }
}
