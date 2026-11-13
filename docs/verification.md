# Verifying device data

Veneer only ships values that come from a checkable source. For phones and desktops, vendor pages and public user-agent
samples are usually enough. For consoles, TVs, VR headsets and in-app browsers they are not: the values that matter
(user agent, screen, WebGL renderer, client hints) are only visible on the device itself. This guide shows owners of
those devices how to read them and send them to us, and how maintainers review them.

> **Türkçe özet:** Cihazınızda (konsol, TV, VR, uygulama içi tarayıcı) `veneer serve` tarafından sunulan `/contribute`
> sayfasını açın, **Raporu oluştur** düğmesine basın ve çıkan JSON'u GitHub'daki **Device report** şablonuna yapıştırın.
> Rapor yalnızca donanım ve tarayıcı özelliklerini içerir; IP, çerez, saat dilimi, kimlik ve serbest metin içermez.
> Sunucu hiçbir şey saklamaz ve hiçbir yere göndermez.

## What the report contains

The `/contribute` page runs the same collector that Veneer's live checks use, then keeps only an allowlist of fields:

| Kept | Never included |
| --- | --- |
| `userAgent`, `appVersion`, `platform`, `vendor`, `oscpu` | IP address (the server does not log it, the page never sends anything) |
| `languages` (browser language list) | cookies, storage, account names |
| `hardwareConcurrency`, `deviceMemory`, `maxTouchPoints` | timezone |
| `screen` (size, available size, colour depth), `devicePixelRatio` | canvas and audio fingerprints, hashes, identifiers |
| `webgl` vendor and renderer strings | free text |
| `userAgentData` (client hints: brands, platform, model, platform version, architecture) | anything a newer collector adds before it is reviewed |
| `webdriver` (must be `false` for a valid report) | |
| media features: `pointer`, `any-pointer`, `hover`, `color-gamut`, `dynamic-range`, `orientation` | |

Strings are cut at 512 characters and lists at 64 items. You see the full report before you copy it, and nothing leaves
the page unless you paste it somewhere yourself.

## Owners: how to collect a report

You need a computer on the same network as the device, and the `veneer` binary
([Releases](https://github.com/LLCForge/Veneer/releases) or `cargo install --git https://github.com/LLCForge/Veneer veneer-cli`).

1. **Start the server so the device can reach it.** Binding to anything other than `127.0.0.1` requires a token for the
   API; the `/contribute` page itself does not need it.

   ```bash
   veneer serve --host 0.0.0.0 --port 8787 --token "$(openssl rand -hex 24)"
   ```

   Find the computer's LAN address (`ipconfig` on Windows, `ip addr` on Linux, `ipconfig getifaddr en0` on macOS).
   Stop the server with Ctrl+C when you are done.

2. **Open the page on the device.** Type `http://<computer-address>:8787/contribute` into the device's browser.

   | Device | How to open the page |
   | --- | --- |
   | PlayStation, Xbox, Nintendo Switch | the system web browser (on some consoles it is only reachable through a help or sign-in screen; use whatever opens a web page) |
   | Smart TV (Tizen, webOS, Android TV) | the built-in web browser app |
   | VR headset | the headset's built-in browser |
   | In-app browser (Instagram, Facebook, …) | send the link to yourself in the app (for example in a direct message) and open it from there, so it opens inside the app |

   The page is available in Turkish and English (TR/EN switch at the top).

3. **Create the report.** Press **Create report**. Read the JSON. If `webdriver` is `true`, the browser is automated and
   the report cannot be used.

4. **Copy it.** Press **Copy JSON**. Over plain `http://` on a LAN address the browser treats the page as insecure: it
   hides client hints (`userAgentData` is `null`) and blocks the clipboard. The page then selects the text so you can copy
   it by hand. Such a report is still useful; for client hints, open the page through `https://` (for example behind a
   local reverse proxy with a trusted certificate) or on `localhost` when the device can run the server itself.

5. **Submit it.** Open a [Device report](https://github.com/LLCForge/Veneer/issues/new?template=device-report.yml)
   issue, fill in the model and the system software version exactly as the device's settings show them, and paste the
   JSON unchanged. Do not attach screenshots, serial numbers or anything that identifies you.

## Reporting a wrong value

If a value in `crates/veneer/data/` is wrong or outdated and you can point to a public source, open a
[Data correction](https://github.com/LLCForge/Veneer/issues/new?template=data-correction.yml) issue with the file, the
current value, the correct value and the source URL. Accepted sources, in order of preference:

1. official vendor documentation, spec pages or developer guides;
2. official release notes and version feeds;
3. `/contribute` reports from real devices;
4. browser source code and public datasets.

Forum posts and user-agent lists without provenance are not enough on their own.

## Maintainers: reviewing a report

1. **Check the report is genuine:** `format` is `veneer-device-report`, `webdriver` is `false`, the user agent, platform
   and client hints agree with each other and with the device class in the issue.
2. **Compare with the dataset:** generate a profile for the same device (`veneer generate --device <id>`) and compare the
   user agent template, screen sizes, touch points, cores, memory and WebGL strings.
3. **Decide the confidence:** one report from a device confirms that a value exists (`medium`); two independent reports,
   or one report plus an official source, make it `high`. Values that differ between reports of the same model and
   firmware need more reports before they are added.
4. **Record the source:** add the data, then a row in `crates/veneer/data/SOURCES.md`
   (data | "device report #<issue number>" | confidence). Do not copy anything from the issue except the values used.
5. **Run the checks:** `cargo test -p veneer` must stay at zero validator issues. The E2E workflow then launches the
   matching browser engine for each profile class and asserts that the collector, request headers, FingerprintJS and the
   lie checks agree with the profile.

## Automated verification

`tests/e2e` holds the browser tests that run on every pull request (`.github/workflows/e2e.yml`):

- **profiles:** for profiles of each engine, launch Chromium, Firefox or WebKit with the profile through the
  `packages/veneer` helpers, run the collector and require zero live errors, check the request headers the server saw,
  run FingerprintJS (open-source, MIT) in the page and compare its components with the profile, and run CreepJS-style
  lie checks (native-looking functions, prototype accessors, iframes, media queries, workers);
- **parity:** build the WASM package with `wasm-bindgen-cli` and require byte-identical profiles, init scripts, request
  headers and validation reports from the WASM build and the native `veneer` binary for the same seeds;
- **contribute:** open `/contribute` and check the report keeps only allowlisted fields and sends nothing.

Run them locally:

```bash
cd packages/veneer && npm run build && cd ../../tests/e2e
npm ci
npx playwright install chromium        # or firefox, webkit
npx playwright test --project=chromium
VENEER_BIN=../../target/release/veneer node --test parity.test.mjs
```

The contribute test runs when `VENEER_API` points at a running `veneer serve` (for example `http://127.0.0.1:8787`).
