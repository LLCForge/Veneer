import type { Constraints, Profile } from '@llcforge/veneer';

export interface SetupVeneerOptions {
  /** Constraints for the run's profile (engine defaults to chromium; use `{ engine: 'gecko' }` for Firefox). */
  constraints?: Constraints & { engine?: 'chromium' | 'gecko' };
  /** Use this profile instead of generating one. */
  profile?: Profile;
}

type On = (event: 'before:browser:launch', handler: (browser: { family: string; name: string }, launchOptions: { args: string[]; preferences: Record<string, unknown>; env: Record<string, unknown> }) => unknown) => void;

/** Call from `setupNodeEvents` and return the result as the config. */
export function setupVeneer<C extends { env?: Record<string, unknown> }>(on: On, config: C, options?: SetupVeneerOptions): C & {
  userAgent: string;
  viewportWidth: number;
  viewportHeight: number;
};
