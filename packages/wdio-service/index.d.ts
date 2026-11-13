import type { Constraints, Profile } from '@llcforge/veneer';

declare namespace VeneerService {
  interface Options {
    /** Constraints for the worker's profile; the engine follows `browserName`. */
    constraints?: Constraints;
    /** Use this profile instead of generating one. */
    profile?: Profile;
  }
}

declare class VeneerService {
  constructor(options?: VeneerService.Options);
  readonly options: VeneerService.Options;
  /** The profile of this worker session (set in `beforeSession`). */
  profile: Profile | undefined;
  beforeSession(config: unknown, capabilities: Record<string, unknown>): void;
  before(capabilities: unknown, specs: string[], browser: unknown): Promise<void>;
}

export = VeneerService;
