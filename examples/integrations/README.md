# Integration examples

| File | What |
| --- | --- |
| `remote-browser.js` | Apply a profile to a remote Chromium (Browserless, Docker, a grid node) with Playwright `connectOverCDP` or Puppeteer `connect`. |
| `launch-config.js` | Print Chromium launch switches or a Firefox `user.js` for a profile. |
| `test.js` | Tests for both; the remote test uses a local Chromium with a DevTools port as the "remote". |

## Remote browsers (Browserless and similar)

```bash
# Browserless (token in the URL, read from the environment and never printed)
BROWSER_WS="wss://<your-browserless-host>?token=$BROWSERLESS_TOKEN" node remote-browser.js https://example.com
# Any Chromium started with --remote-debugging-port=9222
BROWSER_WS="$(curl -s http://127.0.0.1:9222/json/version | jq -r .webSocketDebuggerUrl)" CLIENT=puppeteer \
  node remote-browser.js https://example.com
```

Remote services run Chromium, so the example asks for `engine: 'chromium'`. Launch switches cannot be changed on a
browser that is already running; everything is applied per context/page instead (Playwright context options + init
script + CDP header override, or `veneer.cdpCommands` for Puppeteer). With Puppeteer the example disconnects instead of
closing, since the remote service owns the browser. The printed JSON holds the seed so a run can be reproduced.

## Launch configuration

```bash
node launch-config.js chromium --os windows --locale tr-TR
node launch-config.js firefox --os linux --locale de-DE > ~/.mozilla/firefox/<profile>/user.js
```

Chromium switches (`veneer.chromiumArgs`): `--lang`, `--user-agent`, `--window-size` (the available screen area, as a
maximised window), `--force-device-scale-factor`. Switches that hide automation, such as
`--disable-blink-features=AutomationControlled`, are never added.

Firefox prefs (`veneer.firefoxPrefs` / `firefoxUserJs`): `general.useragent.override`, `general.appversion.override`,
`general.platform.override`, `general.oscpu.override`, `intl.accept_languages`, `intl.locale.requested`,
`dom.maxHardwareConcurrency`, `layout.css.devPixelsPerPx`, and `privacy.resistFingerprinting=false` (it would
otherwise replace these values). Firefox has no timezone pref: start it with `TZ=<profile.timezone>`.
Font allow-listing is not generated, because the matching pref could not be verified against a primary source.

Run: `node --test examples/integrations/test.js` (with `VENEER_E2E=1` and `VENEER_API` for the remote test).
