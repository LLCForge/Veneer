'use strict';

const veneer = require('@llcforge/veneer');

const ENGINE_OF = { chromium: 'chromium', firefox: 'gecko', webkit: 'webkit' };
const MAX_PREFIX = 128;

function libraryOf(launchContext) {
  const library = launchContext && launchContext.browserPlugin ? launchContext.browserPlugin.library : undefined;
  if (!library) throw new Error('launchContext has no browser plugin library');
  // Playwright browser types have name(); Puppeteer only drives Chromium here.
  if (typeof library.name === 'function') return { playwright: true, name: library.name() };
  return { playwright: false, name: 'chromium' };
}

/**
 * `browserPoolOptions` for Crawlee crawlers (PlaywrightCrawler, PuppeteerCrawler) or a bare BrowserPool:
 * every browser gets its own Veneer profile, seeded by the Crawlee session that launched it (or the launch id),
 * so the same session gets the same fingerprint again.
 */
function veneerBrowserPoolOptions(options = {}) {
  if (options === null || typeof options !== 'object') throw new TypeError('options must be an object');
  const { constraints = {}, seedPrefix = 'crawlee', onProfile } = options;
  if (typeof seedPrefix !== 'string' || seedPrefix.length > MAX_PREFIX) {
    throw new TypeError('seedPrefix must be a string of at most 128 characters');
  }
  if (onProfile !== undefined && typeof onProfile !== 'function') throw new TypeError('onProfile must be a function');
  const profiles = new WeakMap();

  const preLaunchHook = (pageId, launchContext) => {
    const { playwright, name } = libraryOf(launchContext);
    const key = (launchContext.session && launchContext.session.id) || launchContext.id || pageId;
    const profile = veneer.generate({ ...constraints, engine: ENGINE_OF[name], seed: `${seedPrefix}:${key}` });
    profiles.set(launchContext, profile);
    if (onProfile) onProfile(profile, launchContext);

    const launchOptions = launchContext.launchOptions || {};
    launchContext.launchOptions = launchOptions;
    if (playwright) {
      // Crawlee passes launchOptions to launchPersistentContext, or to newPage() for incognito pages.
      Object.assign(launchOptions, veneer.contextOptions(profile));
      if (name === 'firefox') {
        launchOptions.firefoxUserPrefs = { ...launchOptions.firefoxUserPrefs, ...veneer.firefoxPrefs(profile) };
      }
    }
    if (name === 'chromium') {
      launchOptions.args = [...(launchOptions.args || []), ...veneer.chromiumArgs(profile)];
    }
  };

  const postPageCreateHook = async (page, browserController) => {
    const profile = profiles.get(browserController.launchContext);
    if (!profile) return;
    if (typeof page.context === 'function') {
      await page.addInitScript(veneer.toInitScript(profile));
      if (profile.browser.engine === 'chromium') {
        const session = await page.context().newCDPSession(page);
        const [uaOverride] = veneer.cdpCommands(profile);
        await session.send(uaOverride.method, uaOverride.params);
      }
    } else {
      const session = await page.createCDPSession();
      await veneer.applyCdpCommands((method, params) => session.send(method, params), profile);
    }
  };

  const poolOptions = {
    // Crawlee's own fingerprint injector would fight with the Veneer profile.
    useFingerprints: false,
    preLaunchHooks: [preLaunchHook],
    postPageCreateHooks: [postPageCreateHook],
  };
  // Not enumerable: BrowserPool rejects unknown option keys, and spreading drops this one.
  Object.defineProperty(poolOptions, 'profileOf', {
    value: (browserController) => profiles.get(browserController.launchContext),
  });
  return poolOptions;
}

module.exports = { veneerBrowserPoolOptions };
