# @llcforge/wdio-veneer-service

[WebdriverIO](https://webdriver.io) service for [Veneer](../veneer): every worker session gets one coherent profile.

```js
// wdio.conf.js
const VeneerService = require('@llcforge/wdio-veneer-service');

exports.config = {
  capabilities: [{ browserName: 'chrome' }],
  services: [[VeneerService, { constraints: { os: 'windows', locale: 'tr-TR' } }]],
};
```

In a spec, the profile is on the service instance; log `profile.seed` from a hook if you need to reproduce it.

| Hook | Chrome / Chromium / Edge | Firefox |
| --- | --- | --- |
| `beforeSession` | `veneer.chromiumArgs` appended to `goog:chromeOptions.args` / `ms:edgeOptions.args` | `veneer.firefoxPrefs` merged into `moz:firefoxOptions.prefs` |
| `before` | `veneer.cdpCommands` on every open page via `browser.getPuppeteer()` (user agent + client hints, `Accept-Language`, timezone, locale, metrics, touch, init script) | init script as a BiDi preload script (`browser.scriptAddPreloadScript`); needs a BiDi session |

Options: `constraints` (engine follows `browserName`) or a ready `profile`. A profile whose engine does not match
the browser fails the session instead of running with contradicting values. Safari is not supported; automation
markers are not hidden. Firefox has no timezone pref: run the driver with `TZ=<profile.timezone>`.

Tests: `npm test` (unit, stand-in browser). With `VENEER_E2E=1`, `VENEER_API`, `CHROMEDRIVER` and `webdriverio`
installed, a real standalone WebdriverIO 10 session against Chromium is verified with the collector.
The test-runner integration (`wdio run`) and the Firefox path were not run.
