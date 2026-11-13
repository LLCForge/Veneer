# @llcforge/veneer-crawlee

[Crawlee](https://crawlee.dev) integration for [Veneer](../veneer): every browser in the pool gets its own coherent
profile, seeded by the Crawlee session that launched it, so a session keeps the same fingerprint when it comes back.

```js
const { PlaywrightCrawler } = require('crawlee');
const { veneerBrowserPoolOptions } = require('@llcforge/veneer-crawlee');

const veneerPool = veneerBrowserPoolOptions({ constraints: { os: 'windows', locale: 'de-DE' } });

const crawler = new PlaywrightCrawler({
  browserPoolOptions: veneerPool,
  async requestHandler({ page, browserController, request, log }) {
    const profile = veneerPool.profileOf(browserController);
    log.info(`${request.url} as ${profile.device.id} / ${profile.browser.id} (seed ${profile.seed})`);
  },
});
await crawler.run(['https://example.com']);
```

To add your own hooks, merge the arrays:

```js
browserPoolOptions: {
  ...veneerPool,
  preLaunchHooks: [...veneerPool.preLaunchHooks, myPreLaunchHook],
  postPageCreateHooks: [...veneerPool.postPageCreateHooks, myPostPageCreateHook],
}
```

What happens:

- **Pre-launch:** a profile is generated with seed `<seedPrefix>:<session id>` (or the launch id without a session pool);
  its engine follows the browser plugin (Playwright `chromium`/`firefox`/`webkit`, Puppeteer → Chromium).
  Playwright context options (user agent, locale, timezone, viewport, screen, scale, touch) go into `launchOptions`,
  which Crawlee uses for persistent contexts and for incognito pages alike. Chromium also gets `veneer.chromiumArgs`,
  Firefox gets `veneer.firefoxPrefs` as `firefoxUserPrefs`.
- **Post-page-create:** Playwright pages get the init script and, on Chromium, the CDP user-agent override (client hints,
  `Accept-Language`); Puppeteer pages get all of `veneer.cdpCommands`.
- `useFingerprints: false` turns off Crawlee's own fingerprint injector, which would contradict the profile.
- `profileOf` is not enumerable, so the object can be passed (or spread) into `browserPoolOptions` directly.

| Option | Default | Meaning |
| --- | --- | --- |
| `constraints` | `{}` | Constraints for `veneer.generate` (the engine is set from the plugin). |
| `seedPrefix` | `crawlee` | Prefix of every seed (≤ 128 characters). |
| `onProfile` | — | `(profile, launchContext) => void`, called for each new browser. |

One browser has one fingerprint, so pages of other sessions that share a browser share its fingerprint; use
`maxOpenPagesPerBrowser: 1` or retire browsers with their sessions when you need strict one-session-one-fingerprint.

Tests: `npm test`; with `VENEER_E2E=1`, `VENEER_API`, `@crawlee/browser-pool`, `playwright` and `puppeteer-core`
resolvable, real-browser tests run BrowserPool with Playwright (persistent and incognito) and Puppeteer.
