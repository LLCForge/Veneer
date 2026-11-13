import type { Constraints, Engine, Profile } from '@llcforge/veneer';
import type { TestType, PlaywrightTestArgs, PlaywrightTestOptions, PlaywrightWorkerArgs, PlaywrightWorkerOptions, Expect } from '@playwright/test';

export type VeneerOption = Constraints & { engine?: Engine };

export interface VeneerTestOptions {
  /** Constraints for `veneer.generate`; leave undefined to run without a profile. */
  veneer: VeneerOption | undefined;
}

export interface VeneerTestFixtures {
  /** The profile applied to this test's context (stable per test title), or undefined. */
  veneerProfile: Profile | undefined;
}

export const test: TestType<
  PlaywrightTestArgs & PlaywrightTestOptions & VeneerTestOptions & VeneerTestFixtures,
  PlaywrightWorkerArgs & PlaywrightWorkerOptions
>;
export const expect: Expect;
export function constraintsFor(options: VeneerOption, browserName: string, seed: string): VeneerOption;
