# @llcforge/veneer-cypress

[Cypress](https://www.cypress.io) plugin for [Veneer](../veneer): one coherent profile per run.

```js
// cypress.config.js
const { defineConfig } = require('cypress');
const { setupVeneer } = require('@llcforge/veneer-cypress');

module.exports = defineConfig({
  e2e: {
    setupNodeEvents(on, config) {
      return setupVeneer(on, config, { constraints: { os: 'macos', locale: 'tr-TR' } });
    },
  },
});
```

```js
// cypress/support/e2e.js
require('@llcforge/veneer-cypress/support');
```

In a test, `Cypress.env('veneerProfile')` is the applied profile.

What it does:

| Where | What |
| --- | --- |
| config | `userAgent`, `viewportWidth`, `viewportHeight` from the profile |
| `before:browser:launch` (Chromium family: Chrome, Edge, Electron) | `veneer.chromiumArgs(profile)` and `TZ` |
| `before:browser:launch` (Firefox) | `veneer.firefoxPrefs(profile)` as preferences and `TZ` |
| support file, every app window | the init script, evaluated on `window:before:load` (before the app's scripts) |
| support file, Chromium | user-agent override with client hints and `Accept-Language`, timezone and locale over Cypress's CDP bridge (`Cypress.automation('remote:debugger:protocol', …)`) |

The profile's engine defaults to Chromium; pass `constraints: { engine: 'gecko' }` and run with `--browser firefox`
for a Firefox profile. Launching a browser of another family fails with a clear error instead of running with a
contradicting fingerprint. WebKit profiles are not supported. Automation markers are not hidden.

Notes: `remote:debugger:protocol` is an automation event Cypress uses internally and that community plugins rely
on; it is not part of Cypress's typed public API.

Tests: `npm test` checks the config and launch hooks with a stand-in `on`. These tests did not launch Cypress.
