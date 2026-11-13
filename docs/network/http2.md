# HTTP/2 SETTINGS, WINDOW_UPDATE and priorities

**Veneer cannot change these frames**; the browser's network stack sends them when a connection
starts. They are commonly summarized in the Akamai format
`SETTINGS|WINDOW_UPDATE|PRIORITY|pseudo-header order` (Shuster, "Passive Fingerprinting of HTTP/2
Clients", Black Hat EU 2017). Setting ids: 1 `HEADER_TABLE_SIZE`, 2 `ENABLE_PUSH`,
3 `MAX_CONCURRENT_STREAMS`, 4 `INITIAL_WINDOW_SIZE`, 5 `MAX_FRAME_SIZE`, 6 `MAX_HEADER_LIST_SIZE`,
9 `NO_RFC7540_PRIORITIES`. Pseudo-headers: `m` `:method`, `a` `:authority`, `s` `:scheme`, `p` `:path`.

| Browser | Akamai fingerprint | Notes | Confidence |
| --- | --- | --- | --- |
| Chrome / Chromium | `1:65536;2:0;4:6291456;6:262144\|15663105\|0\|m,a,s,p` | Chromium `net/http/http_network_session.cc`: stream window 6 MB (`kSpdyStreamMaxRecvWindowSize`), session window 15 MB (`kSpdySessionMaxRecvWindowSize`; 15 728 640 - 65 535 = 15 663 105), push disabled. No PRIORITY frames; priority travels in the `priority` header and HEADERS frame. Matches the string published for Chrome 144. | high |
| Firefox | `1:65536;2:0;4:131072;5:16384\|12517377\|0\|m,p,a,s` | Firefox prefs `network.http.http2.default-hpack-buffer` = 65536 and `network.http.http2.push-allowance` = 131072 match settings 1 and 4; `network.http.http2.enabled.deps` defaults to `false`, so the old PRIORITY-frame tree is no longer sent. One secondary source gives another WINDOW_UPDATE value. | medium |
| Safari | `2:0;3:100;4:2097152;9:1\|10420225\|0\|m,s,a,p` | single secondary source; another gives the pseudo-header order `m,s,p,a` | low |

The pseudo-header order is the most stable part and differs per engine family, which is one more
reason to launch the engine that matches the profile. HTTP libraries (Python, Node, Go) have their own
SETTINGS and order; `veneer headers` cannot change that.
