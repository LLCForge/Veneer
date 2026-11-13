'use strict';

const veneer = require('@llcforge/veneer');

function loadPlaywrightTest() {
  try {
    return require('@playwright/test');
  } catch (err) {
    if (err && err.code !== 'MODULE_NOT_FOUND') throw err;
    // `playwright/test` is the same runner when only the `playwright` package is installed.
    return require('playwright/test');
  }
}

const { test: base, expect } = loadPlaywrightTest();

const ENGINE_OF = { chromium: 'chromium', firefox: 'gecko', webkit: 'webkit' };

/** Constraints for the browser under test: the engine follows `browserName` unless a browser is pinned. */
function constraintsFor(options, browserName, seed) {
  if (options === null || typeof options !== 'object' || Array.isArray(options)) {
    throw new TypeError('veneer option must be an object of constraints');
  }
  const constraints = { ...options };
  if (constraints.seed == null) constraints.seed = seed;
  if (constraints.browser == null && constraints.engine == null) constraints.engine = ENGINE_OF[browserName];
  return constraints;
}

async function overrideHeaders(context, page, profile) {
  const session = await context.newCDPSession(page);
  await session.send('Emulation.setUserAgentOverride', veneer.userAgentOverride(profile));
}

const test = base.extend({
  /** Constraints for `veneer.generate`, e.g. `{ os: 'macos', locale: 'tr-TR' }`; `undefined` disables the fixture. */
  veneer: [undefined, { option: true }],

  veneerProfile: async ({ veneer: options, browserName }, use, testInfo) => {
    if (options === undefined) {
      await use(undefined);
      return;
    }
    // Stable per test, so retries and reruns see the same fingerprint.
    const profile = veneer.generate(constraintsFor(options, browserName, testInfo.titlePath.join(' > ')));
    if (veneer.playwrightEngine(profile) !== browserName) {
      throw new Error(`veneer profile needs ${veneer.playwrightEngine(profile)}, but this project runs ${browserName}`);
    }
    await use(profile);
  },

  contextOptions: async ({ contextOptions, veneerProfile }, use) => {
    await use(veneerProfile ? { ...contextOptions, ...veneer.contextOptions(veneerProfile) } : contextOptions);
  },

  context: async ({ context, veneerProfile, browserName }, use) => {
    if (veneerProfile) {
      await context.addInitScript(veneer.toInitScript(veneerProfile));
      if (browserName === 'chromium') {
        context.on('page', (page) => {
          overrideHeaders(context, page, veneerProfile).catch(() => {
            // Page closed before the override could be applied.
          });
        });
      }
    }
    await use(context);
  },

  page: async ({ context, veneerProfile, browserName }, use) => {
    const page = await context.newPage();
    // Await the header override for the main page so its first navigation already carries client hints.
    if (veneerProfile && browserName === 'chromium') await overrideHeaders(context, page, veneerProfile);
    await use(page);
  },
});

module.exports = { test, expect, constraintsFor };
