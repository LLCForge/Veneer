'use strict';

const wasm = require('./wasm/veneer_wasm.js');

const PLAYWRIGHT_ENGINES = { chromium: 'chromium', gecko: 'firefox', webkit: 'webkit' };

/** Generate a coherent profile. Without a seed, a random one is chosen and returned in `profile.seed`. */
function generate(constraints = {}) {
  if (constraints === null || typeof constraints !== 'object' || Array.isArray(constraints)) {
    throw new TypeError('constraints must be an object');
  }
  const seeded = constraints.seed == null ? { ...constraints, seed: globalThis.crypto.randomUUID() } : constraints;
  return wasm.generate(seeded);
}

/** Check any profile against every consistency rule. */
function validate(profile) {
  return wasm.validate(profile);
}

/** Init script for Playwright `addInitScript` or Puppeteer `evaluateOnNewDocument`. */
function toInitScript(profile) {
  return wasm.toInitScript(profile);
}

/** Every OS, browser, device and locale the generator knows. */
function catalog() {
  return wasm.catalog();
}

/** Navigation request headers in the order the profiled browser sends them. */
function navigationHeaders(profile) {
  return wasm.navigationHeaders(profile);
}

/** Expression for `page.evaluate` that resolves to what the page can observe. */
function collectorScript() {
  return wasm.collectorScript();
}

/** Compare collector output from a real browser against the profile. */
function compareObserved(profile, observed) {
  return wasm.compareObserved(profile, observed);
}

/** Playwright browser type whose engine matches the profile: `chromium`, `firefox` or `webkit`. */
function playwrightEngine(profile) {
  return PLAYWRIGHT_ENGINES[profile.browser.engine];
}

/** Options for Playwright `browser.newContext()`. */
function contextOptions(profile) {
  const options = {
    userAgent: profile.navigator.userAgent,
    locale: profile.locale,
    timezoneId: profile.timezone,
    viewport: { ...profile.viewport },
    screen: { width: profile.screen.width, height: profile.screen.height },
    deviceScaleFactor: profile.screen.devicePixelRatio,
  };
  const media = profile.graphics && profile.graphics.media;
  if (media && media.colorScheme) options.colorScheme = media.colorScheme;
  const handheld = profile.device.kind === 'phone' || profile.device.kind === 'tablet';
  // Playwright rejects isMobile on Firefox.
  if (handheld && profile.browser.engine !== 'gecko') {
    options.isMobile = true;
    options.hasTouch = true;
  }
  return options;
}

/** CDP `UserAgentMetadata` so Sec-CH-UA headers match the profile, or `undefined` for browsers without client hints. */
function userAgentMetadata(profile) {
  const u = profile.navigator.userAgentData;
  if (!u) return undefined;
  return {
    brands: u.brands.map((b) => ({ ...b })),
    fullVersionList: u.fullVersionList.map((b) => ({ ...b })),
    fullVersion: u.uaFullVersion,
    platform: u.platform,
    platformVersion: u.platformVersion,
    architecture: u.architecture,
    model: u.model,
    mobile: u.mobile,
    bitness: u.bitness,
    wow64: u.wow64,
    formFactors: [...u.formFactors],
  };
}

/** CDP `Emulation.setUserAgentOverride` parameters: user agent, Accept-Language, platform and client hints. */
function userAgentOverride(profile) {
  return {
    userAgent: profile.navigator.userAgent,
    // Chromium adds the q-values itself; passing a finished header would double them.
    acceptLanguage: profile.navigator.languages.join(','),
    platform: profile.navigator.platform,
    userAgentMetadata: userAgentMetadata(profile),
  };
}

/** CSS media features only emulation can set (color-gamut has no Playwright option). */
function emulatedMediaFeatures(profile) {
  const media = profile.graphics && profile.graphics.media;
  const features = [];
  if (media && media.colorGamut) features.push({ name: 'color-gamut', value: media.colorGamut });
  if (media && media.colorScheme) features.push({ name: 'prefers-color-scheme', value: media.colorScheme });
  return features;
}

async function applyCdp(session, profile) {
  await session.send('Emulation.setUserAgentOverride', userAgentOverride(profile));
  const features = emulatedMediaFeatures(profile);
  if (features.length) await session.send('Emulation.setEmulatedMedia', { features });
}

/**
 * Playwright: a context configured for the profile. On Chromium, every page (including popups)
 * also gets matching request headers through CDP.
 */
async function newContext(browser, profile, options = {}) {
  const context = await browser.newContext({ ...contextOptions(profile), ...options });
  await context.addInitScript(toInitScript(profile));
  if (browser.browserType().name() === 'chromium') {
    context.on('page', (page) => {
      context
        .newCDPSession(page)
        .then((session) => applyCdp(session, profile))
        .catch(() => {
          // Page closed before the override could be applied.
        });
    });
  }
  return context;
}

/** Playwright: a page whose request headers are already overridden before its first navigation. */
async function newPage(context, profile) {
  const page = await context.newPage();
  if (context.browser() && context.browser().browserType().name() === 'chromium') {
    await applyCdp(await context.newCDPSession(page), profile);
  }
  return page;
}

