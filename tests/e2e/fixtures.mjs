// Shared setup: the Veneer package under test and the profiles each engine is checked with.
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const require = createRequire(import.meta.url);
const here = path.dirname(fileURLToPath(import.meta.url));

/** `packages/veneer` (with its WASM build) unless VENEER_PACKAGE points to another build of the same API. */
export const veneer = require(process.env.VENEER_PACKAGE ?? path.join(here, '..', '..', 'packages', 'veneer'));

export const BASE = `http://127.0.0.1:${process.env.E2E_PORT ?? 8813}`;

export const FINGERPRINTJS = require.resolve('@fingerprintjs/fingerprintjs/dist/fp.min.js');

/** Profiles per Playwright engine. Seeds are fixed so a failure reproduces with `veneer generate`. */
export const CASES = {
  chromium: [
    { os: 'windows', browser: 'chrome', locale: 'de-DE' },
    { os: 'windows', browser: 'edge' },
    { os: 'windows', browser: 'brave' },
    { os: 'macos', browser: 'chrome', kind: 'laptop' },
    { os: 'linux', browser: 'chrome' },
    { os: 'android', browser: 'chrome', locale: 'tr' },
    { device: 'android-galaxy-s26-ultra', browser: 'samsung', locale: 'tr' },
    { os: 'chromeos', browser: 'chrome' },
  ],
  firefox: [
    { os: 'windows', browser: 'firefox', locale: 'tr' },
    { os: 'macos', browser: 'firefox' },
    { os: 'linux', browser: 'firefox' },
    { os: 'android', browser: 'firefox' },
  ],
  webkit: [
    { os: 'ios', browser: 'safari', locale: 'tr' },
    { os: 'ipados', browser: 'safari' },
    { os: 'macos', browser: 'safari' },
    { os: 'ios', browser: 'chrome-ios' },
    { os: 'ios', browser: 'instagram' },
  ],
};

export function casesFor(engine) {
  return CASES[engine].map((c, i) => ({ ...c, seed: `e2e-${engine}-${i}` }));
}

export const label = (c) => Object.entries(c).filter(([k]) => k !== 'seed').map(([, v]) => v).join(' ');

/** Launch the engine matching the profile and block every request that is not for the fixture server. */
export async function open(playwright, profile) {
  const session = await veneer.launchWithProfile(playwright, profile, { headless: true });
  await session.context.route((url) => url.origin !== BASE, (route) => route.abort());
  return session;
}
