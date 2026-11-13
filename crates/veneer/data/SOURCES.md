# Dataset sources

The dataset is hand-curated. It contains no personal data — only aggregate hardware and software characteristics.

Each group is tagged with a confidence level so contributors know where verification is most valuable:

- **high** — taken from vendor release notes or widely documented, stable behavior
- **medium** — consistent with public user-agent databases and observed traffic, not verified on every device
- **low** — best available public information; needs verification on real hardware

| Data | Source | Confidence |
| --- | --- | --- |
| Chrome, Edge, Firefox, Brave, Chrome for iOS, Firefox for iOS versions | Official feeds via `scripts/update-versions.mjs`: Chrome Version History API, Edge update feed, Mozilla product-details (refreshed weekly) | high |
| Safari 27 / 26.5 and the frozen `iPhone OS 18_7` token | WebKit bug tracker and public coverage of the iOS 26+ user-agent freeze | medium |
| Safari version pinned per older iOS (15.8 → 15.6, 16.7 → 16.6, 17.7 → 17.6, 18.7 → 18.6) | Observed Safari user agents on those iOS releases | medium |
| Which iOS / Android versions each device runs | Vendor support lists and launch software versions | medium |
| iPhone 17 / Air / 18 Pro / 18 Pro Max screens; Galaxy S26 family and Z Fold8 Ultra launch details | Apple and Samsung spec pages, iFixit, GSMArena | high |
| Apple hardware ids (`iPhone17,1`, …) | Public device-identifier references; iPhone 18 ids are not yet known and are omitted | medium |
| Galaxy Z Fold8 / Fold8 Ultra model numbers and CSS viewport sizes | Single sources and estimates from panel resolutions | low |
| Brave client-hint full versions (`N.0.0.0`) | Brave's reduced client hints | medium |
| Instagram and Facebook in-app user agents and app versions | Public user-agent databases | low |
| Pixel 10 GPU string, Snapdragon X and Intel Mac WebGL strings | Public device databases | low |
| Reduced Chromium UA strings (`Chrome/N.0.0.0`, `Android 10; K`, `CrOS x86_64 14541.0.0`) | Chromium User-Agent Reduction documentation | high |
| GREASE brand algorithm | Chromium source (`user_agent_utils.cc`); unit-tested against real Chrome 120, 131 and 132 headers | high |
| Windows / macOS / Linux desktop WebGL strings (ANGLE format) | Public fingerprint galleries and ANGLE renderer string format | medium |
| Firefox sanitized WebGL renderers (`..., or similar`) | Firefox `webgl.sanitize-unmasked-renderer` behavior | medium |
| Screen sizes, pixel ratios, core and memory counts | Manufacturer specifications | high |
| Taskbar / menu bar insets, toolbar heights | Measured defaults; vary with user settings | medium |
| Windows, macOS, Linux, Android, iOS font lists | Default fonts shipped with each OS | medium |
| Android phone GPU strings | Public device databases | medium |
| Samsung Internet version | Public release history | low |
| Xbox, PlayStation, Nintendo Switch user agents | Public user-agent databases | low |
| Samsung Tizen and LG webOS TV user agents | Platform developer documentation and user-agent databases | low |
| Meta Quest Browser user agent | Public user-agent databases | low |
| Console, TV and VR WebGL strings and font lists | Not publicly documented; left empty or approximate | low |
| Weights (relative frequencies) | Estimates informed by public market-share statistics | medium |

## P2 special devices (handhelds, Fire, Vision Pro)

