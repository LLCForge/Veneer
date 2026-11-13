'use strict';

// Unit tests with a stand-in webContents; Electron itself is not launched here.
const test = require('node:test');
const assert = require('node:assert/strict');
const veneer = require('@llcforge/veneer');
const el = require('..');

function fakeWebContents(failOn) {
  const state = { attached: false, sent: [] };
  return {
    state,
    debugger: {
      attach: (v) => {
        assert.equal(v, '1.3');
        state.attached = true;
      },
      detach: () => {
        state.attached = false;
      },
      isAttached: () => state.attached,
      sendCommand: async (m, p) => {
        if (m === failOn) throw new Error('boom');
        state.sent.push([m, p]);
      },
    },
  };
}

test('attaches, sends every command, detaches on demand', async () => {
  const profile = veneer.generate({ os: 'windows', browser: 'chrome', seed: 'el-1' });
  const wc = fakeWebContents();
  const detach = await el.applyToWebContents(wc, profile);
  assert.equal(wc.state.attached, true);
  assert.deepEqual(wc.state.sent.map(([m]) => m), veneer.cdpCommands(profile).map((c) => c.method));
  detach();
  assert.equal(wc.state.attached, false);
});

test('detaches again when a command fails', async () => {
  const profile = veneer.generate({ os: 'windows', browser: 'chrome', seed: 'el-2' });
  const wc = fakeWebContents('Emulation.setLocaleOverride');
  await assert.rejects(el.applyToWebContents(wc, profile), /boom/);
  assert.equal(wc.state.attached, false);
});

test('session user agent and command-line switches', async () => {
  const profile = veneer.generate({ os: 'macos', browser: 'chrome', locale: 'tr-TR', seed: 'el-3' });
  let args;
  el.applyToElectronSession({ setUserAgent: (...a) => (args = a) }, profile);
  assert.deepEqual(args, [profile.navigator.userAgent, profile.navigator.languages.join(',')]);
  const switches = el.commandLineSwitches(profile);
  assert.deepEqual(switches[0], ['lang', profile.locale]);
  assert.ok(switches.some(([n, v]) => n === 'user-agent' && v === profile.navigator.userAgent));
  await assert.rejects(el.applyToWebContents({}, profile), /webContents/);
});
