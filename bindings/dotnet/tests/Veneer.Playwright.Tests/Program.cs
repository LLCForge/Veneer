using System;
using System.Collections.Concurrent;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using Microsoft.Playwright;
using Veneer;
using Veneer.Playwright;

var seen = new ConcurrentQueue<WebHeaderCollection>();
var port = FreePort();
using var listener = new HttpListener();
listener.Prefixes.Add($"http://127.0.0.1:{port}/");
listener.Start();
_ = Task.Run(async () =>
{
    while (listener.IsListening)
    {
        HttpListenerContext ctx;
        try
        {
            ctx = await listener.GetContextAsync().ConfigureAwait(false);
        }
        catch (HttpListenerException)
        {
            return;
        }
        var headers = new WebHeaderCollection();
        foreach (string key in ctx.Request.Headers)
        {
            headers[key.ToLowerInvariant()] = ctx.Request.Headers[key];
        }
        seen.Enqueue(headers);
        var body = Encoding.UTF8.GetBytes("<!doctype html><title>veneer</title>");
        ctx.Response.ContentType = "text/html";
        ctx.Response.ContentLength64 = body.Length;
        await ctx.Response.OutputStream.WriteAsync(body).ConfigureAwait(false);
        ctx.Response.Close();
    }
});

using var playwright = await Playwright.CreateAsync();
foreach (var constraints in new[]
{
    new JsonObject { ["os"] = "windows", ["browser"] = "chrome", ["seed"] = "dotnet-e2e" },
    new JsonObject { ["os"] = "android", ["browser"] = "chrome", ["seed"] = "dotnet-e2e" },
})
{
    seen.Clear();
    var profile = Fingerprints.Generate(constraints);
    var (browser, _, page) = await VeneerPlaywright.LaunchWithProfileAsync(
        playwright, profile, new BrowserTypeLaunchOptions { ExecutablePath = Environment.GetEnvironmentVariable("VENEER_CHROME") });
    try
    {
        await page.GotoAsync($"http://127.0.0.1:{port}/");
        var report = await VeneerPlaywright.VerifyPageAsync(page, profile);
        if (!report.Valid)
        {
            throw new InvalidOperationException("live comparison failed: " + string.Join("; ", report.Issues));
        }
    }
    finally
    {
        await browser.CloseAsync();
    }
    var first = seen.First();
    var expected = Fingerprints.NavigationHeaders(profile)
        .Where(h => h.Name is "user-agent" or "accept-language" || h.Name.StartsWith("sec-ch-ua", StringComparison.Ordinal));
    foreach (var h in expected)
    {
        if (first[h.Name] != h.Value)
        {
            throw new InvalidOperationException($"{constraints["os"]}: header {h.Name} = {first[h.Name]}, want {h.Value}");
        }
    }
    Console.WriteLine($"ok   {constraints["os"]}: live comparison and request headers match");
}
listener.Stop();
Console.WriteLine("all checks passed");

static int FreePort()
{
    using var socket = new TcpListener(IPAddress.Loopback, 0);
    socket.Start();
    return ((IPEndPoint)socket.LocalEndpoint).Port;
}
