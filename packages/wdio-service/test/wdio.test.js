'use strict';

// Unit tests with stand-in capabilities and browser objects; WebdriverIO itself is not started here.
const test = require('node:test');
const assert = require('node:assert/strict');
const veneer = require('@llcforge/veneer');
const VeneerService = require('..');

test('chrome: args in capabilities, CDP on every page', async () => {
  const service = new VeneerService({ constraints: { os: 'windows', seed: 'wdio-1' } });
  const caps = { browserName: 'chrome', 'goog:chromeOptions': { args: ['--headless=new'] } };
  service.beforeSession({}, caps);
  assert.deepEqual(caps['goog:chromeOptions'].args, ['--headless=new', ...veneer.chromiumArgs(service.profile)]);
  const sent = [];
  const page = { createCDPSession: async () => ({ send: async (m) => sent.push(m) }) };
  await service.before({}, [], { getPuppeteer: async () => ({ pages: async () => [page, page] }) });
  assert.equal(sent.length, 2 * veneer.cdpCommands(service.profile).length);
});

test('edge uses ms:edgeOptions', () => {
  const service = new VeneerService({ constraints: { browser: 'edge', seed: 'wdio-edge' } });
  const caps = { browserName: 'MicrosoftEdge' };
  service.beforeSession({}, caps);
  assert.ok(caps['ms:edgeOptions'].args.length > 0);
});

test('firefox: prefs plus BiDi preload script', async () => {
  const service = new VeneerService({ constraints: { os: 'linux', seed: 'wdio-ff' } });
  const caps = { alwaysMatch: { browserName: 'firefox', webSocketUrl: true } };
  service.beforeSession({}, caps);
  assert.equal(caps.alwaysMatch['moz:firefoxOptions'].prefs['general.useragent.override'], service.profile.navigator.userAgent);
  let decl;
  await service.before({}, [], { isBidi: true, scriptAddPreloadScript: async (p) => (decl = p.functionDeclaration) });
  assert.equal(typeof new Function(`return (${decl})`)(), 'function');
  await assert.rejects(service.before({}, [], { isBidi: false }), /BiDi/);
});

test('rejects unsupported browsers and mismatched profiles', () => {
  assert.throws(() => new VeneerService().beforeSession({}, { browserName: 'safari' }), /not "safari"/);
  const chromeProfile = veneer.generate({ os: 'windows', browser: 'chrome', seed: 'wdio-mm' });
  assert.throws(() => new VeneerService({ profile: chromeProfile }).beforeSession({}, { browserName: 'firefox' }), /chromium/);
});

// Real browser: VENEER_E2E=1, VENEER_API, CHROMEDRIVER, and `webdriverio` + `playwright` resolvable.
test('webdriverio standalone session with real Chromium', { skip: !(process.env.VENEER_E2E && process.env.CHROMEDRIVER) }, async () => {
  const { remote } = require('webdriverio');
  const service = new VeneerService({ constraints: { os: 'macos', locale: 'ja-JP', seed: 'wdio-e2e' } });
  const capabilities = {
    browserName: 'chrome',
    'goog:chromeOptions': { binary: require('playwright').chromium.executablePath(), args: ['--headless=new'] },
    'wdio:chromedriverOptions': { binary: process.env.CHROMEDRIVER },
  };
  service.beforeSession({}, capabilities);
  const browser = await remote({ capabilities, logLevel: 'error' });
  try {
    await service.before(capabilities, [], browser);
    await browser.url(`${process.env.VENEER_API}/`);
    const observed = await browser.execute(new Function(`return (${veneer.collectorScript()});`));
    const report = await veneer.compareObserved(service.profile, observed);
    assert.deepEqual(report.issues.filter((i) => i.severity === 'error'), []);
  } finally {
    await browser.deleteSession();
  }
});