| Data | Source | Confidence |
| --- | --- | --- |
| Amazon Silk UA format on Fire OS 8 (`Android 11; <model>`, `Silk/x like Chrome/y Safari/537.36`, no `Build/`, no `Mobile`) | AWS Silk developer guide (docs.aws.amazon.com/silk/latest/developerguide/user-agent.html) + real UAs on user-agents.net/browsers/silk (KFTUWI, KFSNWI, KFRAPWI, AFTKM, AFTKRT) | high |
| Silk versions 152.7.9 / 152.0.7977.158, 150.7.2 / 150.0.7871.253, 148.4.3 / 148.0.7778.265 | user-agents.net/browsers/silk (same build on Fire tablets and Fire TV) | medium |
| Fire HD 8 model codes (KFRAWI 2 GB, KFRAPWI 3 GB, KFRASWI 4 GB), 6 cores, Fire OS 8 / Android 11, 1280x800 tvdpi, 5-point touch | developer.amazon.com/docs/device-specs/ft-device-specifications-firehd-models.html | high |
| Fire HD 8 CSS screen 601x962 at dpr 1.33125 | Derived from 1280x800 tvdpi (213/160); rounding of the CSS size not observed on a device | medium |
| Fire TV Stick 4K 2nd gen (AFTKM) and 4K Max 2nd gen (AFTKRT): 4 cores, 2 GB, Fire OS 8 | developer.amazon.com/docs/device-specs/device-specifications-fire-tv-streaming-media-player.html | high |
| Fire TV CSS screen 960x540 at dpr 2 | Amazon Fire TV "Display and Layout" docs (960x540 dp, xhdpi) | medium |
| Fire OS `navigator.platform` `Linux armv81` | Same frozen value as Chromium on Android | medium |
| Silk toolbar heights | Same estimate as other Chromium Android browsers | low |
| Steam Deck LCD/OLED: 4 cores / 8 threads, 16 GB, 1280x800, touchscreen | Wikipedia "Steam Deck" (Valve spec) | high |
| Steam Deck Firefox `maxTouchPoints` 0 | Mozilla bug 1870900 comment 6 | high |
| Steam Deck desktop mode at scale 1 (1280x800 CSS) | Reviews describing the unscaled KDE desktop; no Valve statement | medium |
| ROG Ally / Ally X: Z1 6c/12t, Z1 Extreme 8c/16t, 16 GB (Ally X 24 GB), 7" 1920x1080, 10-point touch, Windows 11 | rog.asus.com RC71L spec page, Wikipedia "ROG Ally" | high |
| ROG Ally `maxTouchPoints` 10 in Chromium | Chromium reports the digitizer contact count on Windows; 10-point panel per ASUS | medium |
| ROG Ally Windows scaling 150% (1280x720 CSS, dpr 1.5) | Secondary guides describe 150% as the default | medium |
| Vision Pro Safari `maxTouchPoints` 5 (not shipped as a device: screen size unverified) | WebKit `Navigator.cpp` (returns 5 under `ENABLE(IOS_TOUCH_EVENTS)`, which covers visionOS) | medium |
| WebGL strings for Steam Deck, ROG Ally, Fire tablets and Fire TV | No verified real sample found; left empty | n/a |

## P10 network-dx (docs/network, src/network.rs)

| Data | Source | Confidence |
| --- | --- | --- |
| JA4 format (JA4_a fields, GREASE ignored, sorted ciphers/extensions) | FoxIO `ja4` repo, `technical_details/JA4.md` | high |
| Chrome JA4 `t13d1517h2_8daaf6152771_cb7bf5808d99` (TCP) and `q13d0312h3_55b375c5d22e_178839b6cec1` (QUIC); 15 cipher suites | FoxIO `ja4` README examples; JA4_ro example in `JA4.md` lists the 15 suites | high |
| Firefox offers 17 cipher suites (e.g. `t13d1715h2_...` for Firefox 127) | Secondary JA4 write-ups (ip8.com, evomi.com) agreeing with NSS's default suite list | medium |
| Safari JA4 values | Secondary sources disagree; not used in rules | low |
| Chrome HTTP/2 SETTINGS `1:65536;2:0;4:6291456;6:262144`, WINDOW_UPDATE 15663105, order `m,a,s,p` | Chromium `net/http/http_network_session.cc` (6 MB stream / 15 MB session window, push off); scrapfly.io HTTP/2 guide (Chrome 144) | high |
| Firefox HTTP/2 `1:65536;2:0;4:131072;5:16384\|12517377\|0\|m,p,a,s` | Firefox `StaticPrefList.yaml` (`default-hpack-buffer` 65536, `push-allowance` 131072, `enabled.deps` false); secondary fingerprint lists | medium |
| Safari HTTP/2 `2:0;3:100;4:2097152;9:1\|10420225\|0\|m,s,a,p` | Single secondary source; pseudo-header order disputed | low |
| Chromium `--force-webrtc-ip-handling-policy` and its four values; pref `webrtc.ip_handling_policy` | Chromium `content_switches.cc`, `webrtc_ip_handling_policy.cc`, `chrome/common/pref_names.h` | high |
| Firefox `media.peerconnection.ice.{default_address_only,no_host,proxy_only_if_behind_proxy}`, `media.peerconnection.enabled` | Firefox `modules/libpref/init/all.js`, `StaticPrefList.yaml` | high |
| Chromium DoH prefs `dns_over_https.mode` / `.templates`; Firefox `network.trr.mode` / `.uri` | Chromium `pref_names.h`; Firefox `StaticPrefList.yaml`, `all.js` | high |
| Firefox `network.trr.mode` value meanings (0, 2, 3, 5) | Mozilla TRR documentation | medium |
| Chromium `--disable-quic`, `--enable-quic`, `--origin-to-force-quic-on`; Firefox `network.http.http3.enable` | Chromium `network_switch_list.h`; Firefox `StaticPrefList.yaml` | high |

