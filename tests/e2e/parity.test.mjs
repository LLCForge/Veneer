// The WASM build and the native CLI must produce identical profiles, init scripts, headers and reports for the
// same seeds. Run: VENEER_BIN=<path to veneer> node --test parity.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const require = createRequire(import.meta.url);
const here = path.dirname(fileURLToPath(import.meta.url));
const BIN = process.env.VENEER_BIN;
if (!BIN) throw new Error('set VENEER_BIN to the native veneer binary');
const wasm = require(process.env.VENEER_WASM ?? path.join(here, '..', '..', 'packages', 'veneer', 'wasm', 'veneer_wasm.js'));

const cli = (args, input) => execFileSync(BIN, args, { input, maxBuffer: 64 * 1024 * 1024 }).toString();
const native = (c) =>
  JSON.parse(cli(['generate', '--full', '--format', 'json', ...Object.entries(c).flatMap(([k, v]) => [`--${k}`, v])]));

const catalog = JSON.parse(cli(['catalog']));

/** String equality that reports the first difference instead of two multi-kilobyte strings. */
function sameText(actual, expected, what) {
  if (actual === expected) return;
  let i = 0;
  while (i < actual.length && actual[i] === expected[i]) i++;
  assert.fail(`${what}: differs at ${i} (lengths ${actual.length}/${expected.length}): ${JSON.stringify(actual.slice(i, i + 80))} vs ${JSON.stringify(expected.slice(i, i + 80))}`);
}

/** Every OS and browser, a few seeds each, plus unconstrained seeds. */
function cases() {
  const list = [];
  for (const os of catalog.os) for (let i = 0; i < 4; i++) list.push({ os: os.id ?? os, seed: `parity-os-${i}` });
  for (const b of catalog.browsers) for (let i = 0; i < 3; i++) list.push({ browser: b.id ?? b, seed: `parity-browser-${i}` });
  for (let i = 0; i < 40; i++) list.push({ seed: `parity-any-${i}` });
  return list;
}

test('catalog is identical', () => {
  assert.deepEqual(wasm.catalog(), catalog);
});

test('collector script is identical', () => {
  sameText(wasm.collectorScript().trimEnd(), cli(['collector']).trimEnd(), 'collector');
});

test('profiles, init scripts, headers and reports are identical for the same seeds', () => {
  for (const c of cases()) {
    const expected = native(c);
    const profile = wasm.generate(c);
    const what = JSON.stringify(c);
    assert.deepEqual(profile, expected.profile, `profile ${what}`);
    sameText(wasm.toInitScript(profile), expected.initScript, `init script ${what}`);
    assert.deepEqual(wasm.navigationHeaders(profile), expected.headers, `headers ${what}`);
    assert.deepEqual(wasm.validate(profile), expected.report, `report ${what}`);
  }
});
