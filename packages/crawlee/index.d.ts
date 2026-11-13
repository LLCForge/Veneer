import type { Constraints, Profile } from '@llcforge/veneer';

export interface VeneerCrawleeOptions {
  /** Constraints for every generated profile; the engine follows the browser plugin. */
  constraints?: Constraints;
  /** Seed prefix; the seed is `<prefix>:<session id or launch id>`. Default `crawlee`. */
  seedPrefix?: string;
  /** Called with each new profile, e.g. to log `profile.seed`. */
  onProfile?: (profile: Profile, launchContext: unknown) => void;
}

export interface LaunchContextLike {
  id?: string;
  session?: { id: string };
  launchOptions?: Record<string, unknown>;
}

export interface VeneerBrowserPoolOptions {
  useFingerprints: false;
  preLaunchHooks: Array<(pageId: string, launchContext: LaunchContextLike) => void>;
  postPageCreateHooks: Array<(page: unknown, browserController: { launchContext: unknown }) => Promise<void>>;
  /** Profile of the browser that owns a page's controller. */
  profileOf(browserController: { launchContext: unknown }): Profile | undefined;
}

/** Spread into `browserPoolOptions` of PlaywrightCrawler / PuppeteerCrawler, or into `new BrowserPool()`. */
export function veneerBrowserPoolOptions(options?: VeneerCrawleeOptions): VeneerBrowserPoolOptions;
