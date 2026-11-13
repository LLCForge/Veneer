'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const { readFileSync } = require('node:fs');
const veneer = require('@llcforge/veneer');
const { parseArgs, bundle } = require('../bundle.js');

const root = path.join(__dirname, '..');

test('bundle has allowed commands only', () => {
  const b = bundle(parseArgs(['--os', 'macos', '--seed', 'ext-1']));
  const source = readFileSync(path.join(root, 'background.js'), 'utf8');
  for (const { method } of b.commands) assert.ok(source.includes(`'${method}'`), `${method} must be allowed`);
  assert.equal(b.profile.seed, 'ext-1');
});

test('argument validation', () => {
  assert.throws(() => parseArgs(['--evil', 'x']), /unknown flag/);
  assert.throws(() => parseArgs(['--os']), /needs a value/);
  assert.throws(() => parseArgs(['--os', 'x'.repeat(200)]), /needs a value/);
  assert.deepEqual(parseArgs(['--locale', 'tr-TR']), { locale: 'tr-TR', engine: 'chromium' });
});

test('locales have the same keys and the manifest is valid JSON', () => {
  const en = JSON.parse(readFileSync(path.join(root, '_locales/en/messages.json'), 'utf8'));
  const tr = JSON.parse(readFileSync(path.join(root, '_locales/tr/messages.json'), 'utf8'));
  assert.deepEqual(Object.keys(tr).sort(), Object.keys(en).sort());
  const manifest = JSON.parse(readFileSync(path.join(root, 'manifest.json'), 'utf8'));
  assert.equal(manifest.manifest_version, 3);
  assert.deepEqual(manifest.permissions, ['debugger', 'storage']);
});

// Real browser: VENEER_E2E=1, VENEER_API and `playwright` resolvable (Chromium with --load-extension).
test('extension applies a bundle in real Chromium', { skip: !process.env.VENEER_E2E }, async () => {
  const { chromium } = require('playwright');
  const context = await chromium.launchPersistentContext('', {
    channel: 'chromium',
    headless: true,
    args: [`--disable-extensions-except=${root}`, `--load-extension=${root}`],
  });
  try {
    const worker = context.serviceWorkers()[0] || (await context.waitForEvent('serviceworker'));
    const page = await context.newPage();
    const url = `${process.env.VENEER_API}/`;
    await page.goto(url);
    await page.bringToFront();
    const b = bundle(parseArgs(['--os', 'android', '--locale', 'tr-TR', '--seed', 'ext-e2e']));
    const profile = veneer.generate({ os: 'android', locale: 'tr-TR', seed: 'ext-e2e', engine: 'chromium' });

    const rejected = await worker.evaluate(async (u) => {
      const [tab] = await chrome.tabs.query({ active: true, lastFocusedWindow: true });
      try {
        await globalThis.veneerApply(tab.id, [{ method: 'Runtime.evaluate', params: { expression: '1' } }]);
        return 'accepted';
      } catch (err) {
        return String(err.message);
      }
    }, url);
    assert.match(rejected, /not allowed/);

    const reloaded = page.waitForEvent('load');
    await worker.evaluate(
      async ([u, commands]) => {
        const [tab] = await chrome.tabs.query({ active: true, lastFocusedWindow: true });
        await globalThis.veneerApply(tab.id, commands);
      },
      [url, b.commands],
    );
    await reloaded;
    // Applying again replaces the session instead of failing or stacking init scripts.
    const again = page.waitForEvent('load');
    await worker.evaluate(async (commands) => {
      const [tab] = await chrome.tabs.query({ active: true, lastFocusedWindow: true });
      await globalThis.veneerApply(tab.id, commands);
    }, b.commands);
    await again;
    const report = await veneer.verifyPage(page, profile);
    assert.deepEqual(report.issues.filter((i) => i.severity === 'error'), []);
    assert.equal(await page.evaluate(() => navigator.userAgent), profile.navigator.userAgent);
  } finally {
    await context.close();
  }
});
