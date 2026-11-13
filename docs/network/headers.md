# Request headers

`navigation_headers(profile)` returns the headers a real browser with the profile sends for a
top-level navigation, in that browser's order. Chromium pages opened through `launchWithProfile`,
`newPage` or `applyProfile` get `User-Agent`, `Accept-Language` and the `Sec-CH-UA*` client hints from
the profile through CDP `Emulation.setUserAgentOverride`.

## Verifying what was actually sent

Capture the navigation request and compare it with the profile:

```js
const { browser, page } = await veneer.launchWithProfile(playwright, profile);
const response = await page.goto('https://example.com/');
const sent = await response.request().allHeaders(); // an object: values compared, order skipped
const report = veneer.compareHeaders(profile, sent);
```

```bash
veneer check-headers --profile profile.json captured.json   # exit code 1 on a mismatch
curl -s -X POST http://127.0.0.1:8787/v1/headers/compare \
  -H 'content-type: application/json' -d @body.json         # {"profile": {...}, "headers": [...]}
```

Accepted header forms: `[{ "name": ..., "value": ... }]` (Playwright `headersArray()`),
`[[name, value]]`, or `{ "name": "value" }` (Playwright `headers()`; the order check is skipped
because JSON objects do not keep order). At most 256 headers, names up to 256 bytes, values up to 8 KB.

| Rule | Severity | Meaning |
| --- | --- | --- |
| `headers.user-agent` | error | missing or different from `navigator.userAgent` |
| `headers.accept-language` | error | missing or different from the engine's q-value scheme for `navigator.languages` |
| `headers.sec-ch-ua`, `headers.sec-ch-ua-mobile`, `headers.sec-ch-ua-platform` | error | missing or different on a client-hints browser, or **sent at all** for a Firefox/Safari profile (a Chromium engine is showing through) |
| `headers.order` | warning | the expected headers that were sent appear in another relative order |

Only identity headers are compared by value: `sec-fetch-*`, `accept` and `priority` depend on how the
navigation started (typed URL, link click, `page.goto`) and on the protocol.

Playwright's `headersArray()` returns headers sorted by name, not in wire order, so the order check
is only meaningful for a capture taken on the server side (for example Node's `req.rawHeaders`) or
from a HAR/proxy that keeps order.

## What real Chromium sends (Playwright 1.64, bundled Chromium 156, HTTP/1.1)

Wire order of an unmodified navigation: `Host`, `Connection`, `sec-ch-ua`, `sec-ch-ua-mobile`,
`sec-ch-ua-platform`, `Upgrade-Insecure-Requests`, `User-Agent`, `Accept`, `Sec-Fetch-Site`,
`Sec-Fetch-Mode`, `Sec-Fetch-User`, `Sec-Fetch-Dest`, `Accept-Encoding`, then `Accept-Language`
(added last by the network stack). `priority` is not sent over HTTP/1.1. This matches
`navigation_headers`.

Two findings from capturing profiles applied with `launchWithProfile`:

- When `Accept-Language` is set through CDP (`Emulation.setUserAgentOverride` `acceptLanguage`, or
  Playwright's `locale` option), Chromium inserts it right after `User-Agent` instead of last.
  `compare_headers` reports this as `headers.order`. The `--accept-lang` launch switch keeps the
  natural position but Playwright's `locale` option overrides it.
- Chromium adds q-values to the CDP `acceptLanguage` list itself, so it must receive the plain list
  (`de-DE,de,en-US,en`); a list that already has q-values is sent as `de;q=0.9;q=0.9`.

## Headers for non-browser clients

`veneer headers` prints the profile's navigation headers in browser order for HTTP libraries:

```bash
veneer headers --os windows --browser chrome --format curl --url https://example.com/
veneer headers --profile profile.json --format python   # also: json, node
```

This aligns header *values and order* only. HTTP libraries still send their own TLS ClientHello and
HTTP/2 SETTINGS (see [tls.md](tls.md) and [http2.md](http2.md)), which identify them regardless of the
headers. curl also sends its own `Host` header first, and Python `requests` adds `Connection`.