/** Playwright: launch the engine that matches the profile and return `{ browser, context, page }`. */
async function launchWithProfile(playwright, profile, launchOptions = {}) {
  const browserType = playwright[playwrightEngine(profile)];
  if (!browserType) throw new Error(`playwright has no ${playwrightEngine(profile)} browser type`);
  const browser = await browserType.launch(launchOptions);
  try {
    const context = await newContext(browser, profile);
    const page = await newPage(context, profile);
    return { browser, context, page };
  } catch (err) {
    await browser.close();
    throw err;
  }
}

/** Puppeteer: apply the profile to a page before its first navigation. */
async function applyProfile(page, profile) {
  const session = await page.createCDPSession();
  await applyCdp(session, profile);
  await page.emulateTimezone(profile.timezone);
  await page.setViewport({
    ...profile.viewport,
    deviceScaleFactor: profile.screen.devicePixelRatio,
    isMobile: profile.device.kind === 'phone' || profile.device.kind === 'tablet',
    hasTouch: profile.navigator.maxTouchPoints > 0,
  });
  await page.evaluateOnNewDocument(toInitScript(profile));
}

/** Run the collector in a live page (Playwright or Puppeteer) and compare it with the profile. */
async function verifyPage(page, profile) {
  const observed = await page.evaluate(collectorScript());
  return compareObserved(profile, observed);
}

module.exports = {
  generate,
  validate,
  toInitScript,
  catalog,
  navigationHeaders,
  collectorScript,
  compareObserved,
  playwrightEngine,
  contextOptions,
  userAgentMetadata,
  userAgentOverride,
  newContext,
  newPage,
  launchWithProfile,
  applyProfile,
  verifyPage,
};

// --- Integration helpers (framework-free; used by packages/cdp, selenium, extension, electron, …) ---

function requireEngine(profile, engine, caller) {
  const actual = profile && profile.browser ? profile.browser.engine : undefined;
  if (actual !== engine) {
    // A Chromium browser cannot coherently present a Gecko/WebKit profile (and vice versa), so fail closed.
    throw new TypeError(`${caller} needs a ${engine} profile, got ${String(actual)}`);
  }
}

/**
 * CDP commands, in order, that apply a Chromium profile to one target: user agent and client hints,
 * timezone, locale, device metrics, touch and the init script. Send them before the first navigation.
 */
function cdpCommands(profile) {
  requireEngine(profile, 'chromium', 'cdpCommands');
  const touch = profile.navigator.maxTouchPoints;
  return [
    // Chromium derives the q-weighted Accept-Language header itself; a list that already has q-values
    // would leak into navigator.languages as "de;q=0.9".
    {
      method: 'Emulation.setUserAgentOverride',
      params: { ...userAgentOverride(profile), acceptLanguage: profile.navigator.languages.join(',') },
    },
    { method: 'Emulation.setTimezoneOverride', params: { timezoneId: profile.timezone } },
    { method: 'Emulation.setLocaleOverride', params: { locale: profile.locale } },
    {
      method: 'Emulation.setDeviceMetricsOverride',
      params: {
        width: profile.viewport.width,
        height: profile.viewport.height,
        deviceScaleFactor: profile.screen.devicePixelRatio,
        mobile: profile.device.kind === 'phone' || profile.device.kind === 'tablet',
        screenWidth: profile.screen.width,
        screenHeight: profile.screen.height,
      },
    },
    {
      method: 'Emulation.setTouchEmulationEnabled',
      params: touch > 0 ? { enabled: true, maxTouchPoints: Math.min(touch, 16) } : { enabled: false },
    },
    // New-document scripts only run in sessions where the Page domain is enabled.
    { method: 'Page.enable', params: {} },
    { method: 'Page.addScriptToEvaluateOnNewDocument', params: { source: toInitScript(profile) } },
  ];
}

/** Send `cdpCommands(profile)` through any `send(method, params)` function, one after another. */
async function applyCdpCommands(send, profile) {
  if (typeof send !== 'function') throw new TypeError('send must be a function');
  for (const { method, params } of cdpCommands(profile)) {
    await send(method, params);
  }
}

/**
 * Chromium command-line switches that match the profile. Nothing here hides automation:
 * `--disable-blink-features=AutomationControlled` and similar are deliberately never added.
 */
function chromiumArgs(profile) {
  requireEngine(profile, 'chromium', 'chromiumArgs');
  return [
    `--lang=${profile.locale}`,
    `--user-agent=${profile.navigator.userAgent}`,
    // A maximised window is as large as the available screen area.
    `--window-size=${profile.screen.availWidth},${profile.screen.availHeight}`,
    `--force-device-scale-factor=${profile.screen.devicePixelRatio}`,
  ];
}

/**
 * Firefox preferences that match a Gecko profile. `privacy.resistFingerprinting` is turned off because it
 * would replace these values (UTC timezone, generic user agent). Set the timezone with the `TZ` environment variable.
 */
