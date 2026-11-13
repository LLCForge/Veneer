'use strict';

const veneer = require('@llcforge/veneer');

const DEFAULT_TIMEOUT_MS = 15_000;
// CDP replies to the commands we send are small; anything far larger is not a reply we asked for.
const MAX_MESSAGE_BYTES = 16 * 1024 * 1024;

/**
 * Minimal CDP client over a raw WebSocket (Node 22+ global `WebSocket`, or pass a constructor).
 * `send(method, params, sessionId)` resolves with the result or rejects with the CDP error.
 */
function connect(wsUrl, { WebSocket: WS = globalThis.WebSocket, timeoutMs = DEFAULT_TIMEOUT_MS } = {}) {
  let url;
  try {
    url = new URL(wsUrl);
  } catch {
    return Promise.reject(new TypeError('wsUrl must be a ws:// or wss:// URL'));
  }
  if (url.protocol !== 'ws:' && url.protocol !== 'wss:') {
    return Promise.reject(new TypeError('wsUrl must be a ws:// or wss:// URL'));
  }
  if (typeof WS !== 'function') return Promise.reject(new TypeError('no WebSocket implementation available'));

  return new Promise((resolve, reject) => {
    const socket = new WS(url.href);
    const pending = new Map();
    let nextId = 1;
    let closed = false;

    const failAll = (err) => {
      for (const { reject: rej, timer } of pending.values()) {
        clearTimeout(timer);
        rej(err);
      }
      pending.clear();
    };

    socket.addEventListener('open', () => {
      resolve({
        send(method, params = {}, sessionId) {
          if (closed) return Promise.reject(new Error('CDP connection is closed'));
          if (typeof method !== 'string' || method.length === 0 || method.length > 128) {
            return Promise.reject(new TypeError('method must be a non-empty string'));
          }
          const id = nextId++;
          const message = { id, method, params };
          if (sessionId !== undefined) message.sessionId = sessionId;
          return new Promise((res, rej) => {
            const timer = setTimeout(() => {
              pending.delete(id);
              rej(new Error(`${method} timed out after ${timeoutMs} ms`));
            }, timeoutMs);
            pending.set(id, { resolve: res, reject: rej, timer, method });
            socket.send(JSON.stringify(message));
          });
        },
        close() {
          closed = true;
          socket.close();
        },
      });
    });

    socket.addEventListener('message', (event) => {
      const data = typeof event.data === 'string' ? event.data : String(event.data);
      if (data.length > MAX_MESSAGE_BYTES) return;
      let msg;
      try {
        msg = JSON.parse(data);
      } catch {
        return;
      }
      if (msg === null || typeof msg !== 'object' || typeof msg.id !== 'number') return; // events
      const entry = pending.get(msg.id);
      if (!entry) return;
      pending.delete(msg.id);
      clearTimeout(entry.timer);
      if (msg.error) entry.reject(new Error(`${entry.method}: ${String(msg.error.message || 'CDP error')}`));
      else entry.resolve(msg.result);
    });

    socket.addEventListener('error', () => {
      const err = new Error('CDP WebSocket error');
      if (!closed) reject(err);
      failAll(err);
    });

    socket.addEventListener('close', () => {
      closed = true;
      reject(new Error('CDP WebSocket closed before it opened'));
      failAll(new Error('CDP connection closed'));
    });
  });
}

/** Apply a Chromium profile through any `send(method, params)` function (raw client, Puppeteer, Electron, chrome.debugger). */
function applyToSession(send, profile) {
  return veneer.applyCdpCommands(send, profile);
}

/**
 * Attach to a page target over a browser-level connection (flat sessions) and apply the profile.
 * Returns the `sessionId`; keep the connection open, the overrides last as long as the session.
 */
async function attachAndApply(client, targetId, profile) {
  if (typeof targetId !== 'string' || targetId.length === 0 || targetId.length > 128) {
    throw new TypeError('targetId must be a non-empty string');
  }
  const { sessionId } = await client.send('Target.attachToTarget', { targetId, flatten: true });
  await applyToSession((method, params) => client.send(method, params, sessionId), profile);
  return sessionId;
}

/** Connect to a page target's WebSocket URL (from `/json/list`) and apply the profile. Returns the open client. */
async function applyToTarget(wsUrl, profile, options) {
  const client = await connect(wsUrl, options);
  try {
    await applyToSession((method, params) => client.send(method, params), profile);
    return client;
  } catch (err) {
    client.close();
    throw err;
  }
}

module.exports = { connect, applyToSession, attachAndApply, applyToTarget, cdpCommands: veneer.cdpCommands };
