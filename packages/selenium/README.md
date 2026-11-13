# @llcforge/veneer-selenium

[Selenium WebDriver](https://www.selenium.dev/documentation/webdriver/) (JavaScript) helpers for [Veneer](../veneer).

## Chrome / Edge (CDP through ChromeDriver)

```js
const { Builder } = require('selenium-webdriver');
const chrome = require('selenium-webdriver/chrome');
const veneer = require('@llcforge/veneer');
const { chromiumOptions, applyProfile } = require('@llcforge/veneer-selenium');

const profile = veneer.generate({ os: 'windows', browser: 'chrome', locale: 'tr-TR' });
const driver = await new Builder()
  .forBrowser('chrome')
  .setChromeOptions(chromiumOptions(profile, new chrome.Options()))
  .build();
await applyProfile(driver, profile); // before the first driver.get()
await driver.get('https://example.com');
```

`chromiumOptions` adds `veneer.chromiumArgs(profile)` (`--lang`, `--user-agent`, `--window-size`,
`--force-device-scale-factor`). `applyProfile` sends `veneer.cdpCommands(profile)` with `driver.sendDevToolsCommand`:
user agent and client hints, `Accept-Language`, timezone, locale, device metrics, touch and the init script.
The overrides belong to the current tab; call `applyProfile` again after switching to a new tab or window.
For Edge use `edge.Options` and `forBrowser('MicrosoftEdge')` with an Edge profile (`browser: 'edge'`).

## Firefox (prefs + WebDriver BiDi)

```js
const firefox = require('selenium-webdriver/firefox');
const { firefoxOptions, applyProfile } = require('@llcforge/veneer-selenium');

const profile = veneer.generate({ os: 'linux', browser: 'firefox' });
const options = firefoxOptions(profile, new firefox.Options()).enableBidi();
const driver = await new Builder().forBrowser('firefox').setFirefoxOptions(options).build();
await applyProfile(driver, profile); // BiDi script.addPreloadScript with the init script
```

`firefoxOptions` sets `veneer.firefoxPrefs(profile)` (user agent, `appVersion`, platform, `oscpu`,
`intl.accept_languages`, `intl.locale.requested`, `dom.maxHardwareConcurrency`, `layout.css.devPixelsPerPx`,
`privacy.resistFingerprinting=false`) and the window size. Firefox has no timezone pref: start the driver with
`TZ=<profile.timezone>` in its environment (Linux/macOS).

`applyBidiPreloadScript(driver, profile)` works on any BiDi-enabled session, Chromium included.
Safari profiles need WebKit; use Playwright for those. Automation markers (`navigator.webdriver`) are left alone.

Tests: `npm test` (unit, uses `selenium-webdriver` Options when installed). With `VENEER_E2E=1`, `VENEER_API` and
`CHROMEDRIVER=<path to a chromedriver matching the Chromium major>`, a real ChromeDriver session is verified.
The Firefox/BiDi path is covered by unit tests only.
