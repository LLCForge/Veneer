export type Engine = 'chromium' | 'gecko' | 'webkit';
export type Kind = 'desktop' | 'laptop' | 'phone' | 'tablet' | 'console' | 'tv' | 'vr' | 'handheld';

export interface Constraints {
  /** `windows`, `macos`, `linux`, `chromeos`, `android`, `ios`, `ipados`, `xbox`, `playstation`, `nintendo`, `tizen`, `webos`, `horizonos` */
  os?: string;
  /** `chrome`, `edge`, `firefox`, `safari`, `samsung`, `playstation`, `playstation4`, `nintendo`, `samsung-tv`, `lg-tv`, `meta-quest` */
  browser?: string;
  kind?: Kind;
  /** Exact device id, for example `iphone-15` or `mac-macbook-air-m2`. */
  device?: string;
  /** `tr-TR` or a bare language such as `tr`. */
  locale?: string;
  /** IANA timezone such as `Europe/Istanbul`. */
  timezone?: string;
  /** Same seed and constraints always give the same profile. */
  seed?: string;
  /** Screen width range in CSS pixels. */
  minWidth?: number;
  maxWidth?: number;
  /** Screen height range in CSS pixels. */
  minHeight?: number;
  maxHeight?: number;
  /** devicePixelRatio range, each in (0, 10]. */
  minDpr?: number;
  maxDpr?: number;
}

export interface Brand {
  brand: string;
  version: string;
}

export interface UserAgentData {
  brands: Brand[];
  fullVersionList: Brand[];
  mobile: boolean;
  platform: string;
  platformVersion: string;
  architecture: string;
  bitness: string;
  model: string;
  uaFullVersion: string;
  formFactors: string[];
  wow64: boolean;
}

export interface Profile {
  seed: string;
  device: { id: string; kind: Kind; model?: string };
  os: { id: string; name: string; version: string };
  browser: { id: string; name: string; engine: Engine; version: string; major: number };
  navigator: {
    userAgent: string;
    appVersion: string;
    userAgentData?: UserAgentData;
    platform: string;
    vendor: string;
    language: string;
    languages: string[];
    hardwareConcurrency: number;
    deviceMemory?: number;
    maxTouchPoints: number;
    oscpu?: string;
  };
  screen: {
    width: number;
    height: number;
    availWidth: number;
    availHeight: number;
    colorDepth: number;
    pixelDepth: number;
    devicePixelRatio: number;
  };
  viewport: { width: number; height: number };
  webgl: { vendor: string; renderer: string; unmaskedVendor?: string; unmaskedRenderer?: string };
  canvas: { seed: number };
  audio: { seed: number };
  fonts: string[];
  locale: string;
  timezone: string;
  /** WebGL capabilities, WebGPU availability and media-query values (extension module `graphics`). */
  graphics?: GraphicsProfile;
  /** Navigator/window values derived from device, OS, browser and locale (extension module `navapi`). */
  navapi?: NavApiProfile;
}

/** Fields are absent when no verified data exists; the init script then leaves the browser's own value. */
export interface NavApiProfile {
  /** `MediaDeviceInfo.kind` per `enumerateDevices()` entry before any permission (ids and labels empty). */
  mediaDevices?: string[];
  /** `navigator.pdfViewerEnabled`; `true` also means the standard five-entry `navigator.plugins`. */
  pdfViewerEnabled?: boolean;
  connection?: { effectiveType: string; rtt: number; downlink: number; saveData: boolean };
  /** `null` times mean `Infinity`. */
  battery?: { charging: boolean; chargingTime: number | null; dischargingTime: number | null; level: number };
  storageQuota?: number;
  jsHeapSizeLimit?: number;
  /** `AudioContext.baseLatency`: fixed seconds, or frames divided by the context's sample rate. */
  audio?: { baseLatency?: number; baseLatencyFrames?: number };
  /** `navigator.keyboard.getLayoutMap()` entries (`KeyboardEvent.code` to unshifted character). */
  keyboardLayout?: Record<string, string>;
  voices?: { name: string; lang: string; localService: boolean; default: boolean }[];
  orientation?: { type: string; angle: number };
  /** Maximized window geometry (`screenLeft`/`screenTop` equal `screenX`/`screenY`). */
  window?: { outerWidth: number; outerHeight: number; screenX: number; screenY: number };
  notificationPermission?: string;
  /** APIs the browser does not expose, e.g. `getBattery`, `hid`, `Notification`, `window.orientation`. */
  absent?: string[];
}

export interface Issue {
  rule: string;
  severity: 'error' | 'warning';
  message: string;
}

export interface Report {
  valid: boolean;
  issues: Issue[];
}

export interface ContextOptions {
  userAgent: string;
  locale: string;
  timezoneId: string;
  viewport: { width: number; height: number };
  screen: { width: number; height: number };
  deviceScaleFactor: number;
  isMobile?: boolean;
  hasTouch?: boolean;
}

