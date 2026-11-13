# Veneer for .NET

.NET 8 bindings for [Veneer](../../README.md), using P/Invoke over the C ABI library from [`crates/veneer-ffi`](../../crates/veneer-ffi).

| Project | Purpose |
| --- | --- |
| `Veneer` | `Fingerprints.Generate`, `Validate`, `InitScript`, `NavigationHeaders`, `Catalog`, `CollectorScript`, `CompareObserved` (no dependencies) |
| `Veneer.Playwright` | `VeneerPlaywright.NewContextAsync`, `NewPageAsync`, `LaunchWithProfileAsync`, `VerifyPageAsync` for Microsoft.Playwright |

## Setup

```bash
cargo build -p veneer-ffi --release
```

Set `VENEER_FFI_LIB` to the built library, or place it next to your application.

## Usage

```csharp
using System.Text.Json.Nodes;
using Microsoft.Playwright;
using Veneer;
using Veneer.Playwright;

var profile = Fingerprints.Generate(new JsonObject { ["os"] = "windows", ["browser"] = "chrome", ["seed"] = "session-42" });
using var playwright = await Playwright.CreateAsync();
var (browser, context, page) = await VeneerPlaywright.LaunchWithProfileAsync(playwright, profile);
await page.GotoAsync("https://example.com");
var report = await VeneerPlaywright.VerifyPageAsync(page, profile);
await browser.CloseAsync();
```

## Tests

```bash
cd bindings/dotnet && dotnet run --project tests/Veneer.Tests
```
