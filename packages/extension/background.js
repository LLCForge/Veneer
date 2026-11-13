'use strict';

// Only the commands `veneer.cdpCommands(profile)` produces; anything else in a pasted bundle is refused.
const ALLOWED = new Set([
  'Emulation.setUserAgentOverride',
  'Emulation.setTimezoneOverride',
  'Emulation.setLocaleOverride',
  'Emulation.setDeviceMetricsOverride',
  'Emulation.setTouchEmulationEnabled',
  'Page.enable',
  'Page.addScriptToEvaluateOnNewDocument',
]);
const MAX_COMMANDS = 16;
const MAX_BUNDLE_CHARS = 2 * 1024 * 1024;

/** Throws unless `commands` is a short list of allowed CDP commands with object params. */
function checkCommands(commands) {
  if (!Array.isArray(commands) || commands.length === 0 || commands.length > MAX_COMMANDS) {
    throw new Error('commands must be a non-empty list of at most 16 CDP commands');
  }
  if (JSON.stringify(commands).length > MAX_BUNDLE_CHARS) throw new Error('bundle is too large');
  for (const c of commands) {
    if (!c || typeof c !== 'object' || !ALLOWED.has(c.method)) {
      throw new Error(`command not allowed: ${String(c && c.method)}`);
    }
    if (!c.params || typeof c.params !== 'object' || Array.isArray(c.params)) {
      throw new Error(`${c.method}: params must be an object`);
    }
  }
  return commands;
}

async function apply(tabId, commands) {
  if (!Number.isInteger(tabId)) throw new Error('tabId must be an integer');
  checkCommands(commands);
  const target = { tabId };
  // getTargets().attached is true for any client (DevTools, automation), so remember our own attachments.
  // A fresh session per apply drops the previous profile's overrides and init script.
  const key = `tab:${tabId}`;
  if ((await chrome.storage.session.get(key))[key] === true) {
    await chrome.debugger.detach(target).catch(() => {});
    await chrome.storage.session.remove(key);
  }
  await chrome.debugger.attach(target, '1.3');
  try {
    for (const { method, params } of commands) await chrome.debugger.sendCommand(target, method, params);
  } catch (err) {
    await chrome.debugger.detach(target).catch(() => {});
    throw err;
  }
  await chrome.storage.session.set({ [key]: true });
  // The init script runs on new documents, so reload to apply it to this page.
  await chrome.tabs.reload(tabId);
}

async function detach(tabId) {
  if (!Number.isInteger(tabId)) throw new Error('tabId must be an integer');
  await chrome.debugger.detach({ tabId }).catch(() => {});
  await chrome.storage.session.remove(`tab:${tabId}`);
  await chrome.tabs.reload(tabId);
}

chrome.debugger.onDetach.addListener((source) => {
  if (Number.isInteger(source.tabId)) chrome.storage.session.remove(`tab:${source.tabId}`);
});

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  // Only this extension's own pages may drive the debugger.
  if (sender.id !== chrome.runtime.id || !message || typeof message !== 'object') return false;
  let run = null;
  if (message.type === 'apply') run = apply(message.tabId, message.commands);
  else if (message.type === 'detach') run = detach(message.tabId);
  if (!run) return false;
  run.then(
    () => sendResponse({ ok: true }),
    (err) => sendResponse({ ok: false, error: String(err && err.message ? err.message : err) }),
  );
  return true;
});

// For tests and the service-worker console: apply a bundle to a tab directly.
globalThis.veneerApply = apply;
globalThis.veneerDetach = detach;
