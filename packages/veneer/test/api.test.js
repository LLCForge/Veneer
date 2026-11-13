'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const veneer = require('..');

test('same seed gives the same profile', () => {
  const a = veneer.generate({ os: 'windows', seed: 'js-1' });
  assert.deepEqual(veneer.generate({ os: 'windows', seed: 'js-1' }), a);
  assert.equal(a.os.id, 'windows');
});

test('generated profiles validate cleanly across devices', () => {
  for (let i = 0; i < 300; i++) {
    const report = veneer.validate(veneer.generate({ seed: `js-${i}` }));
    assert.equal(report.valid, true);
    assert.deepEqual(report.issues, []);
  }
});

test('unseeded profiles get a random seed', () => {
  const a = veneer.generate();
  const b = veneer.generate();
  assert.notEqual(a.seed, b.seed);
});

test('helpers map engines and options', () => {
  const safari = veneer.generate({ device: 'iphone-15', seed: 'js-safari' });
  assert.equal(veneer.playwrightEngine(safari), 'webkit');
  assert.equal(veneer.userAgentMetadata(safari), undefined);
  assert.equal(veneer.contextOptions(safari).isMobile, true);

  const chrome = veneer.generate({ os: 'windows', browser: 'chrome', seed: 'js-chrome' });
  assert.equal(veneer.playwrightEngine(chrome), 'chromium');
  assert.equal(veneer.userAgentMetadata(chrome).platform, 'Windows');
  assert.ok(veneer.toInitScript(chrome).includes(chrome.navigator.userAgent));
});

test('invalid input throws', () => {
  assert.throws(() => veneer.generate({ os: 'beos' }), /unknown os/);
  assert.throws(() => veneer.generate({ colour: 'red' }), /invalid constraints/);
  assert.throws(() => veneer.generate('windows'), TypeError);
});
