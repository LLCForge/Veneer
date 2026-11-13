# TLS ClientHello: JA3 and JA4

**Veneer cannot change TLS.** The ClientHello is built by the browser's TLS library: BoringSSL in
Chromium (Chrome, Edge, Brave, Samsung Internet, Android WebView), NSS in Firefox, and Apple's stack in
Safari and in every browser on iOS/iPadOS (they must use WebKit). Launch the Playwright engine that
matches `profile.browser.engine` (`playwrightEngine(profile)` picks it) and the TLS fingerprint is the
real one of that engine family. A Firefox profile loaded through Chromium shows a Chromium ClientHello,
whatever the headers say.

## JA3

JA3 is an MD5 of the ClientHello version, cipher suites, extensions, elliptic curves and point
formats, in the order sent. Chrome randomizes the order of its ClientHello extensions (since
Chrome 110), so its JA3 changes from connection to connection and is not a stable identifier for
Chromium browsers.

## JA4

JA4 (FoxIO) sorts ciphers and extensions, so it is stable across Chrome's extension permutation, and it
ignores GREASE values. Format `JA4_a_JA4_b_JA4_c`:

| Part | Content |
| --- | --- |
| JA4_a | protocol (`t` TCP, `q` QUIC, `d` DTLS), TLS version (`13`), SNI (`d` domain, `i` none), cipher count (2 digits), extension count (2 digits), first and last character of the first ALPN value (`h2`, `h3`, `00` for none) |
| JA4_b | first 12 hex characters of the SHA-256 of the sorted cipher list |
| JA4_c | first 12 hex characters of the SHA-256 of the sorted extensions and signature algorithms |

Published fingerprints:

| Client | JA4 | Source | Confidence |
| --- | --- | --- | --- |
| Chrome (TCP) | `t13d1517h2_8daaf6152771_cb7bf5808d99` | FoxIO JA4 README, labelled example | high |
| Chrome (QUIC) | `q13d0312h3_55b375c5d22e_178839b6cec1` | FoxIO JA4 README, labelled example | high |
| Chrome-style ClientHello with 16 extensions | `t13d1516h2_8daaf6152771_02713d6af862` | FoxIO JA4 technical details (its raw form lists the same 15 ciphers) | high |
| Firefox (NSS) | 17 cipher suites, e.g. `t13d1715h2_5b57614c22b0_...` reported for Firefox 127 | secondary sources; 17 matches NSS's default suite list | medium (count), low (hashes) |
| Safari | sources disagree (`t13d1314h2`, `t13d1214h2`, `t13d1516h2` reported for different 2024-2025 builds) | secondary sources | low |

Extension counts change between releases (Chrome went from 16 to 17), so Veneer does not compare them.

## Informational check

`check_ja4(profile, ja4)` (CLI `veneer net ja4 --profile p.json <JA4>`, API `POST /v1/network/ja4`
with `{"profile": ..., "ja4": "..."}`, JS `checkJa4`) compares a JA4 you captured (for example from a
TLS echo service you trust) with the profile's engine. Every finding is a **warning**; the profile
stays valid.

| Rule | Fires when |
| --- | --- |
| `tls.ja4.protocol` | a DTLS (WebRTC) fingerprint was supplied |
| `tls.ja4.version` | the highest offered version is not TLS 1.3 |
| `tls.ja4.sni` | no SNI (expected only for IP-literal URLs) |
| `tls.ja4.alpn` | first ALPN is not `h2` (TCP) or `h3` (QUIC); `00` means no ALPN, typical of HTTP libraries |
| `tls.ja4.ciphers` | a Chromium profile without 15 suites, or a Firefox profile without 17 |
| `tls.ja4.cipher-hash` | a Chromium profile without Chromium's cipher hash, or a Firefox profile *with* it |

For WebKit profiles only the format, version, SNI and ALPN are checked; for consoles, TVs and VR
headsets only the format. The result's `notes` say what was skipped.
