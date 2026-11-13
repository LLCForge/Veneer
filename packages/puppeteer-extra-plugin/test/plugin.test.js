'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const veneer = require('@llcforge/veneer');
const veneerPlugin = require('..');

test('plugin generates a chromium profile and adds launch switches', async () => {
  const plugin = veneerPlugin({ constraints: { os: 'windows', seed: 'pep-unit' } });
  assert.equal(plugin.name, 'veneer');
  assert.equal(plugin.profile.browser.engine, 'chromium');
  const options = await plugin.beforeLaunch({ args: ['--lang=en-US'] });
  assert.equal(options.args.filter((a) => a.startsWith('--lang=')).length, 1);
  assert.ok(options.args.includes('--lang=en-US'));
  assert.ok(options.args.some((a) => a === `--user-agent=${plugin.profile.navigator.userAgent}`));
  assert.ok(!options.args.some((a) => a.includes('AutomationControlled')));
});

test('rejects a non-chromium profile', () => {
  const safari = veneer.generate({ device: 'iphone-15', seed: 'pep-safari' });
  assert.throws(() => veneerPlugin({ profile: safari }), /chromium/);
});

test('works with puppeteer-extra and real Chromium', { skip: !process.env.VENEER_E2E }, async () => {
  const { addExtra } = require('puppeteer-extra');
  const puppeteer = addExtra(require('puppeteer-core'));
  const plugin = veneerPlugin({ constraints: { os: 'android', locale: 'de-DE', seed: 'pep-e2e' } });
  puppeteer.use(plugin);
  const browser = await puppeteer.launch({ executablePath: require('playwright').chromium.executablePath(), headless: true });
  try {
    const page = await browser.newPage();
    let sent;
    page.on('request', (r) => {
      if (r.isNavigationRequest()) sent = r.headers();
    });
    await page.goto(`${process.env.VENEER_API}/`);
    const report = await veneer.verifyPage(page, plugin.profile);
    assert.deepEqual(report.issues.filter((i) => i.severity === 'error'), []);
    assert.equal(sent['user-agent'], plugin.profile.navigator.userAgent);
    assert.equal(sent['sec-ch-ua-mobile'], '?1');
  } finally {
    await browser.close();
  }
});