function firefoxPrefs(profile) {
  requireEngine(profile, 'gecko', 'firefoxPrefs');
  const n = profile.navigator;
  const prefs = {
    'privacy.resistFingerprinting': false,
    'general.useragent.override': n.userAgent,
    'general.appversion.override': n.appVersion,
    'general.platform.override': n.platform,
    'intl.accept_languages': n.languages.join(', '),
    'intl.locale.requested': profile.locale,
    'dom.maxHardwareConcurrency': n.hardwareConcurrency,
    'layout.css.devPixelsPerPx': String(profile.screen.devicePixelRatio),
  };
  if (n.oscpu) prefs['general.oscpu.override'] = n.oscpu;
  return prefs;
}

/** `user.js` file content for a Firefox profile directory. Values are JSON-encoded, so they cannot break out. */
function firefoxUserJs(profile) {
  return Object.entries(firefoxPrefs(profile))
    .map(([name, value]) => `user_pref(${JSON.stringify(name)}, ${JSON.stringify(value)});`)
    .join('\n')
    .concat('\n');
}

Object.assign(module.exports, { cdpCommands, applyCdpCommands, chromiumArgs, firefoxPrefs, firefoxUserJs });

/** Compare captured request headers with the profile. Pass an object (`allHeaders()`) unless you have wire order. */
function compareHeaders(profile, headers) {
  return wasm.compareHeaders(profile, headers);
}

/** Informational check of a JA4 TLS fingerprint you captured. Veneer cannot change TLS. */
function checkJa4(profile, ja4) {
  if (typeof ja4 !== 'string') throw new TypeError('ja4 must be a string');
  return wasm.checkJa4(profile, ja4);
}

/** Check the proxy's country and/or timezone (which you supply; no IP lookups) against the profile. */
function checkLocation(profile, { country, timezone } = {}) {
  return wasm.checkLocation(profile, country, timezone);
}

/** Launch settings that limit WebRTC IP exposure for an engine (`chromium`, `gecko`, `webkit`). */
function webrtcSettings(engine, policy = 'public-interface-only') {
  return wasm.webrtcSettings(engine, policy);
}

Object.assign(module.exports, { compareHeaders, checkJa4, checkLocation, webrtcSettings });

/** `count` profiles with their ids; `unique` never repeats a (device, browser, version, screen, locale). */
function generateBatch(constraints = {}, count = 1, unique = false) {
  if (constraints === null || typeof constraints !== 'object' || Array.isArray(constraints)) {
    throw new TypeError('constraints must be an object');
  }
  if (!Number.isInteger(count) || count < 1) {
    throw new TypeError('count must be a positive integer');
  }
  const seeded = constraints.seed == null ? { ...constraints, seed: globalThis.crypto.randomUUID() } : constraints;
  return wasm.generateBatch(seeded, count, unique === true);
}

/** Regenerate the exact profile behind a profile id. */
function generateFromId(id) {
  if (typeof id !== 'string') {
    throw new TypeError('id must be a string');
  }
  return wasm.generateFromId(id);
}

/** Short shareable id for constraints that carry the profile's seed. */
function profileId(constraints) {
  return wasm.profileId(constraints);
}

/** Merge extra dataset files (`[{ path, text }]`) into the dataset. */
function extendDataset(files) {
  if (!Array.isArray(files)) {
    throw new TypeError('files must be an array of { path, text }');
  }
  return wasm.extendDataset(files);
}

Object.assign(module.exports, { generateBatch, generateFromId, profileId, extendDataset });

/** Validate with options: `{ disable, rules, headers, geo, today, maxAgeDays }`. */
function validateWith(profile, options = {}) {
  return wasm.validateWith(profile, options);
}

/** Full profile from collector output of a real browser (`page.evaluate(collectorScript())`). */
function capture(observed, headers = []) {
  if (observed === null || typeof observed !== 'object') throw new TypeError('observed must be an object');
  return wasm.capture(observed, headers);
}

/** Profile from another format: 'auto', 'veneer', 'collector', 'fingerprint-suite' or 'flat'. */
function importProfile(input, format = 'auto') {
  if (typeof format !== 'string') throw new TypeError('format must be a string');
  return wasm.importProfile(input, format);
}

/** SARIF 2.1.0 log for `[{ artifact, report }]`. */
function toSarif(reports, options = {}) {
  if (!Array.isArray(reports)) throw new TypeError('reports must be an array of { artifact, report }');
  return wasm.toSarif(reports, options);
}

/** How far a batch of profiles is from the dataset's distribution. */
function drift(profiles) {
  if (!Array.isArray(profiles)) throw new TypeError('profiles must be an array');
  return wasm.drift(profiles);
}

/** Every rule and the profile fields it protects (custom rules from `options.rules` included). */
function ruleCoverage(options = {}) {
  return wasm.ruleCoverage(options);
}

/** Request headers of the document request in a HAR log. */
function headersFromHar(har) {
  return wasm.headersFromHar(har);
}

Object.assign(module.exports, { validateWith, capture, importProfile, toSarif, drift, ruleCoverage, headersFromHar });