## P3a special-uas (iOS app and browser user agents)

Samples were read on 2026-11-13 from whatmyuseragent.com (per-app and per-version pages) and checkproxy.org (user-agent lists); "observed" below means seen in several real strings there.

| Data | Source | Confidence |
| --- | --- | --- |
| Google app (GSA) iOS format: real OS version with three components (`26_5_0`, `27_0_0`), `GSA/<ver>.<build> Mobile/15E148 Safari/604.1` | Owner's sample plus ~60 observed strings (checkproxy.org GSA list, whatmyuseragent.com Google Search App) | high |
| GSA versions 446.3.1011462875, 445.5.1006576801, 444.2.1001595231, 443.5.997327796 | Observed on iOS 26.6–27.2 | medium |
| Chrome for iOS format: real OS version as `major_minor_bugfix`, `CriOS/<full> Mobile/15E148 Safari/604.1` | Chromium `ios/web/common/user_agent.mm` (`kUserAgentBugFixVersion` enabled by default in `ios/web/common/features.mm`) | high |
| Chrome for iOS 158.0.8127.35 and 157.0.8103.37 | Chrome Releases blog, "Chrome Stable for iOS Update" posts | high |
| Firefox for iOS format: hard-coded `CPU iPhone OS 18_7`, `FxiOS/<ver> Mobile/15E148 Safari/604.1` (iPad sends a plain desktop Safari UA, so it is not modelled as Firefox) | firefox-ios `BrowserKit/Sources/Shared/UserAgent.swift` at tags firefox-v158.0 and firefox-v160.1 | high |
| Firefox for iOS 160.1 / 160.0, minimum iOS 15.0 | firefox-ios `version.xcconfig` at release tags; GitHub releases; `Common.xcconfig` deployment target | high |
| Edge for iOS format: `EdgiOS/<full> Version/<iOS major>.0 Mobile/15E148 Safari/604.1`, three-component OS version | Observed (checkproxy.org Edge list, whatmyuseragent.com Microsoft Edge) | medium |
| Edge for iOS 157.0.4341.42, 155.0.4283.61 | Observed on iOS 26.7 / 27.0 | medium |
| Instagram iOS format (2026): real OS version, `Mobile/<iOS build>`, `iOS <ver with underscores>`, `IABMV/1`, trailing `Safari/604.1` | ~25 observed strings for Instagram 448–452 | high |
| Instagram 452.0.1.39.54 / 1093431980, 451.0.0.37.60 / 1088109557, 450.1.0.35.61 / 1084815185 | Observed; the build number is identical across devices for each version | medium |
| Facebook iOS format (2026): `Mobile/<iOS build> Safari/604.1 [FBAN/FBIOS;...;FBRV/<n>;IABMV/1];FBNV/1` | ~30 observed strings for FBAV 577–580 | high |
| Facebook 580.0.0.47.72 (FBBV 1074444153, FBRV 1079602028), 579.0.0.57.68 (1068430959, 1075661921) | Observed; FBRV varies per user, the most common value is used | medium |
| TikTok iOS format `Mobile/15E148 Safari/604.1 musical_ly_<ver> BytedanceWebview/d8a21c6`, versions 46.8.0 / 46.7.0 | Observed for 46.x | medium |
| X (Twitter) iOS format `Mobile/<iOS build> Twitter for iPhone/<ver>`, versions 11.98 / 11.97 | Observed | medium |
| LinkedIn iOS format `Mobile/15E148 [LinkedInApp]/<ver>` on the WebKit OS token, versions 9.32.4122 / 9.32.3977 | Observed | medium |
| LINE iOS format `Mobile/15E148 Safari Line/<ver>`, versions 26.16.1 / 26.15.1 | Few observed strings | low |
| iOS build numbers 17.7 → 21H16, 18.7 → 22H20, 26.5 → 23F77, 27.0 → 24A437 (`{osBuild}`) | Wikipedia "iOS 17", "iOS 18", "iOS 26", "iOS 27" version tables; 23F77 also seen in Facebook and X user agents on 26.5 | high (27.0: medium, three 27.0 builds exist; the general release is used) |
| iPhone 18 Pro `iPhone19,2`, iPhone 18 Pro Max `iPhone19,3` (US A3473; global models use `iPhone19,7`) | The Apple Wiki identifier tables and EveryMac (via search results; pages block automated fetches) | medium |
| Snapchat, Pinterest, WeChat, Telegram, Messenger iOS; all Android in-app user agents | Not added: see the P3a report (format needs data that could not be verified or a generator change) | n/a |

