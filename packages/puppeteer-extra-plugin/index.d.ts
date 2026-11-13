import type { Constraints, Profile } from '@llcforge/veneer';
import { PuppeteerExtraPlugin } from 'puppeteer-extra-plugin';

declare function veneerPlugin(opts?: veneerPlugin.VeneerPluginOptions): veneerPlugin.VeneerPlugin;

declare namespace veneerPlugin {
  interface VeneerPluginOptions {
    /** Use this profile (must be a Chromium profile). */
    profile?: Profile;
    /** Otherwise generate one with these constraints (engine is forced to chromium). */
    constraints?: Constraints;
    /** Add `--lang`, `--user-agent`, `--window-size`, `--force-device-scale-factor` on launch (default true). */
    launchArgs?: boolean;
  }

  class VeneerPlugin extends PuppeteerExtraPlugin {
    constructor(opts?: VeneerPluginOptions);
    readonly profile: Profile;
  }
}

export = veneerPlugin;
