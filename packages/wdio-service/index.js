'use strict';

const veneer = require('@llcforge/veneer');

const CHROMIUM_CAPS = { chrome: 'goog:chromeOptions', chromium: 'goog:chromeOptions', microsoftedge: 'ms:edgeOptions', msedge: 'ms:edgeOptions' };

function engineOf(browserName) {
  const name = String(browserName || '').toLowerCase();
  if (CHROMIUM_CAPS[name]) return 'chromium';
  if (name === 'firefox') return 'gecko';
  throw new Error(`wdio-veneer-service supports Chrome, Chromium, Edge and Firefox, not "${browserName}"`);
}

/**
 * WebdriverIO service: one Veneer profile per worker session.
 * Launch side (`beforeSession`): browser switches or Firefox prefs in the capabilities.
 * Session side (`before`): Chromium gets `veneer.cdpCommands` on every open page through `browser.getPuppeteer()`;
 * Firefox gets the init script as a WebDriver BiDi preload script.
 */
class VeneerService {
  constructor(options = {}) {
    if (options === null || typeof options !== 'object') throw new TypeError('service options must be an object');
    this.options = options;
    this.profile = undefined;
  }

  beforeSession(_config, capabilities) {
    const caps = capabilities && capabilities.alwaysMatch ? capabilities.alwaysMatch : capabilities;
    if (!caps || typeof caps !== 'object' || 'browserName' in caps === false) {
      throw new Error('wdio-veneer-service needs standalone capabilities with browserName');
    }
    const engine = engineOf(caps.browserName);
    this.profile = this.options.profile || veneer.generate({ ...this.options.constraints, engine });
    if (this.profile.browser.engine !== engine) {
      throw new Error(`Veneer profile is ${this.profile.browser.engine}, but the session browser is ${caps.browserName}`);
    }
    if (engine === 'chromium') {
      const key = CHROMIUM_CAPS[String(caps.browserName).toLowerCase()];
      const opts = caps[key] || {};
      caps[key] = { ...opts, args: [...(opts.args || []), ...veneer.chromiumArgs(this.profile)] };
    } else {
      const opts = caps['moz:firefoxOptions'] || {};
      caps['moz:firefoxOptions'] = { ...opts, prefs: { ...opts.prefs, ...veneer.firefoxPrefs(this.profile) } };
    }
  }

  async before(_capabilities, _specs, browser) {
    if (!this.profile) throw new Error('beforeSession did not run; is the service configured for this capability?');
    if (this.profile.browser.engine === 'chromium') {
      const puppeteer = await browser.getPuppeteer();
      for (const page of await puppeteer.pages()) {
        const session = await page.createCDPSession();
        await veneer.applyCdpCommands((method, params) => session.send(method, params), this.profile);
      }
      return;
    }
    if (!browser.isBidi) throw new Error('Firefox needs a WebDriver BiDi session (webSocketUrl: true) for the init script');
    await browser.scriptAddPreloadScript({ functionDeclaration: `() => {\n${veneer.toInitScript(this.profile)}\n}` });
  }
}

module.exports = VeneerService;
module.exports.default = VeneerService;
module.exports.VeneerService = VeneerService;