## P5 inject-graphics (data/extra/gpu-*.json, src/extras/graphics.rs)

| Data | Source | Confidence |
| --- | --- | --- |
| Chromium/Windows ANGLE D3D11 WebGL limits (`gpu-angle-d3d11.json` params) | Measured in Chromium 156 (Playwright build, headed) on Intel Xe-LPG (0x7D67) through ANGLE D3D11; cross-checked against ANGLE `src/libANGLE/renderer/d3d/d3d11/renderer11_utils.cpp` GenerateCaps (feature level 11 limits are GPU-independent) | high |
| NVIDIA override MAX_VERTEX_UNIFORM_VECTORS 4095 (and derived component counts) | ANGLE renderer11_utils.cpp: feature `skipVSConstantRegisterZero` is enabled for NVIDIA and subtracts one vector | high |
| ANGLE D3D11 shader precision (all highp) | Same Chromium 156 measurement | high |
| ANGLE D3D11 WebGL 1/2 extension lists | Same measurement on one Intel GPU; NVIDIA/AMD/Qualcomm lists on D3D11 not verified separately | medium |
| MAX_SAMPLES | Driver-dependent (format sample counts); deliberately not set | — |
| WebGPU availability by engine, OS, version and GPU (`gpu-webgpu.json`) | gpuweb wiki Implementation-Status (github.com/gpuweb/gpuweb/wiki/Implementation-Status): Chrome 113 Windows/macOS/ChromeOS, Windows ARM64 behind a flag, Android 121 (ARM/Qualcomm/Intel, Android 12+) and 139 (Imagination, Android 16+), Linux 144 (Intel Gen12+); Firefox 141 Windows, 145 macOS 26+, 147 all macOS; Safari 26 | high (Firefox macOS mapping: medium) |
| GPUAdapterInfo vendor/architecture names and PCI-id mapping | Dawn `src/dawn/gpu_info.json` (lower-case, spaces to hyphens); `intel`/`xe-lpg` confirmed in Chromium 156 | high |
| iPhone `color-gamut: p3` | Apple tech specs (iPhone 8, iPhone 11: "Wide color display (P3)"); iPhone 7 not checked | high (iPhone 7: medium) |
| Chromium `document.fonts.check()` returns true for any non-web-font family | Measured in Chromium 156 | high |

## P4 inject-navigator (data/extra/speech-voices.json, data/extra/keyboard-layouts.json, src/extras/navapi.rs)

