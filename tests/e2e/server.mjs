// Fixture server for the e2e tests: plain pages on 127.0.0.1 that record the request headers each navigation sent.
import { createServer } from 'node:http';

const PORT = Number(process.env.E2E_PORT ?? 8813);
const ID = /^[a-z0-9-]{1,64}$/;
const MAX_RECORDS = 256;
const seen = new Map();

const PAGE = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Veneer e2e</title></head>
<body><main><h1>Veneer e2e</h1><canvas width="16" height="16"></canvas></main></body></html>`;

// Reports what a dedicated worker sees; workers do not run page init scripts, so they are a classic lie detector.
const WORKER = `postMessage({
  userAgent: navigator.userAgent,
  platform: navigator.platform,
  language: navigator.language,
  languages: [...navigator.languages],
  hardwareConcurrency: navigator.hardwareConcurrency,
  deviceMemory: navigator.deviceMemory ?? null,
  timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
});`;

function send(res, status, type, body) {
  res.writeHead(status, { 'content-type': type, 'cache-control': 'no-store', 'x-content-type-options': 'nosniff' });
  res.end(body);
}

const server = createServer((req, res) => {
  if (req.method !== 'GET') return send(res, 405, 'text/plain', 'method not allowed');
  const [, kind, id = ''] = (req.url ?? '/').split('?')[0].split('/');
  if (kind === 'health') return send(res, 200, 'text/plain', 'ok');
  if (kind === 'worker.js') return send(res, 200, 'text/javascript', WORKER);
  if (kind === 'blank') return send(res, 200, 'text/html; charset=utf-8', '<!doctype html><title>blank</title>');
  if (!ID.test(id)) return send(res, 404, 'text/plain', 'not found');
  if (kind === 'page') {
    if (seen.size >= MAX_RECORDS) seen.delete(seen.keys().next().value);
    seen.set(id, req.headers);
    return send(res, 200, 'text/html; charset=utf-8', PAGE);
  }
  if (kind === 'seen' && seen.has(id)) return send(res, 200, 'application/json', JSON.stringify(seen.get(id)));
  return send(res, 404, 'text/plain', 'not found');
});

server.listen(PORT, '127.0.0.1', () => console.log(`e2e fixture server on http://127.0.0.1:${PORT}`));
