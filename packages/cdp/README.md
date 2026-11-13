# @llcforge/veneer-cdp

Apply a [Veneer](../veneer) profile to a Chromium target over a raw Chrome DevTools Protocol connection.
No Playwright, no Puppeteer: works with any Chromium-based browser started with `--remote-debugging-port`,
with remote browsers, and with any object that can send CDP commands.

```js
const veneer = require('@llcforge/veneer');
const { applyToTarget } = require('@llcforge/veneer-cdp');

const profile = veneer.generate({ os: 'windows', browser: 'chrome', locale: 'tr-TR' });
// page target from http://127.0.0.1:9222/json/list
const client = await applyToTarget(page.webSocketDebuggerUrl, profile);
await client.send('Page.navigate', { url: 'https://example.com' });
```

On a browser-level endpoint (`/json/version` → `webSocketDebuggerUrl`) use flat sessions:

```js
const { connect, attachAndApply } = require('@llcforge/veneer-cdp');
const client = await connect(browserWsUrl);
const { targetId } = await client.send('Target.createTarget', { url: 'about:blank' });
const sessionId = await attachAndApply(client, targetId, profile);
await client.send('Page.navigate', { url: 'https://example.com' }, sessionId);
```

What is applied, in order (`veneer.cdpCommands(profile)`):

| Command | Sets |
| --- | --- |
| `Emulation.setUserAgentOverride` | user agent, `Accept-Language`, `navigator.platform`, client hints (`Sec-CH-UA*`) |
| `Emulation.setTimezoneOverride` | IANA timezone |
| `Emulation.setLocaleOverride` | ICU locale (`Intl`, date formatting) |
| `Emulation.setDeviceMetricsOverride` | viewport, device pixel ratio, screen size, mobile |
| `Emulation.setTouchEmulationEnabled` | touch and `maxTouchPoints` |
| `Page.addScriptToEvaluateOnNewDocument` | the Veneer init script (WebGL, canvas/audio noise, hardware, fonts …) |

Overrides last as long as the CDP session stays open, so keep the client open while the page is in use.
Only Chromium profiles (`engine: 'chromium'`) are accepted; a Chromium browser cannot present a Firefox or Safari
profile coherently. `navigator.webdriver` and other automation markers are not touched.

`applyToSession(send, profile)` takes any `send(method, params)` function, so the same commands work with
Puppeteer (`session.send`), Electron (`webContents.debugger.sendCommand`) and `chrome.debugger`.

Tests: `node --test`; set `VENEER_E2E=1` and `VENEER_API` to run the real-Chromium test.
