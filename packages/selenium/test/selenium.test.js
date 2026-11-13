'use strict';

// Unit tests with selenium-webdriver Options objects (when installed) and a stand-in driver.
// A real WebDriver session is not started here: that needs a driver binary matching the browser.
const test = require('node:test');
const assert = require('node:assert/strict');
const veneer = require('@llcforge/veneer');
const sel = require('..');

function tryRequire(name) {
  try {
    return require(name);
  } catch {
    return undefined;
  }
}
const chrome = tryRequire('selenium-webdriver/chrome');
const firefox = tryRequire('selenium-webdriver/firefox');

test('chromium switches land on chrome.Options', { skip: !chrome }, () => {
  const profile = veneer.generate({ os: 'windows', browser: 'chrome', seed: 'sel-chrome' });
  const options = sel.chromiumOptions(profile, new chrome.Options());
  const args = options.get('goog:chromeOptions').args;
  assert.deepEqual(args, veneer.chromiumArgs(profile));
});

test('firefox prefs land on firefox.Options', { skip: !firefox }, () => {
  const profile = veneer.generate({ os: 'linux', browser: 'firefox', seed: 'sel-ff' });
  const options = sel.firefoxOptions(profile, new firefox.Options());
  const prefs = options.get('moz:firefoxOptions').prefs;
  assert.equal(prefs['general.useragent.override'], profile.navigator.userAgent);
  assert.equal(prefs['privacy.resistFingerprinting'], false);
  assert.equal(prefs['intl.accept_languages'], profile.navigator.languages.join(', '));
});

test('applyProfile uses CDP for chromium and BiDi for gecko', async () => {
  const chromeProfile = veneer.generate({ os: 'macos', browser: 'chrome', seed: 'sel-cdp' });
  const sent = [];
  await sel.applyProfile({ sendDevToolsCommand: async (m, p) => sent.push([m, p]) }, chromeProfile);
  assert.equal(sent[0][0], 'Emulation.setUserAgentOverride');
  assert.equal(sent.at(-1)[0], 'Page.addScriptToEvaluateOnNewDocument');

  const ffProfile = veneer.generate({ os: 'windows', browser: 'firefox', seed: 'sel-bidi' });
  let declared;
  const id = await sel.applyBidiPreloadScript({}, ffProfile, {
    scriptManager: async (contexts) => {
      assert.deepEqual(contexts, []);
      return { addPreloadScript: async (fn) => ((declared = fn), 'script-1') };
    },
  });
  assert.equal(id, 'script-1');
  assert.ok(declared.startsWith('() => {'));
  // The declaration must parse as a function.
  assert.equal(typeof new Function(`return (${declared})`)(), 'function');
});

test('rejects wrong objects and engines', async () => {
  const profile = veneer.generate({ os: 'windows', browser: 'chrome', seed: 'sel-bad' });
  assert.throws(() => sel.chromiumOptions(profile, {}), /addArguments/);
  await assert.rejects(sel.applyCdp({}, profile), /sendDevToolsCommand/);
  const safari = veneer.generate({ device: 'iphone-15', seed: 'sel-safari' });
  await assert.rejects(sel.applyProfile({}, safari), /chromium or gecko/);
});

// Real browser: VENEER_E2E=1, VENEER_API, CHROMEDRIVER (driver binary matching the Chromium major) and
// `playwright` resolvable for the Chromium binary path.
test('ChromeDriver session matches the profile', { skip: !(process.env.VENEER_E2E && process.env.CHROMEDRIVER && chrome) }, async () => {
  const { Builder } = require('selenium-webdriver');
  const profile = veneer.generate({ os: 'windows', browser: 'chrome', locale: 'tr-TR', seed: 'sel-e2e' });
  const options = sel.chromiumOptions(profile, new chrome.Options());
  options.setChromeBinaryPath(require('playwright').chromium.executablePath());
  options.addArguments('--headless=new');
  const driver = await new Builder()
    .forBrowser('chrome')
    .setChromeOptions(options)
    .setChromeService(new chrome.ServiceBuilder(process.env.CHROMEDRIVER))
    .build();
  try {
    await sel.applyProfile(driver, profile);
    await driver.get(`${process.env.VENEER_API}/`);
    const observed = await driver.executeAsyncScript(
      `const done = arguments[arguments.length - 1]; Promise.resolve(${veneer.collectorScript()}).then(done);`,
    );
    const report = await veneer.compareObserved(profile, observed);
    assert.deepEqual(report.issues.filter((i) => i.severity === 'error'), []);
  } finally {
    await driver.quit();
  }
});
