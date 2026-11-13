'use strict';

const { PuppeteerExtraPlugin } = require('puppeteer-extra-plugin');
const veneer = require('@llcforge/veneer');

/**
 * puppeteer-extra plugin: one Veneer profile per plugin instance (one browser), applied to every page
 * before its first navigation. Pass `{ profile }` or `{ constraints }`.
 */
class VeneerPlugin extends PuppeteerExtraPlugin {
  constructor(opts = {}) {
    super(opts);
    this._applied = new WeakMap();
    const { profile, constraints = {} } = this.opts;
    this.profile = profile || veneer.generate({ ...constraints, engine: 'chromium' });
    if (this.profile.browser.engine !== 'chromium') {
      // Puppeteer's Firefox support has no CDP; a Gecko profile cannot be applied coherently here.
      throw new TypeError('puppeteer-extra-plugin-veneer needs a chromium profile');
    }
  }

  get name() {
    return 'veneer';
  }

  get defaults() {
    return { profile: undefined, constraints: {}, launchArgs: true };
  }

  async beforeLaunch(options) {
    if (this.opts.launchArgs) {
      const args = Array.isArray(options.args) ? options.args : [];
      // Caller-provided switches of the same name win.
      const taken = new Set(args.map((a) => String(a).split('=')[0]));
      options.args = [...args, ...veneer.chromiumArgs(this.profile).filter((a) => !taken.has(a.split('=')[0]))];
    }
    return options;
  }

  /** Applies the profile once per page, whichever of `newPage` or the target event sees the page first. */
  apply(page) {
    let done = this._applied.get(page);
    if (!done) {
      done = page
        .createCDPSession()
        .then((session) => veneer.applyCdpCommands((method, params) => session.send(method, params), this.profile))
        .catch((err) => {
          // A page that closed during setup is not an error; anything else is.
          if (!page.isClosed()) throw err;
        });
      this._applied.set(page, done);
    }
    return done;
  }

  async onBrowser(browser) {
    // `browser.newPage()` can resolve before the target event reaches plugins, so make it wait for the profile.
    const newPage = browser.newPage.bind(browser);
    browser.newPage = async (...args) => {
      const page = await newPage(...args);
      await this.apply(page);
      return page;
    };
  }

  async onPageCreated(page) {
    await this.apply(page);
  }
}

module.exports = (opts) => new VeneerPlugin(opts);
module.exports.VeneerPlugin = VeneerPlugin;
