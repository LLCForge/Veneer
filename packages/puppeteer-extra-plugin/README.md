# @llcforge/puppeteer-extra-plugin-veneer

[puppeteer-extra](https://github.com/berstend/puppeteer-extra) plugin that applies one coherent
[Veneer](../veneer) profile to every page of a browser.

```js
const puppeteer = require('puppeteer-extra');
const veneerPlugin = require('@llcforge/puppeteer-extra-plugin-veneer');

const plugin = veneerPlugin({ constraints: { os: 'windows', locale: 'tr-TR' } });
puppeteer.use(plugin);

const browser = await puppeteer.launch();
const page = await browser.newPage(); // profile already applied
await page.goto('https://example.com');
console.log(plugin.profile.navigator.userAgent);
```

Options:

| Option | Default | Meaning |
| --- | --- | --- |
| `profile` | — | Use this Chromium profile as is. |
| `constraints` | `{}` | Otherwise generate one (`engine` is forced to `chromium`). |
| `launchArgs` | `true` | Add `veneer.chromiumArgs(profile)` on launch (`--lang`, `--user-agent`, `--window-size`, `--force-device-scale-factor`); switches you pass yourself win. |

Each page gets `veneer.cdpCommands(profile)` over its own CDP session: user agent and client hints, `Accept-Language`,
timezone, locale, device metrics, touch and the init script. `browser.newPage()` resolves only after the profile is
applied; pages opened another way (popups, other contexts) are handled from the target-created event.

One plugin instance is one fingerprint. For one fingerprint per browser, create a plugin per launch with
`addExtra(require('puppeteer'))`. The plugin does not hide `navigator.webdriver` or other automation markers.

Tests: `npm test`; with `VENEER_E2E=1`, `VENEER_API`, `puppeteer-core` and `playwright` (for the cached Chromium path)
resolvable, the real-browser test runs too.