| Data | Source | Confidence |
| --- | --- | --- |
| Windows SAPI voice names per language (`windowsLocal`) | Microsoft Support "Appendix A: Supported languages and voices" (support.microsoft.com/…/4486e345-7730-53da-fcfe-55cc64300f01); exact `Microsoft X - Language (Region)` strings cross-checked with readium/speech `json/*.json`; en-US list and alphabetical order match a real Chrome 106 Windows 10/11 dump (@unblocked-web/default-browser-emulator `browser-speech.json`); tr-TR (Tolga only) observed on a Turkish Windows 11 host in Chromium | medium (en-US, tr-TR: high). Assumes an install has only its display language's voices. en-AU left out: Microsoft says James, readium says Richard |
| Chrome's 19 `Google …` network voices, order, `localService:false`, default = first OS voice | Chromium `chrome/browser/resources/network_speech_synthesis/manifest.json` (byte-checked; two names contain U+00A0), `content/browser/speech/tts_controller_impl.cc` (OS voices first), `speech_synthesis_impl.cc` (`is_local_service = !remote`, `is_default = i == 0`) | high (final order after `FinalizeVoiceOrdering`: medium) |
| Edge, Brave, Firefox, macOS, iOS, Android voice lists | Not added: Edge exposes 250+ online voices, Firefox `voiceURI` format and mobile/macOS current dumps not verified | n/a |
| Keyboard layout maps (US, UK, German, French, Spanish, Latin American, Italian, Brazilian ABNT2, Turkish Q, Japanese 106) | kbdlayout.info KLC downloads (kbdus, kbduk, kbdgr, kbdfr, kbdsp, kbdla, kbdit, kbdbr, kbdtuq, kbd106); code set and dead-key handling from Chromium `ui/events/keycodes/dom/dom_keyboard_layout*.{h,cc}`; Turkish Q map and Chromium's iteration order observed exactly on a Windows host | high (Japanese kbd106 vs. stub: medium) |
| Locale → Windows default layout (en-US/en-CA/en-AU/pl-PL → US; Polish Programmers equals US unshifted) | kbdlayout.info, Windows defaults | medium; nl-NL and ko-KR left out (not verified) |
| HTML-standard PDF plugin list and mime types | html.spec.whatwg.org "system-state" (PDF viewer plugins); observed in headed Chromium | high |
| `pdfViewerEnabled` false on Chrome Android, true on Firefox Android | Chromium `dom_plugin_array.cc` (`IsPdfViewerAvailable`, Android flag off by default); Firefox `Navigator.cpp` / `pdfjs.disabled` default | medium; iOS/iPadOS Safari not verified (left as browser default) |
| `enumerateDevices` before permission (one entry per kind, empty ids/labels; Chromium includes audiooutput, Firefox/Safari only inputs) | Media Capture spec, WPT `MediaDevices-enumerateDevices.https.html`, observed in Chromium | high; desktop "no built-in camera or microphone" is an assumption (low) |
| `navigator.connection` presence, rtt rounded to 50 ms (cap 3000), downlink to 0.05 Mbps (cap 10), `type`/`downlinkMax` only Android/ChromeOS; Brave disables it | Chromium `network_state_notifier.cc`, MDN BCD `NetworkInformation.json`, Brave wiki "Deviations from Chromium" | high (chosen rtt 50–150 ms, downlink 1.5–10 Mbps: medium) |
| Battery API presence (Chromium only) and no-battery values (charging, 0, Infinity, 1) | MDN BCD `Navigator.json`, W3C Battery Status spec | high; discharging laptops report `Infinity` times (spec-allowed); Brave's fixed values not verified (left as browser default) |
| Storage quota: Chromium 80 % pool × 75 % per origin of total disk; Firefox min(10 %, 10 GiB); Safari ~60 % | Chromium `storage/browser/quota/quota_settings.cc`, MDN "Storage quotas and eviction criteria" | high (formulas); disk sizes per device class: low |
| `jsHeapSizeLimit` = min(RAM/2, 4 GiB) + 3 × semi-space (RAM/128, power of two in [2, 32] MiB), desktop Chromium | V8 `src/heap/heap.cc`; 4395630592 observed in headed Chromium on Windows | high for ≥ 8 GB, medium below; Android (quantized, RAM/4) left out |
| `AudioContext.baseLatency`: Chromium Windows 0.01 (WASAPI 10 ms period), Chromium macOS 256 frames / sampleRate, Firefox 0 | Chromium `audio_context.cc`, `audio_manager_mac.cc`; Firefox `AudioContext.h`; 0.01 observed on Windows | high (macOS: code only) |
| hid/usb/serial/bluetooth presence per browser and platform (Web Serial on Chrome Android 138+, Firefox 151+ desktop, no Web Bluetooth on Linux by default, Brave disables Bluetooth and Serial) | MDN BCD `Navigator.json`, Chrome 138 release notes, Brave wiki | high (Brave: medium) |
| `userActivation`, `screen.orientation`, `pdfViewerEnabled` in Safari 16.4+; `Notification` undefined in iOS Safari outside home-screen apps; `window.orientation` only on mobile | MDN BCD (`Navigator.json`, `ScreenOrientation.json`, `Notification.json`), caniuse `mdn-api_window_orientation`, Screen Orientation spec (natural orientation angle 0) | high |
| Maximized window: Chromium Windows `outer* = avail*`, `screenX/Y = 0`; Firefox Windows `-8` and `+16` | Real measurements quoted in github.com/ProxyShard/ShardBrowser/issues/140 and bugzilla.mozilla.org/show_bug.cgi?id=1811326 | medium (macOS menu-bar `screenY`: low; mobile not set) |

## P8 js-integrations (packages/veneer firefoxPrefs, chromiumArgs, docker/)

