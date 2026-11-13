# HTTP/3 (QUIC)

**Veneer cannot change QUIC.** HTTP/3 runs over UDP with TLS 1.3 inside QUIC. Its ClientHello has its
own JA4 (`q...`, for example Chrome `q13d0312h3_55b375c5d22e_178839b6cec1`, FoxIO), and the QUIC
transport parameters are a further fingerprint surface. Public per-browser data on transport
parameters is thin; one source gives Chrome's connection flow-control limit as 15 MB, the same as its
HTTP/2 session window.

- Browsers start on TCP (HTTP/2) and move to HTTP/3 after an `Alt-Svc` header or an HTTPS DNS record
  advertises it, so the first request to a host is normally HTTP/2.
- Chromium switches (`components/network_session_configurator/common/network_switch_list.h`):
  `--disable-quic`, `--enable-quic`, `--origin-to-force-quic-on=host:443`.
  Firefox pref: `network.http.http3.enable` (default `true`, `StaticPrefList.yaml`).
- HTTP CONNECT proxies and most SOCKS5 setups carry TCP only. Through such a proxy the browser stays on
  HTTP/2, which is normal. If the proxy does not carry UDP but your network allows direct UDP, QUIC
  could bypass the proxy: disable it (`--disable-quic`, `network.http.http3.enable = false`).
- Safari's HTTP/3 behaviour depends on the OS version; Veneer has no verified data for it.
