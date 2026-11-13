'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const { mkdtempSync, rmSync } = require('node:fs');
const { tmpdir } = require('node:os');
const path = require('node:path');
const veneer = require('@llcforge/veneer');
const cdp = require('..');

test('applyToSession sends every command in order', async () => {
  const profile = veneer.generate({ os: 'android', browser: 'chrome', seed: 'cdp-unit' });
  const sent = [];
  await cdp.applyToSession(async (method, params) => sent.push({ method, params }), profile);
  assert.deepEqual(
    sent.map((c) => c.method),
    [
      'Emulation.setUserAgentOverride',
      'Emulation.setTimezoneOverride',
      'Emulation.setLocaleOverride',
      'Emulation.setDeviceMetricsOverride',
      'Emulation.setTouchEmulationEnabled',
      'Page.enable',
      'Page.addScriptToEvaluateOnNewDocument',
    ],
  );
  assert.equal(sent[0].params.userAgent, profile.navigator.userAgent);
  assert.equal(sent[3].params.mobile, true);
  assert.equal(sent[4].params.enabled, true);
});

test('non-Chromium profiles are rejected', async () => {
  const safari = veneer.generate({ device: 'iphone-15', seed: 'cdp-safari' });
  await assert.rejects(cdp.applyToSession(async () => {}, safari), /needs a chromium profile/);
});

test('connect rejects bad URLs', async () => {
  await assert.rejects(cdp.connect('http://127.0.0.1:1'), /ws:\/\//);
  await assert.rejects(cdp.connect('not a url'), /ws:\/\//);
});

// Real browser: set VENEER_E2E=1, VENEER_API (for compareObserved/page) and have `playwright` resolvable.
test('applies a profile to a raw Chromium target', { skip: !process.env.VENEER_E2E }, async () => {
  const { chromium } = require('playwright');
  const port = 9300 + Math.floor(Math.random() * 500);
  const userDataDir = mkdtempSync(path.join(tmpdir(), 'veneer-cdp-'));
  const proc = spawn(chromium.executablePath(), [
    '--headless=new',
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${userDataDir}`,
    '--no-first-run',
    'about:blank',
  ]);
  try {
    let targets;
    for (let i = 0; i < 50 && !targets; i++) {
      targets = await fetch(`http://127.0.0.1:${port}/json/list`).then((r) => r.json(), () => undefined);
      if (!targets) await new Promise((r) => setTimeout(r, 200));
    }
    const page = targets.find((t) => t.type === 'page');
    const profile = veneer.generate({ os: 'windows', browser: 'chrome', locale: 'tr-TR', seed: 'cdp-e2e' });
    const client = await cdp.applyToTarget(page.webSocketDebuggerUrl, profile);
    await client.send('Page.navigate', { url: `${process.env.VENEER_API}/` });
    await new Promise((r) => setTimeout(r, 1500));
    const { result } = await client.send('Runtime.evaluate', {
      expression: veneer.collectorScript(),
      awaitPromise: true,
      returnByValue: true,
    });
    client.close();
    assert.equal(result.value.userAgent, profile.navigator.userAgent);
    assert.equal(result.value.timezone, profile.timezone);
    const report = await veneer.compareObserved(profile, result.value);
    const errors = report.issues.filter((i) => i.severity === 'error');
    assert.deepEqual(errors, []);
  } finally {
    proc.kill();
    await new Promise((r) => setTimeout(r, 500));
    // Chromium on Windows can hold files briefly after exit; a leftover temp dir is harmless.
    try {
      rmSync(userDataDir, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
    } catch {}
  }
});
