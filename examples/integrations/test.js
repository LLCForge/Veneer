'use strict';

// node --test examples/integrations/test.js
// The remote-browser test starts a local Chromium with a DevTools port as the "remote" (VENEER_E2E=1, VENEER_API).
const test = require('node:test');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const { mkdtempSync, rmSync } = require('node:fs');
const { tmpdir } = require('node:os');
const path = require('node:path');
const veneer = require('@llcforge/veneer');
const { parse, render } = require('./launch-config.js');

test('launch-config renders chromium switches and a firefox user.js', () => {
  const c = parse(['chromium', '--os', 'windows', '--seed', 'ex-1']);
  const chromiumOut = render(c.target, veneer.generate(c.constraints));
  assert.match(chromiumOut, /^--lang=/);
  assert.doesNotMatch(chromiumOut, /AutomationControlled/);
  const f = parse(['firefox', '--os', 'linux', '--seed', 'ex-2']);
  const userJs = render(f.target, veneer.generate(f.constraints));
  assert.match(userJs, /user_pref\("general\.useragent\.override", "Mozilla\/5\.0 \(X11;/);
  assert.match(userJs, /user_pref\("privacy\.resistFingerprinting", false\);/);
  assert.throws(() => parse(['webkit']), /chromium or firefox/);
  assert.throws(() => parse(['chromium', '--evil', 'x']), /bad flag/);
});

async function withLocalChromium(fn) {
  const { chromium } = require('playwright');
  const port = 9800 + Math.floor(Math.random() * 100);
  const dir = mkdtempSync(path.join(tmpdir(), 'veneer-remote-'));
  const proc = spawn(chromium.executablePath(), ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${dir}`, '--no-first-run', 'about:blank']);
  try {
    let version;
    for (let i = 0; i < 50 && !version; i++) {
      version = await fetch(`http://127.0.0.1:${port}/json/version`).then((r) => r.json(), () => undefined);
      if (!version) await new Promise((r) => setTimeout(r, 200));
    }
    return await fn(version.webSocketDebuggerUrl);
  } finally {
    proc.kill();
    await new Promise((r) => setTimeout(r, 500));
    // Chromium on Windows can hold files briefly after exit; a leftover temp dir is harmless.
    try {
      rmSync(dir, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
    } catch {}
  }
}

for (const client of ['playwright', 'puppeteer']) {
  test(`remote-browser example with ${client}`, { skip: !process.env.VENEER_E2E }, async () => {
    const { withPlaywright, withPuppeteer } = require('./remote-browser.js');
    const profile = veneer.generate({ engine: 'chromium', os: 'macos', seed: `remote-${client}` });
    const report = await withLocalChromium((ws) => (client === 'playwright' ? withPlaywright : withPuppeteer)(ws, profile, `${process.env.VENEER_API}/`));
    assert.deepEqual(report.issues.filter((i) => i.severity === 'error'), []);
  });
}
