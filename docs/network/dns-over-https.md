# DNS over HTTPS

DNS is resolved by the browser or the OS, outside Veneer. Through an HTTP CONNECT or `socks5h` proxy,
the proxy resolves host names and the browser's DNS settings do not matter. With plain SOCKS5 (local
resolution) or without a proxy, DNS queries leave from your own network and can disagree with the
proxy location.

| Browser | Setting | Values | Source |
| --- | --- | --- | --- |
| Chrome / Chromium | prefs `dns_over_https.mode`, `dns_over_https.templates`; enterprise policies `DnsOverHttpsMode`, `DnsOverHttpsTemplates` | `off`, `automatic`, `secure`; templates are DoH URI templates | Chromium `chrome/common/pref_names.h`, Chrome Enterprise policy list |
| Firefox | `network.trr.mode`, `network.trr.uri`, `network.trr.custom_uri` | `0` default (off), `2` DoH first with fallback, `3` DoH only, `5` off by choice | Firefox `StaticPrefList.yaml` and `all.js`, Mozilla TRR documentation |
| Safari, iOS browsers | no browser setting; system-wide encrypted DNS (configuration profiles, Network Extension) | - | Apple platform documentation |

Playwright Firefox example:

```js
await playwright.firefox.launch({
  firefoxUserPrefs: { 'network.trr.mode': 3, 'network.trr.uri': 'https://dns.example/dns-query' },
});
```

Few real users run DoH-only mode; a proxy that resolves names (HTTP CONNECT or `socks5h`) is the
less conspicuous fix.
