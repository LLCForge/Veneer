'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const { constraintsFor } = require('..');

test('engine follows the project browser unless a browser is pinned', () => {
  assert.deepEqual(constraintsFor({ os: 'macos' }, 'webkit', 's'), { os: 'macos', seed: 's', engine: 'webkit' });
  assert.deepEqual(constraintsFor({ os: 'linux' }, 'firefox', 's'), { os: 'linux', seed: 's', engine: 'gecko' });
  assert.deepEqual(constraintsFor({ browser: 'edge', seed: 'x' }, 'chromium', 's'), { browser: 'edge', seed: 'x' });
});

test('rejects non-object options', () => {
  assert.throws(() => constraintsFor('macos', 'chromium', 's'), /object/);
  assert.throws(() => constraintsFor(null, 'chromium', 's'), /object/);
});
