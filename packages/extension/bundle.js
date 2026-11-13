#!/usr/bin/env node
'use strict';

// Prints a profile bundle for the extension popup: node bundle.js --os macos --locale tr-TR
const veneer = require('@llcforge/veneer');

const FLAGS = new Set(['os', 'browser', 'kind', 'device', 'locale', 'timezone', 'seed', 'country']);
const MAX_VALUE = 128;

function parseArgs(argv) {
  const constraints = {};
  for (let i = 0; i < argv.length; i += 2) {
    const flag = argv[i];
    const value = argv[i + 1];
    const name = typeof flag === 'string' && flag.startsWith('--') ? flag.slice(2) : '';
    if (!FLAGS.has(name)) throw new Error(`unknown flag ${String(flag)}; use --${[...FLAGS].join(', --')}`);
    if (typeof value !== 'string' || value.length === 0 || value.length > MAX_VALUE) {
      throw new Error(`--${name} needs a value of 1-${MAX_VALUE} characters`);
    }
    constraints[name] = value;
  }
  // chrome.debugger drives a Chromium browser, so only Chromium profiles make sense.
  return { ...constraints, engine: 'chromium' };
}

function bundle(constraints) {
  const profile = veneer.generate(constraints);
  return {
    profile: { seed: profile.seed, device: profile.device.id, browser: `${profile.browser.id} ${profile.browser.version}` },
    commands: veneer.cdpCommands(profile),
  };
}

if (require.main === module) {
  try {
    process.stdout.write(`${JSON.stringify(bundle(parseArgs(process.argv.slice(2))))}\n`);
  } catch (err) {
    process.stderr.write(`${String(err && err.message ? err.message : err)}\n`);
    process.exitCode = 1;
  }
}

module.exports = { parseArgs, bundle };
