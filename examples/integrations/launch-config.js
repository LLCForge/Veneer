'use strict';

// Print launch configuration for a profile: Chromium switches, or a Firefox user.js.
//
//   node launch-config.js chromium --os windows --locale tr-TR
//   node launch-config.js firefox  --os linux  --locale de-DE > /path/to/firefox-profile/user.js
const veneer = require('@llcforge/veneer');

const FLAGS = new Set(['os', 'browser', 'kind', 'device', 'locale', 'timezone', 'country', 'seed']);

function parse(argv) {
  const [target, ...rest] = argv;
  if (target !== 'chromium' && target !== 'firefox') throw new Error('first argument: chromium or firefox');
  const constraints = { engine: target === 'chromium' ? 'chromium' : 'gecko' };
  for (let i = 0; i < rest.length; i += 2) {
    const name = String(rest[i]).replace(/^--/, '');
    const value = rest[i + 1];
    if (!FLAGS.has(name) || typeof value !== 'string' || value.length === 0 || value.length > 128) {
      throw new Error(`bad flag ${String(rest[i])}`);
    }
    constraints[name] = value;
  }
  return { target, constraints };
}

function render(target, profile) {
  if (target === 'firefox') return `// Veneer seed ${JSON.stringify(profile.seed)}; start Firefox with TZ=${profile.timezone}\n${veneer.firefoxUserJs(profile)}`;
  // One switch per line; pass them as an argument array, never through a shell string.
  return `${veneer.chromiumArgs(profile).join('\n')}\n`;
}

if (require.main === module) {
  try {
    const { target, constraints } = parse(process.argv.slice(2));
    process.stdout.write(render(target, veneer.generate(constraints)));
  } catch (err) {
    console.error(String(err && err.message ? err.message : err));
    process.exitCode = 1;
  }
}

module.exports = { parse, render };
