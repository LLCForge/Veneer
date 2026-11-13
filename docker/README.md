# Playwright + Veneer image

`docker/Dockerfile` builds a ready test environment from this repository:

- stage 1 (`rust:1.94-slim-bookworm`): the WASM build of `@llcforge/veneer` (wasm-bindgen version from `Cargo.lock`)
  and the native `veneer` CLI;
- stage 2 (`mcr.microsoft.com/playwright:v1.64.1-noble`, browsers included): `playwright` and `@playwright/test`
  1.64.1, `@llcforge/veneer`, `@llcforge/veneer-playwright`, `@llcforge/veneer-cdp`, `veneer` on `PATH`, running as
  the image's unprivileged `pwuser`.

```bash
docker build -f docker/Dockerfile -t veneer-playwright .
docker run --rm --init --ipc=host -v "$PWD/tests:/app/tests" veneer-playwright npx playwright test
docker run --rm veneer-playwright veneer generate --os macos --format json
```

A spec in `tests/`:

```js
const { test, expect } = require('@llcforge/veneer-playwright');
test.use({ veneer: { os: 'windows', locale: 'tr-TR' } });
test('profile applied', async ({ page, veneerProfile }) => {
  await page.goto('https://example.com');
  expect(await page.evaluate(() => navigator.userAgent)).toBe(veneerProfile.navigator.userAgent);
});
```

The default `playwright.config.js` runs Chromium, Firefox and WebKit projects; the fixture picks a profile whose
engine matches each project. `Dockerfile.dockerignore` keeps `target/`, `node_modules` and local WASM builds out of
the build context. When `wasm-bindgen` in `Cargo.lock` changes, pass `--build-arg WASM_BINDGEN_VERSION=<version>`.

Validation: the Dockerfile was checked with `dockerfile-utils` (no problems). The image has not been built or run
here, and nothing is pushed to a registry.
