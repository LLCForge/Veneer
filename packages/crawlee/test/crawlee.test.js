'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const veneer = require('@llcforge/veneer');
const { veneerBrowserPoolOptions } = require('..');

function fakeLaunchContext(name, sessionId) {
  return {
    id: 'launch-1',
    session: sessionId ? { id: sessionId } : undefined,
    browserPlugin: { library: { name: () => name } },
    launchOptions: { args: ['--foo'] },
  };
}

test('same session gives the same fingerprint, the engine follows the plugin', () => {
  const opts = veneerBrowserPoolOptions({ constraints: { os: 'windows' } });
  assert.equal(opts.useFingerprints, false);
  const a = fakeLaunchContext('chromium', 'session_A');
  const b = fakeLaunchContext('chromium', 'session_A');
  opts.preLaunchHooks[0]('p1', a);
  opts.preLaunchHooks[0]('p2', b);
  const pa = opts.profileOf({ launchContext: a });
  assert.deepEqual(pa, opts.profileOf({ launchContext: b }));
  assert.equal(pa.browser.engine, 'chromium');
  assert.equal(a.launchOptions.userAgent, pa.navigator.userAgent);
  assert.ok(a.launchOptions.args.includes('--foo'));
  assert.ok(a.launchOptions.args.includes(`--lang=${pa.locale}`));

  const c = fakeLaunchContext('chromium', 'session_C');
  opts.preLaunchHooks[0]('p4', c);
  assert.notEqual(opts.profileOf({ launchContext: c }).seed, pa.seed);

  const ff = fakeLaunchContext('firefox', 'session_B');
  opts.preLaunchHooks[0]('p3', ff);
  assert.equal(opts.profileOf({ launchContext: ff }).browser.engine, 'gecko');
  assert.equal(ff.launchOptions.firefoxUserPrefs['general.useragent.override'], ff.launchOptions.userAgent);
});

test('validates options', () => {
  assert.throws(() => veneerBrowserPoolOptions({ seedPrefix: 'x'.repeat(200) }), /seedPrefix/);
  assert.throws(() => veneerBrowserPoolOptions({ onProfile: 1 }), /onProfile/);
});

for (const useIncognitoPages of [false, true]) {
  test(`BrowserPool + Playwright Chromium (useIncognitoPages: ${useIncognitoPages})`, { skip: !process.env.VENEER_E2E }, async () => {
    const { BrowserPool, PlaywrightPlugin } = require('@crawlee/browser-pool');
    const playwright = require('playwright');
    const options = veneerBrowserPoolOptions({ constraints: { os: 'macos', locale: 'fr-FR' } });
    const pool = new BrowserPool({
      ...options,
      browserPlugins: [new PlaywrightPlugin(playwright.chromium, { launchOptions: { headless: true }, useIncognitoPages })],
    });
    try {
      const page = await pool.newPage();
      const profile = options.profileOf(pool.getBrowserControllerByPage(page));
      let sent;
      page.on('request', (r) => {
        if (r.isNavigationRequest()) sent = r.headers();
      });
      await page.goto(`${process.env.VENEER_API}/`);
      const report = await veneer.verifyPage(page, profile);
      assert.deepEqual(report.issues.filter((i) => i.severity === 'error'), []);
      assert.equal(sent['user-agent'], profile.navigator.userAgent);
      assert.equal(sent['sec-ch-ua-platform'], '"macOS"');
      assert.match(sent['accept-language'], /^fr-FR/);
    } finally {
      await pool.destroy();
    }
  });
}

test('BrowserPool + Puppeteer', { skip: !process.env.VENEER_E2E }, async () => {
  const { BrowserPool, PuppeteerPlugin } = require('@crawlee/browser-pool');
  const options = veneerBrowserPoolOptions({ constraints: { os: 'android' } });
  const pool = new BrowserPool({
    ...options,
    browserPlugins: [
      new PuppeteerPlugin(require('puppeteer-core'), {
        launchOptions: { headless: true, executablePath: require('playwright').chromium.executablePath() },
      }),
    ],
  });
  try {
    const page = await pool.newPage();
    const profile = options.profileOf(pool.getBrowserControllerByPage(page));
    await page.goto(`${process.env.VENEER_API}/`);
    const report = await veneer.verifyPage(page, profile);
    assert.deepEqual(report.issues.filter((i) => i.severity === 'error'), []);
  } finally {
    await pool.destroy();
  }
});
