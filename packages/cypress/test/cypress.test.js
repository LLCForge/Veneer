'use strict';

// Unit tests with a stand-in `on`; Cypress itself is not launched here.
const test = require('node:test');
const assert = require('node:assert/strict');
const veneer = require('@llcforge/veneer');
const { setupVeneer } = require('..');

function run(options, browser) {
  let handler;
  const config = setupVeneer((event, fn) => {
    assert.equal(event, 'before:browser:launch');
    handler = fn;
  }, { env: { keep: 1 } }, options);
  const launch = { args: ['--x'], preferences: {}, env: {} };
  return { config, launch: browser ? handler(browser, launch) : undefined, handler };
}

test('chromium profile: config, args, env and CDP commands', () => {
  const { config, launch } = run({ constraints: { os: 'windows', seed: 'cy-1' } }, { family: 'chromium', name: 'chrome' });
  const p = config.env.veneerProfile;
  assert.equal(config.userAgent, p.navigator.userAgent);
  assert.equal(config.viewportWidth, p.viewport.width);
  assert.equal(config.env.keep, 1);
  assert.ok(config.env.veneerInitScript.includes(p.navigator.userAgent));
  assert.deepEqual(config.env.veneerCdp.map((c) => c.method), ['Emulation.setUserAgentOverride', 'Emulation.setTimezoneOverride', 'Emulation.setLocaleOverride']);
  assert.deepEqual(launch.args, ['--x', ...veneer.chromiumArgs(p)]);
  assert.equal(launch.env.TZ, p.timezone);
});

test('firefox profile sets preferences', () => {
  const { config, launch } = run({ constraints: { engine: 'gecko', os: 'linux', seed: 'cy-2' } }, { family: 'firefox', name: 'firefox' });
  assert.equal(launch.preferences['general.useragent.override'], config.userAgent);
  assert.deepEqual(config.env.veneerCdp, []);
});

test('wrong browser family fails loudly', () => {
  const { handler } = run({ constraints: { seed: 'cy-3' } });
  assert.throws(() => handler({ family: 'firefox', name: 'firefox' }, { args: [], preferences: {}, env: {} }), /chromium/);
  const safari = veneer.generate({ device: 'iphone-15', seed: 'cy-safari' });
  assert.throws(() => setupVeneer(() => {}, {}, { profile: safari }), /WebKit/);
});