export interface UserAgentMetadata {
  brands: Brand[];
  fullVersionList: Brand[];
  fullVersion: string;
  platform: string;
  platformVersion: string;
  architecture: string;
  model: string;
  mobile: boolean;
  bitness: string;
  wow64: boolean;
  formFactors: string[];
}

export function generate(constraints?: Constraints): Profile;
export function validate(profile: Profile): Report;
export function toInitScript(profile: Profile): string;
export function playwrightEngine(profile: Profile): 'chromium' | 'firefox' | 'webkit';
export function contextOptions(profile: Profile): ContextOptions;
export function userAgentMetadata(profile: Profile): UserAgentMetadata | undefined;

export interface CdpCommand {
  method: string;
  params: Record<string, unknown>;
}
export type FirefoxPrefs = Record<string, string | number | boolean>;
/** Runtime helpers (exported by index.js) that were not typed above. */
export function catalog(): unknown;
export function navigationHeaders(profile: Profile): { name: string; value: string }[];
export function collectorScript(): string;
export function compareObserved(profile: Profile, observed: unknown): Report | Promise<Report>;
export function userAgentOverride(profile: Profile): {
  userAgent: string;
  acceptLanguage: string;
  platform: string;
  userAgentMetadata?: UserAgentMetadata;
};
/** Ordered CDP commands that apply a Chromium profile to one target. Throws for non-Chromium profiles. */
export function cdpCommands(profile: Profile): CdpCommand[];
/** Sends `cdpCommands(profile)` through `send`, one after another. */
export function applyCdpCommands(send: (method: string, params: Record<string, unknown>) => Promise<unknown>, profile: Profile): Promise<void>;
/** Chromium switches that match the profile (`--lang`, `--user-agent`, `--window-size`, `--force-device-scale-factor`). */
export function chromiumArgs(profile: Profile): string[];
/** Firefox preferences that match a Gecko profile. Throws for non-Gecko profiles. */
export function firefoxPrefs(profile: Profile): FirefoxPrefs;
/** `user.js` content for a Firefox profile directory. */
export function firefoxUserJs(profile: Profile): string;

export interface NetworkCheck {
  report: Report;
  notes: string[];
}
export type WebRtcPolicy = 'public-interface-only' | 'disable-non-proxied-udp' | 'disabled';
export interface WebRtcSettings {
  engine: 'chromium' | 'gecko' | 'webkit';
  policy: WebRtcPolicy;
  /** Chromium command-line switches. */
  args: string[];
  /** Chromium `Preferences` entries or Firefox prefs. */
  prefs: Record<string, unknown>;
  /** Pass to Playwright `browserType.launch()` (merge with your own options). */
  playwrightLaunchOptions: { args?: string[]; firefoxUserPrefs?: FirefoxPrefs };
  notes: string[];
}
export type CapturedHeaders =
  | { name: string; value: string }[]
  | [string, string][]
  | Record<string, string>;
/** Compare captured request headers with the profile; the order check needs an array. */
export function compareHeaders(profile: Profile, headers: CapturedHeaders): Report;
/** Informational: does a captured JA4 fit the profile's browser family? Veneer cannot change TLS. */
export function checkJa4(profile: Profile, ja4: string): NetworkCheck;
/** Proxy country/timezone (caller-supplied) against the profile. */
export function checkLocation(profile: Profile, location: { country?: string; timezone?: string }): NetworkCheck;
export function webrtcSettings(engine: 'chromium' | 'gecko' | 'webkit', policy?: WebRtcPolicy): WebRtcSettings;

/** Playwright: launch the engine that matches the profile. Pass Playwright's types as type arguments. */
export function launchWithProfile<B = unknown, C = unknown, P = unknown>(
  playwright: unknown,
  profile: Profile,
  launchOptions?: Record<string, unknown>,
): Promise<{ browser: B; context: C; page: P }>;
/** Run the collector in a live page (Playwright or Puppeteer) and compare it with the profile. */
export function verifyPage(page: unknown, profile: Profile): Promise<Report>;
/** Puppeteer: apply the profile to a page before its first navigation. */
export function applyProfile(page: unknown, profile: Profile): Promise<void>;

/** WebGL values for GPU classes with verified data (currently Chromium on Windows, ANGLE Direct3D 11). */
export interface WebGlCaps {
  /** `getParameter` values keyed by GL constant name; arrays for ranges and viewport dimensions. */
  params: Record<string, number | number[]>;
  /** `getShaderPrecisionFormat` as `[rangeMin, rangeMax, precision]`, keyed `VERTEX_SHADER.HIGH_FLOAT` etc. */
  precision: Record<string, [number, number, number]>;
  extensions: { webgl: string[]; webgl2: string[] };
}
export interface WebGpuInfo {
  present: boolean;
  /** `GPUAdapterInfo.vendor` (Chromium only). */
  vendor?: string;
  /** `GPUAdapterInfo.architecture` (Chromium only, when the PCI device id is known). */
  architecture?: string;
}
export interface MediaValues {
  pointer: 'fine' | 'coarse';
  hover: 'hover' | 'none';
  colorScheme: 'light' | 'dark';
  resolution: number;
  deviceWidth: number;
  deviceHeight: number;
  colorGamut?: 'srgb' | 'p3';
}
export interface GraphicsProfile {
  webgl?: WebGlCaps;
  webgpu?: WebGpuInfo;
  media?: MediaValues;
}

