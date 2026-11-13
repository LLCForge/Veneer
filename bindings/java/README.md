# Veneer for Java

Java 22+ bindings for [Veneer](../../README.md), using the built-in Foreign Function & Memory API over the C ABI library from [`crates/veneer-ffi`](../../crates/veneer-ffi).

| Class | Purpose |
| --- | --- |
| `io.github.llcforge.veneer.Veneer` | `generate`, `validate`, `initScript`, `navigationHeaders`, `catalog`, `collectorScript`, `compareObserved` |
| `...veneer.playwright.VeneerPlaywright` | `newContext`, `newPage`, `launchWithProfile` for Playwright Java |
| `...veneer.selenium.VeneerSelenium` | `apply(driver, profile)` for Chromium-based Selenium drivers (CDP, current tab) |

## Setup

```bash
cargo build -p veneer-ffi --release
```

Set `VENEER_FFI_LIB` to the built library and run Java with `--enable-native-access=ALL-UNNAMED`.

## Usage

```java
ObjectNode profile = Veneer.generate(Map.of("os", "macos", "browser", "chrome", "seed", "session-42"));
try (Playwright playwright = Playwright.create()) {
  VeneerPlaywright.Launched run =
      VeneerPlaywright.launchWithProfile(playwright, profile, new BrowserType.LaunchOptions());
  run.page().navigate("https://example.com");
  run.browser().close();
}
```

## Tests

```bash
cd bindings/java && mvn -B verify
```

These bindings are built and tested in CI; they were not executed on the machine they were written on.
