'use strict';

const veneer = require('@llcforge/veneer');

const FAMILY_OF = { chromium: 'chromium', gecko: 'firefox' };
// Commands the support file sends over Cypress's CDP bridge; viewport stays with Cypress, the init script
// is evaluated in the app window only (not in the Cypress runner).
const SUPPORT_CDP = new Set(['Emulation.setUserAgentOverride', 'Emulation.setTimezoneOverride', 'Emulation.setLocaleOverride']);

/**
 * Call from `setupNodeEvents(on, config)` and return its result. Generates (or takes) one profile for the run,
 * sets `userAgent` and viewport in the config, passes the init script to `support.js` and adjusts browser launch.
 */
function setupVeneer(on, config, options = {}) {
  if (typeof on !== 'function' || !config || typeof config !== 'object') {
    throw new TypeError('setupVeneer(on, config, options) must be called from setupNodeEvents');
  }
  const { constraints = {}, profile: given } = options;
  const profile = given || veneer.generate({ engine: 'chromium', ...constraints });
  const family = FAMILY_OF[profile.browser.engine];
  if (!family) throw new TypeError('Cypress runs Chromium-family browsers and Firefox; this profile needs WebKit');

  on('before:browser:launch', (browser, launchOptions) => {
    if (browser.family !== family) {
      throw new Error(`Veneer profile is for ${family}, but Cypress is launching ${browser.family} (${browser.name})`);
    }
    // Timezone for the whole browser process; CDP refines it on Chromium.
    launchOptions.env = { ...launchOptions.env, TZ: profile.timezone };
    if (family === 'chromium') launchOptions.args.push(...veneer.chromiumArgs(profile));
    else Object.assign(launchOptions.preferences, veneer.firefoxPrefs(profile));
    return launchOptions;
  });

  return {
    ...config,
    userAgent: profile.navigator.userAgent,
    viewportWidth: profile.viewport.width,
    viewportHeight: profile.viewport.height,
    env: {
      ...config.env,
      veneerProfile: profile,
      veneerInitScript: veneer.toInitScript(profile),
      veneerCdp: family === 'chromium' ? veneer.cdpCommands(profile).filter((c) => SUPPORT_CDP.has(c.method)) : [],
    },
  };
}

module.exports = { setupVeneer };
