'use strict';

const veneer = require('@llcforge/veneer');

function requireOptions(options, method) {
  if (!options || typeof options[method] !== 'function') {
    throw new TypeError(`options must be a selenium-webdriver Options object with ${method}()`);
  }
}

/** Add the profile's Chromium switches to `chrome.Options` or `edge.Options`. Returns the same object. */
function chromiumOptions(profile, options) {
  requireOptions(options, 'addArguments');
  options.addArguments(...veneer.chromiumArgs(profile));
  return options;
}

/** Set the profile's Firefox preferences and window size on `firefox.Options`. Returns the same object. */
function firefoxOptions(profile, options) {
  requireOptions(options, 'setPreference');
  for (const [name, value] of Object.entries(veneer.firefoxPrefs(profile))) options.setPreference(name, value);
  if (typeof options.windowSize === 'function') {
    options.windowSize({ width: profile.screen.availWidth, height: profile.screen.availHeight });
  }
  return options;
}

/**
 * Chromium (ChromeDriver / msedgedriver): send `veneer.cdpCommands` through the driver's DevTools endpoint.
 * Applies to the current tab; call again after switching to a new tab or window.
 */
async function applyCdp(driver, profile) {
  if (!driver || typeof driver.sendDevToolsCommand !== 'function') {
    throw new TypeError('applyCdp needs a Chromium driver with sendDevToolsCommand()');
  }
  await veneer.applyCdpCommands((method, params) => driver.sendDevToolsCommand(method, params), profile);
}

/**
 * WebDriver BiDi (Firefox, or Chromium with BiDi enabled): register the init script as a preload script for
 * every browsing context. The session must have been created with `webSocketUrl: true` (`options.enableBidi()`).
 * Returns the preload script id.
 */
async function applyBidiPreloadScript(driver, profile, { scriptManager } = {}) {
  const getScriptManager = scriptManager || require('selenium-webdriver/bidi/scriptManager');
  const manager = await getScriptManager([], driver);
  return manager.addPreloadScript(`() => {\n${veneer.toInitScript(profile)}\n}`);
}

/**
 * Apply what can be applied after the session started: CDP on Chromium, a BiDi preload script on Firefox.
 * User agent, language and window size for Firefox come from `firefoxOptions` at session creation.
 */
async function applyProfile(driver, profile) {
  if (profile.browser.engine === 'chromium') return applyCdp(driver, profile);
  if (profile.browser.engine === 'gecko') {
    await applyBidiPreloadScript(driver, profile);
    return undefined;
  }
  throw new TypeError('Selenium can apply chromium or gecko profiles; use Playwright WebKit for Safari profiles');
}

module.exports = { chromiumOptions, firefoxOptions, applyCdp, applyBidiPreloadScript, applyProfile };
