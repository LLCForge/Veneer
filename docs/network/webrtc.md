# WebRTC IP exposure

A page can create an `RTCPeerConnection` and read its ICE candidates. Depending on the policy these
reveal local interface addresses (modern browsers hide host candidates behind mDNS `.local` names by
default) and, through STUN, the public address of the path WebRTC takes, which may bypass the proxy.

`webrtc_settings(engine, policy)` returns launch settings: CLI `veneer net webrtc --engine chromium
--policy public-interface-only`, API `GET /v1/network/webrtc?engine=chromium&policy=public-interface-only`,
JS `webrtcSettings('chromium')`.

| Policy | Chromium | Firefox |
| --- | --- | --- |
| `public-interface-only` | `--force-webrtc-ip-handling-policy=default_public_interface_only` | `media.peerconnection.ice.default_address_only = true`, `media.peerconnection.ice.no_host = true` |
| `disable-non-proxied-udp` | `--force-webrtc-ip-handling-policy=disable_non_proxied_udp` | the above plus `media.peerconnection.ice.proxy_only_if_behind_proxy = true` |
| `disabled` | no switch removes WebRTC; `disable_non_proxied_udp` is used | `media.peerconnection.enabled = false` |

WebKit exposes no such setting to Playwright. The Chromium profile preference with the same effect is
`webrtc.ip_handling_policy` (returned in `prefs`, for a persistent user-data directory).

```js
const engine = profile.browser.engine; // 'chromium' | 'gecko' | 'webkit'
const { playwrightLaunchOptions } = veneer.webrtcSettings(engine);
const { browser, page } = await veneer.launchWithProfile(playwright, profile, playwrightLaunchOptions);
```

`disable_non_proxied_udp` needs a proxy that relays UDP; with a plain HTTP or SOCKS5 proxy WebRTC
falls back to TCP through the proxy or fails. Turning WebRTC off is itself observable
(`RTCPeerConnection` is missing) and rare among real users, so prefer `public-interface-only`.

Sources: Chromium `content/public/common/content_switches.cc` (`force-webrtc-ip-handling-policy`),
`third_party/blink/common/peerconnection/webrtc_ip_handling_policy.cc` (the four policy values),
`chrome/common/pref_names.h` (`webrtc.ip_handling_policy`); Firefox `modules/libpref/init/all.js`
(`media.peerconnection.ice.*`) and `StaticPrefList.yaml` (`media.peerconnection.enabled`).
