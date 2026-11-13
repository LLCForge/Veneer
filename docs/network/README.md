# Network layer

Veneer controls what a page can read (JavaScript APIs) and the request headers a Chromium page sends
through CDP. Everything below the HTTP headers is produced by the browser's own network stack, and
Veneer **cannot change it**: the TLS ClientHello, HTTP/2 SETTINGS and priorities, QUIC transport
parameters, DNS resolution and the egress IP. A profile is only convincing when the engine you launch
matches the profile's engine and the proxy matches its location.

| Topic | What Veneer offers | Document |
| --- | --- | --- |
| Request headers | `navigation_headers`, `compare_headers`, `veneer headers`, `veneer check-headers`, `POST /v1/headers/compare`, JS `compareHeaders` | [headers.md](headers.md) |
| TLS (JA3/JA4) | Documentation and the informational `check_ja4` / `veneer net ja4` / `POST /v1/network/ja4` / JS `checkJa4` | [tls.md](tls.md) |
| HTTP/2 | Documentation | [http2.md](http2.md) |
| HTTP/3 (QUIC) | Documentation | [http3.md](http3.md) |
| DNS over HTTPS | Documentation | [dns-over-https.md](dns-over-https.md) |
| WebRTC IP exposure | `webrtc_settings` / `veneer net webrtc` / `GET /v1/network/webrtc` / JS `webrtcSettings` | [webrtc.md](webrtc.md) |
| Proxy location | `check_location` / `veneer net location` / `POST /v1/network/location` / JS `checkLocation` | [proxy-location.md](proxy-location.md) |

Sources and confidence levels are listed in each document and in `crates/veneer/data/SOURCES.md`.
Values move with browser releases; re-check them against a current browser before relying on an
exact string.
