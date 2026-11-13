# @llcforge/veneer

Coherent browser fingerprints for Node.js, built from the [Veneer](https://github.com/LLCForge/Veneer) Rust core compiled to WebAssembly.

```bash
npm install @llcforge/veneer
```

## Playwright

```js
const playwright = require('playwright');
const { generate, launchWithProfile, verifyPage } = require('@llcforge/veneer');

const profile = generate({ device: 'iphone-17-pro', browser: 'safari', locale: 'tr', seed: 'session-42' });
const { browser, page } = await launchWithProfile(playwright, profile);
await page.goto('https://example.com');
console.log(await verifyPage(page, profile));
await browser.close();
```

`launchWithProfile` starts the matching engine (Chromium, Firefox or WebKit), applies context options, the init script and, on Chromium, client-hint request headers and emulated media for every page and popup.

## Puppeteer

```js
const { generate, applyProfile } = require('@llcforge/veneer');
const page = await browser.newPage();
await applyProfile(page, generate({ os: 'windows', browser: 'chrome' }));
```

## API

| Function | Purpose |
| --- | --- |
| `generate(constraints)` | New profile; a random seed is chosen and returned when none is given |
| `validate(profile)` | Consistency report with score and fixes |
| `toInitScript(profile)`, `collectorScript()`, `compareObserved(profile, observed)` | Browser-side patch, in-page collector, live comparison |
| `navigationHeaders(profile)`, `compareHeaders(profile, headers)` | Request headers in browser order; captured-header check |
| `contextOptions`, `userAgentMetadata`, `userAgentOverride`, `cdpCommands`, `chromiumArgs`, `firefoxPrefs` | Building blocks for other automation tools |
| `newContext`, `newPage`, `launchWithProfile`, `applyProfile`, `verifyPage` | Playwright and Puppeteer helpers |
| `catalog()` | Every OS, browser, device and locale id |

TypeScript types are in `index.d.ts`.

## veneer-verify

```bash
npx veneer-verify --device android-galaxy-s26-ultra --locale tr
```

Opens the fingerprint in a real browser with Playwright, checks it live and opens fingerprint test sites. Requires `playwright` in your project.

MIT licensed.