| Data | Source | Confidence |
| --- | --- | --- |
| Firefox prefs `general.useragent.override`, `general.appversion.override`, `general.platform.override`, `general.oscpu.override` | support.mozilla.org/questions/1417685 (list of override prefs); bugzilla.mozilla.org/show_bug.cgi?id=1711835 | high |
| Firefox pref `dom.maxHardwareConcurrency` (caps `navigator.hardwareConcurrency`, default 16) | support.mozilla.org/questions/1174651 | high |
| Firefox prefs `layout.css.devPixelsPerPx` (CSS px scale, string), `intl.locale.requested` | wiki.archlinux.org/title/Firefox/Tweaks; bugzilla.mozilla.org/show_bug.cgi?id=1726964 | medium |
| Firefox prefs `intl.accept_languages`, `privacy.resistFingerprinting` | Firefox about:config (well-known prefs); RFP overrides UA/timezone | medium |
| Chromium switches `--lang`, `--user-agent`, `--window-size`, `--force-device-scale-factor` | Chromium command-line switches; `--user-agent`/`--lang` verified in a real Chromium 156 run (ChromeDriver + Puppeteer tests) | high |
| Docker base images `mcr.microsoft.com/playwright:v1.64.1-noble`, `rust:1.94-slim-bookworm` | mcr.microsoft.com/v2/playwright/tags/list; hub.docker.com/v2/repositories/library/rust/tags/1.94-slim-bookworm (checked 2026-11-13) | high |
| CDP methods in `cdpCommands` (`Emulation.setUserAgentOverride`, `setTimezoneOverride`, `setLocaleOverride`, `setDeviceMetricsOverride`, `setTouchEmulationEnabled`, `Page.enable`, `Page.addScriptToEvaluateOnNewDocument`) | Chrome DevTools Protocol; exercised against Chromium 156 (raw CDP, Puppeteer, Crawlee, ChromeDriver, WebdriverIO, MV3 `chrome.debugger`) | high |
| Left out: Firefox font allow-list pref | could not verify the pref name against a primary source | — |

## P3b dataset-scale (locales.json, os/*.json versions, desktop screens, data/extra/fonts-locale.json)

