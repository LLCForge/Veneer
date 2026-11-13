import type { Profile } from '@llcforge/veneer';

export interface ChromiumOptionsLike {
  addArguments(...args: string[]): unknown;
}
export interface FirefoxOptionsLike {
  setPreference(name: string, value: string | number | boolean): unknown;
  windowSize?(size: { width: number; height: number }): unknown;
}
export interface DevToolsDriver {
  sendDevToolsCommand(method: string, params?: Record<string, unknown>): Promise<unknown>;
}
export interface ScriptManagerLike {
  addPreloadScript(functionDeclaration: string): Promise<string>;
}

/** Add the profile's Chromium switches to `chrome.Options` / `edge.Options`. */
export function chromiumOptions<T extends ChromiumOptionsLike>(profile: Profile, options: T): T;
/** Set the profile's Firefox preferences (and window size) on `firefox.Options`. */
export function firefoxOptions<T extends FirefoxOptionsLike>(profile: Profile, options: T): T;
/** Send `veneer.cdpCommands(profile)` through ChromeDriver's DevTools endpoint (current tab). */
export function applyCdp(driver: DevToolsDriver, profile: Profile): Promise<void>;
/** Register the init script as a WebDriver BiDi preload script; returns its id. */
export function applyBidiPreloadScript(
  driver: unknown,
  profile: Profile,
  options?: { scriptManager?: (contexts: string[], driver: unknown) => Promise<ScriptManagerLike> },
): Promise<string>;
/** CDP for Chromium profiles, BiDi preload script for Gecko profiles. */
export function applyProfile(driver: unknown, profile: Profile): Promise<void>;
