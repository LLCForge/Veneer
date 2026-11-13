'use strict';

const $ = (id) => document.getElementById(id);
const t = (key) => window.veneerI18n.t(key);

const FIELDS = [
  ['userAgent', (p) => p.navigator.userAgent],
  ['platform', (p) => p.navigator.platform],
  ['vendor', (p) => p.navigator.vendor],
  ['languages', (p) => p.navigator.languages],
  ['hardwareConcurrency', (p) => p.navigator.hardwareConcurrency],
  ['deviceMemory', (p) => p.navigator.deviceMemory ?? null],
  ['maxTouchPoints', (p) => p.navigator.maxTouchPoints],
  ['screen.width', (p) => p.screen.width],
  ['screen.height', (p) => p.screen.height],
  ['screen.availHeight', (p) => p.screen.availHeight],
  ['screen.colorDepth', (p) => p.screen.colorDepth],
  ['devicePixelRatio', (p) => p.screen.devicePixelRatio],
  ['timezone', (p) => p.timezone],
  ['webgl.unmaskedVendor', (p) => p.webgl.unmaskedVendor ?? null],
  ['webgl.unmaskedRenderer', (p) => p.webgl.unmaskedRenderer ?? null],
];

const get = (obj, path) => path.split('.').reduce((o, k) => (o == null ? null : o[k]), obj) ?? null;
const show = (v) => (typeof v === 'string' ? v : JSON.stringify(v));
let state = null;

async function post(path, body) {
  const res = await fetch(path, {
    method: 'POST',
    credentials: 'same-origin',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
  const data = await res.json();
  if (!res.ok) throw new Error(data.error ?? `${t('requestFailed')} (${res.status})`);
  return data;
}

function render() {
  if (!state) return;
  const { profile, observed, report } = state;
  const failed = new Set(report.issues.filter((i) => i.severity === 'error').map((i) => i.rule.replace(/^live\./, '')));
  $('rows').replaceChildren(...FIELDS.map(([field, expected]) => {
    const tr = document.createElement('tr');
    const ok = !failed.has(field);
    for (const text of [field, show(expected(profile)), show(get(observed, field))]) {
      const td = document.createElement('td');
      td.textContent = text;
      tr.append(td);
    }
    const status = document.createElement('td');
    status.className = ok ? 'ok' : 'bad';
    status.textContent = ok ? t('match') : t('differ');
    tr.append(status);
    return tr;
  }));
  $('title').removeAttribute('data-i18n');
  $('title').textContent = `${profile.device.id} · ${profile.browser.name} ${profile.browser.version}`;
  $('score').textContent = `${t('score')} ${report.score}/100`;
  $('score').className = report.valid ? 'badge ok' : 'badge bad';
  const other = report.issues.filter((i) => !FIELDS.some(([f]) => `live.${f}` === i.rule));
  $('error').textContent = other.map((i) => `${i.rule}: ${i.message}`).join(' · ');
}

async function run() {
  if (typeof window.veneerCollect !== 'function') throw new Error(t('collectorMissing'));
  const params = new URLSearchParams(location.search);
  params.delete('lang');
  const res = await fetch(`/v1/fingerprint?${params}`, { credentials: 'same-origin' });
  const data = await res.json();
  if (!res.ok) throw new Error(data.error ?? `${t('requestFailed')} (${res.status})`);
  const profile = data.fingerprints[0].profile;
  const observed = await window.veneerCollect();
  state = { profile, observed, report: await post('/v1/compare', { profile, observed, context: 'page' }) };
  render();
}

document.addEventListener('veneer:lang', render);
run().catch((err) => {
  $('title').removeAttribute('data-i18n');
  $('title').textContent = t('probeFailed');
  $('error').textContent = err.message;
});
