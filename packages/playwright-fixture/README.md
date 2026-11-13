# @llcforge/veneer-playwright

Playwright Test fixture for [Veneer](../veneer). One line per file, project or `describe` block:

```js
// example.spec.js
const { test, expect } = require('@llcforge/veneer-playwright');

test.use({ veneer: { os: 'macos', locale: 'tr-TR' } });

test('looks like a Mac', async ({ page, veneerProfile }) => {
  await page.goto('https://example.com');
  expect(await page.evaluate(() => navigator.platform)).toBe(veneerProfile.navigator.platform);
});
```

Or for a whole project in `playwright.config.js`:

```js
projects: [
  { name: 'win-chrome', use: { browserName: 'chromium', veneer: { os: 'windows', browser: 'chrome' } } },
  { name: 'mac-safari', use: { browserName: 'webkit', veneer: { os: 'macos' } } },
  { name: 'linux-firefox', use: { browserName: 'firefox', veneer: { os: 'linux' } } },
]
```

- `veneer` takes the same constraints as `veneer.generate` (`os`, `browser`, `kind`, `device`, `locale`, `timezone`,
  `seed`, …). Unless you pin `browser` or `engine`, the profile's engine follows the project's `browserName`.
  A pinned browser that needs a different engine fails the test with a clear message.
- Without a `seed`, the test's title path is the seed, so every test has its own stable fingerprint across retries.
- The profile is applied through `contextOptions` (user agent, locale, timezone, viewport, screen, scale, touch),
  the init script on the context, and on Chromium a CDP `Emulation.setUserAgentOverride` per page (client-hint and
  `Accept-Language` headers, including popups). The `page` fixture waits for that override before your test starts.
- `veneerProfile` gives the test the applied profile (`undefined` when `veneer` is not set).

Works with `@playwright/test`, or with `playwright/test` when only the `playwright` package is installed.

Tests: `npm test` (unit) and `VENEER_API=http://127.0.0.1:<port> npx playwright test -c test/e2e` (real Chromium).
