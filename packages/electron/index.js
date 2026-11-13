'use strict';

const veneer = require('@llcforge/veneer');

/**
 * Apply a Chromium profile to an Electron `webContents` through its built-in debugger, before loading a URL.
 * Overrides last while the debugger stays attached (navigations included). Returns a function that detaches.
 */
async function applyToWebContents(webContents, profile) {
  const dbg = webContents && webContents.debugger;
  if (!dbg || typeof dbg.sendCommand !== 'function') throw new TypeError('expected an Electron webContents');
  const attachedHere = !dbg.isAttached();
  if (attachedHere) dbg.attach('1.3');
  try {
    await veneer.applyCdpCommands((method, params) => dbg.sendCommand(method, params), profile);
  } catch (err) {
    if (attachedHere && dbg.isAttached()) dbg.detach();
    throw err;
  }
  return () => {
    if (dbg.isAttached()) dbg.detach();
  };
}

/**
 * Session-wide user agent and Accept-Language (covers workers and requests made outside a page).
 * Call before creating windows on that session.
 */
function applyToElectronSession(electronSession, profile) {
  if (!electronSession || typeof electronSession.setUserAgent !== 'function') {
    throw new TypeError('expected an Electron session');
  }
  if (profile.browser.engine !== 'chromium') throw new TypeError('Electron is Chromium; use a chromium profile');
  electronSession.setUserAgent(profile.navigator.userAgent, profile.navigator.languages.join(','));
}

/** `app.commandLine.appendSwitch(name, value)` pairs (call before `app.whenReady()`). */
function commandLineSwitches(profile) {
  return veneer.chromiumArgs(profile).map((arg) => {
    const eq = arg.indexOf('=');
    return [arg.slice(2, eq), arg.slice(eq + 1)];
  });
}

module.exports = { applyToWebContents, applyToElectronSession, commandLineSwitches };