| Data | Source | Confidence |
| --- | --- | --- |
| `navigator.languages` per locale for Chromium UI languages (de, fr, ja, ru, zh-CN, zh-TW, id, th, fil, es-419, pt-PT, sv, da, nb, fi, cs, sk, hu, ro, bg, el, hr, sl, sr, et, lv, lt, he, ar, fa, sw, ca, ...) | Chromium `IDS_ACCEPT_LANGUAGES` in `components/strings/components_locale_settings_*.xtb` (chromium.googlesource.com, main) | high (Chromium); medium for Gecko/WebKit, which use the same list here |
| Country variants (`en-IE`, `es-CO`, `de-AT`, `fr-CA`, `zh-HK`, ...) as `[region tag, base language (+ en-US, en when the base Chromium list has them)]` | Same shape as the original en-CA / en-AU / es-MX entries; matches Safari/iOS region tags | medium |
| Timezone ids per country (US, CA, AU, MX, BR, RU, ID, CN, ES, PT, es-419 countries, ...) | IANA tz database zones; every id checked to be returned unchanged by Chromium 156 `Intl.DateTimeFormat().resolvedOptions().timeZone` (Playwright `timezoneId`) | high |
| Left out: India (`Asia/Kolkata`), Vietnam (`Asia/Ho_Chi_Minh`), Ukraine (`Europe/Kyiv`), Argentina (`America/Argentina/Buenos_Aires`), Nepal, Myanmar | Chromium canonicalizes these to legacy ids (`Asia/Calcutta`, `Asia/Saigon`, `Europe/Kiev`, `America/Buenos_Aires`, ...) while Firefox/Safari report the IANA id; one id per profile cannot be right for both | — |
| Locale and timezone weights | Rough internet-user population, scaled to the existing weights | low |
| iOS 26.0 / 26.2 / 26.3 / 26.4 / 26.5 / 26.5.2 / 26.6 / 26.6.1 / 26.6.2 / 27.0 / 27.2 releases and build numbers | en.wikipedia.org/wiki/IOS_26, en.wikipedia.org/wiki/IOS_27, support.apple.com/en-us/149076 | high |
| Safari OS token on iOS 26+: `18_6` on 26.0, `18_7` from 26.2 on (and on 27.x) | WebKit bug 298717; ua-parser-js issues 818/828 (iOS 26.2.1 → `18_7` + `Version/26.2`); useragents.io iOS listing; testmuai iOS 27 note | high (26.0, 26.2+), medium (27.x) |
| Safari `Version/` on iOS patch releases (`26.5.2`, `26.6.1`, `26.6.2` carry three components) | Real UAs listed on useragents.io (`18_7` + `Version/26.6.2`, ...) | medium |
| Left out: iOS 26.0.1, 26.1 (`18_6_2`?), 26.2.1, 26.3.1, 26.4.x, 26.5.1 (iPhone 17 only), 26.7.x, 27.0.1, 27.2.1 | Conflicting or missing evidence for the Safari token/version pairing | — |
| iPadOS 26.2 / 26.4 / 26.6 / 27.2 with Safari pinned to the minor version | Same release history; iPad desktop-mode UA `Version/26.5` observed (ua-parser-js #828) | medium |
| macOS Golden Gate 27.0–27.2.1, Tahoe 26.0–26.7.3, Sequoia 15.7–15.8.3, Sonoma 14.8–14.8.9 (UA-CH `platformVersion` = full OS version) | en.wikipedia.org/wiki/MacOS_Golden_Gate, MacOS_Tahoe, MacOS_Sequoia, MacOS_Sonoma release tables (15.7.6 and 14.8.6 are absent there and left out) | high |
| Safari pinned per macOS version (OS minor on Tahoe/27; by release date on Sequoia/Sonoma; Safari 27.0 on Tahoe 26.7.x and Sequoia 15.8.x) | Safari 26.4 security notes (Sonoma + Sequoia), Safari 27 security notes support.apple.com/en-us/149039 (Sequoia + Tahoe) | medium |
| macOS version weights | Release recency | low |
| Which macOS versions each Mac runs (no Golden Gate on Intel, M4 Macs from Sequoia) | Wikipedia supported-hardware lists; launch OS versions | high |
| ChromeOS 16836.32.0 (Chrome 156.0.8078.93, stable) and 16700.70.0 (Chrome 150.0.7871.289, LTC), Chrome pinned per platform | chromiumdash.appspot.com/cros/fetch_serving_builds (servingStable / servingLtc) | high |
| Linux UA-CH `platformVersion` is the empty string | Chromium commit "Return the empty string for UA-Platform-Version hint on Linux"; MDN Sec-CH-UA-Platform-Version | high |
| Linux distribution font sets: Ubuntu 24.04 / 26.04 (desktop manifests + package file lists), Fedora 44 Workstation (comps `fonts` group → default-fonts-core-* → Noto; gnome-desktop → Source Code Pro), Debian 13 GNOME (fonts-dejavu-core via fontconfig-config, fonts-cantarell via gnome-core, fonts-symbola via task-desktop) | releases.ubuntu.com/*/ubuntu-*-desktop-amd64.manifest, packages.ubuntu.com filelists, forge.fedoraproject.org/releng/fedora-comps comps-f44.xml.in, packages.fedoraproject.org, packages.debian.org/trixie | high (Ubuntu), medium (Fedora, Debian); Arch has no default font set and is left out |
| Desktop CSS screen sizes and shares (1920x1080, 1536x864, 1366x768, 1280x720, 2560x1440) | StatCounter desktop screen resolution, worldwide, October 2026 | high (sizes), medium (weights) |
| Other Windows sizes as physical panel / Windows scale pairs with integer CSS results (2048x1152 = 1440p@125%, 2560x1440@1.5 = 4K@150%, 1920x1080@2 = 4K@200%, 1536x960 / 1280x800 = 1920x1200@125/150%, 1440x900@2 = 2880x1800@200%, 1536x1024@1.5 = 2304x1536@150%) | Windows display scaling steps; weights estimated | medium (weights low) |
| Mac "looks like" sizes: MBA 13.3" 1024x640/1280x800/1440x900/1680x1050; MBA 13.6" 1280x832/1470x956/1710x1112; MBP 14" 1024x665/1147x745/1352x878/1512x982/1800x1169; MBP 16" Intel 1344x840/1536x960/1792x1120 | MacRumors forum threads and Apple Community posts listing the Displays settings options | medium |
| Windows locale fonts (Arabic, Hebrew, Japanese, Korean, Chinese Simplified/Traditional, Thai supplemental packages) | learn.microsoft.com/en-us/typography/fonts/windows_11_font_list ("Fonts included in Feature On Demand packages") | high (families), medium (auto-install per locale) |

## P1 android-oem (data/devices/android-{xiaomi,oppo,oneplus,vivo,motorola,samsung-more,google-more}.json)

| Data | Source | Confidence |
| --- | --- | --- |
| Model codes (`Build.MODEL`, UA-CH `model`) for Xiaomi / Redmi / POCO, OPPO / OnePlus (`CPHxxxx`), vivo (`V2xxx`), Motorola (`moto g84 5G`, `motorola edge 40`, ...), Samsung (`SM-…B`), Pixel | opengles.gpuinfo.org device names, miuirom.org, GSMArena "Models", Motorola `PRODUCT_MODEL` in public device trees, archived stock firmware `build.prop` dumps (github.com/xdaGari/tadiphone-buildprop-archive and per-device `*_dump` repos) | high (most common regional code chosen where several exist: medium) |
| WebGL renderer strings (`Adreno (TM) 7xx`, `Mali-G57 MC2`, `Mali-G615 MC6`, `Mali-G720-Immortalis MC12`, `PowerVR B-Series BXM-8-256`, ...) | GL_RENDERER in opengles.gpuinfo.org reports for the exact model code; where no report exists, a sibling phone with the same SoC (Xiaomi 14T ← POCO X6 Pro, POCO M6 Pro ← Redmi Note 13 Pro 4G, Pixel 7a ← Pixel 7) | high (sibling-derived: medium) |
| Galaxy A55 / S24 FE Xclipse renderer `ANGLE ((Samsung Xclipse 530/940) on Vulkan 1.3.304)` and vendor `Samsung Electronics Co., Ltd.` | gpuinfo reports on Android 17 (the string embeds the Vulkan driver version and changes per firmware, so these two devices are limited to Android 17) | medium |
| CSS screen sizes and DPR | Stock default density (`ro.sf.lcd_density`) from firmware dumps / device trees, CSS = ceil(physical / (density / 160)) (Chrome rounds up: Pixel 7 1080 / 2.625 → 412); Samsung FHD+ default 450 dpi measured in Chrome on the Galaxy A54 (bugzilla.mozilla.org/show_bug.cgi?id=1887163); OnePlus 13 / 12R densities from stock dumps (Klozz/oneplus_op5d55l1_dump, Jiovanni-dump/oneplus_op5d35l1_dump) with the 1080-wide default surface seen in gpuinfo; Pixel sizes from Chrome DevTools device presets (devtools-frontend `EmulatedDevices.ts`) and yesviz.com; Pixel 8 Pro / 9 Pro XL default "High" resolution 1008x2244 (gpuinfo surface size, reviews) | medium (computed, not measured in a browser on each device) |
| Android versions per device (12–17) | miuirom.org update history, GSMArena, gpuinfo report OS versions, Samsung One UI 9 rollout coverage (sammobile, beebom), Android 17 Pixel launch 2026-06-16 (9to5google) | medium |
| RAM variants | mi.com / GSMArena spec pages; shares between variants are estimates | high (sizes), low (shares) |
| Device weights | Rough popularity (high 40 / medium 15 / low 5) on the existing scale | low |
| Left out | Redmi Note 14 4G/5G (stock density unknown), vivo X100 (conflicting model code), moto g stylus 2024 / edge 50 Pro / razr 40 / Galaxy Z Flip5/6 (no GL_RENDERER report), Pixel 9 Pro / 10 Pro (default resolution unclear), Galaxy A14 / A05s / M-series / S23 FE / Note 20 Ultra (screen or renderer unverified), OnePlus 12 (default density inferred only), OnePlus 11 / Nord 3 / Nord 4 / 15, OPPO Reno10/11/12/12 F, A38, A60, A78 5G, A79, Find X8, vivo Y17s / Y36 / V30, moto g24 (no stock default density found), every Huawei device (Android 10/11 or HarmonyOS, not in the dataset's OS versions) | — |
| Opera, Vivaldi, Yandex, UC, Huawei Browser | Not added to the embedded dataset; see the P1 report (desktop device files must list Opera; Vivaldi / Yandex client hints and UC / Huawei Browser UAs need generator support) | — |

## Updating

Browser versions go stale every few weeks. When a new stable version ships, add it to `browsers.json`, move most of the weight to it, and drop the oldest entry. Run `cargo test` before opening a pull request — the test suite generates thousands of profiles and fails on any inconsistency.

## P6 validator

| Data | Source | Confidence |
| --- | --- | --- |
| `extra/browser-releases.json` Chromium majors 100-158 first stable date (Chrome, Edge, Brave, Chrome for iOS report the Chromium major) | Chromium Dash schedule API, `https://chromiumdash.appspot.com/fetch_milestone_schedule?mstone=N` field `stable_date` (158 is the scheduled 2026-11-17 date) | high |
| `extra/browser-releases.json` Firefox majors 100-160 release date | `https://product-details.mozilla.org/1.0/firefox_history_major_releases.json` | high |
| `extra/browser-releases.json` Safari 15.0 2021-09-20, 16.0 2022-09-12, 17.0 2023-09-26, 18.0 2024-09-16 | Wikipedia "Safari version history" (macOS tables) | medium |
| Safari 26.0 2025-09-15 | Apple Safari 26 release notes (developer.apple.com/documentation/safari-release-notes) | high |
| Safari 27.0 2026-09-14 | iOS 27 release coverage (MacRumors forum changelog, 91mobiles, Free Press Journal) | medium |
| `navigator.deviceMemory` buckets from Chromium 147: 2-8 on Android, 2-32 elsewhere | chromestatus feature 6330376953921536 "Update Device Memory API limits" (shipped desktop/Android 147) and blink-dev Intent to Ship; observed `32` in Playwright Chromium 156 on Windows | high |