export interface GeneratedProfile {
  /** Short URL-safe id that regenerates this profile with `generateFromId`. */
  id: string;
  profile: Profile;
}
/** One extra dataset file: `os/<id>.json`, `browsers/<id>.json`, `devices/<family>.json` or `locales.json`. */
export interface DataFile {
  path: string;
  text: string;
}
/**
 * `count` (1-10000) profiles; profile i gets seed `<seed>-<i>`. With `unique`, no two share a
 * (device, browser, version, screen, locale); throws when fewer such configurations exist.
 */
export function generateBatch(constraints?: Constraints, count?: number, unique?: boolean): GeneratedProfile[];
/** Regenerate the exact profile behind an id (same dataset version). */
export function generateFromId(id: string): Profile;
/** Id for constraints that carry the profile's seed. */
export function profileId(constraints: Constraints): string;
/** Merge extra dataset files; they must pass the dataset integrity checks or nothing changes. */
export function extendDataset(files: DataFile[]): void;

/** Machine-readable repair attached to every issue. */
export interface Fix {
  /** JSON pointer into the profile (`/headers/<name>` for header rules, the collector output for `live.*`). */
  path: string;
  action: 'set' | 'merge' | 'remove' | 'review';
  expected?: unknown;
}
export interface Issue {
  /** What to do about it. */
  hint?: string;
  fix?: Fix;
}
export interface Report {
  /** 100 minus 25 per error and 5 per warning. */
  score: number;
  /** Version of the rule set that produced the report. */
  rulesVersion: string;
}
export type ImportFormat = 'auto' | 'veneer' | 'collector' | 'fingerprint-suite' | 'flat';
export interface Header {
  name: string;
  value: string;
}
export type ConditionOp =
  | 'eq' | 'ne' | 'in' | 'notIn' | 'lt' | 'lte' | 'gt' | 'gte'
  | 'exists' | 'absent' | 'contains' | 'startsWith' | 'endsWith';
export type Condition =
  | { all: Condition[] }
  | { any: Condition[] }
  | { not: Condition }
  | { path: string; op: ConditionOp; value?: unknown };
/** Rule declared in JSON: fires when `when` holds (or is absent) and `require` does not. */
export interface CustomRule {
  /** Must start with `custom.`. */
  id: string;
  severity: 'error' | 'warning';
  message: string;
  hint?: string;
  when?: Condition;
  require: Condition;
}
export interface ValidateOptions {
  /** Rule ids to skip; `fonts.*` skips a group. */
  disable?: string[];
  rules?: CustomRule[];
  /** Request headers the browser sent, in order. */
  headers?: Header[];
  /** Connection location you know (e.g. from your proxy). Never looked up. */
  geo?: { country?: string; timezone?: string };
  /** `YYYY-MM-DD`; enables the browser release-age rule. */
  today?: string;
  /** Default 180. */
  maxAgeDays?: number;
}
export interface Imported {
  profile: Profile;
  format: Exclude<ImportFormat, 'auto'>;
  /** Profile fields the input did not contain (derived or left empty). */
  missing: string[];
  headers: Header[];
}
export interface DriftBucket {
  value: string;
  expected: number;
  observed: number;
  delta: number;
}
export interface DriftDimension {
  name: 'os' | 'browser' | 'engine' | 'kind' | 'locale';
  totalVariation: number;
  noiseFloor: number;
  drifted: boolean;
  buckets: DriftBucket[];
}
export interface DriftReport {
  count: number;
  dimensions: DriftDimension[];
}
export interface RuleInfo {
  id: string;
  severity: 'error' | 'warning';
  fields: string[];
  needs: 'profile' | 'dataset' | 'headers' | 'geo' | 'today' | 'observed' | 'network' | 'custom';
  hint: string;
}
export interface Coverage {
  rulesVersion: string;
  rules: RuleInfo[];
  fields: Record<string, string[]>;
  unprotected: string[];
}
export function validateWith(profile: Profile, options?: ValidateOptions): Report;
export function capture(observed: Record<string, unknown>, headers?: Header[]): Imported;
export function importProfile(input: unknown, format?: ImportFormat): Imported;
export function toSarif(reports: { artifact: string; report: Report }[], options?: ValidateOptions): Record<string, unknown>;
export function drift(profiles: Profile[]): DriftReport;
export function ruleCoverage(options?: ValidateOptions): Coverage;
export function headersFromHar(har: unknown): Header[];

/** Playwright: a context configured for the profile; on Chromium every page also gets matching request headers. */
export function newContext<B = unknown, C = unknown>(browser: B, profile: Profile, options?: Record<string, unknown>): Promise<C>;
/** Playwright: a page whose request headers are overridden before its first navigation. */
export function newPage<C = unknown, P = unknown>(context: C, profile: Profile): Promise<P>;
