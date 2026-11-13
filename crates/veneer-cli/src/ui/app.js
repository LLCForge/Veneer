'use strict';

const $ = (id) => document.getElementById(id);
const t = (key) => window.veneerI18n.t(key);
const form = $('form');
let lastJson = '';
let lastData = null;
let catalog = null;

async function api(path, options) {
  const res = await fetch(path, { credentials: 'same-origin', ...options });
  if (res.status === 401) {
    $('auth').hidden = false;
    throw new Error(t('needToken'));
  }
  const body = res.status === 204 ? null : await res.json();
  if (!res.ok) throw new Error(body && body.error ? body.error : `${t('requestFailed')} (${res.status})`);
  return body;
}

function fill(select, items) {
  const any = select.options[0];
  select.replaceChildren(any);
  for (const item of items) {
    const option = document.createElement('option');
    option.value = item.id;
    option.textContent = item.label;
    select.append(option);
  }
}

function renderCatalog() {
  if (!catalog) return;
  const keep = Object.fromEntries(['os', 'browser', 'device'].map((k) => [k, form.elements[k].value]));
  fill(form.elements.os, catalog.os.map((o) => ({ id: o.id, label: o.name })));
  fill(form.elements.browser, catalog.browsers.map((b) => ({ id: b.id, label: b.name })));
  fill(form.elements.device, catalog.devices.map((d) => ({ id: d.id, label: `${d.id} (${t(`kind.${d.kind}`)})` })));
  for (const [k, v] of Object.entries(keep)) form.elements[k].value = v;
}

async function loadCatalog() {
  catalog = await api('/v1/catalog');
  renderCatalog();
}

function query() {
  const params = new URLSearchParams();
  for (const [key, value] of new FormData(form)) {
    const text = String(value).trim();
    if (text !== '') params.set(key, text);
  }
  return params;
}

function card(fp) {
  const p = fp.profile;
  const li = document.createElement('li');
  li.className = 'card';
  const title = document.createElement('strong');
  title.textContent = `${p.device.id} · ${p.browser.name} ${p.browser.version}`;
  const meta = document.createElement('span');
  meta.className = 'muted';
  meta.textContent = `${p.os.name} ${p.os.version} · ${p.screen.width}×${p.screen.height} @${p.screen.devicePixelRatio} · ${p.locale} · ${p.timezone}`;
  const score = document.createElement('span');
  score.className = fp.report.valid ? 'badge ok' : 'badge bad';
  score.textContent = `${t('score')} ${fp.report.score}/100`;
  const ua = document.createElement('code');
  ua.textContent = p.navigator.userAgent;
  li.append(title, meta, score, ua);
  return li;
}

function renderResult() {
  if (!lastData) return;
  $('cards').replaceChildren(...lastData.fingerprints.map(card));
  $('json').textContent = lastJson;
}

form.addEventListener('submit', async (event) => {
  event.preventDefault();
  $('error').textContent = '';
  $('generate').disabled = true;
  try {
    const params = query();
    params.set('include', 'headers');
    lastData = await api(`/v1/fingerprint?${params}`);
    lastJson = JSON.stringify(lastData.fingerprints.length === 1 ? lastData.fingerprints[0] : lastData.fingerprints, null, 2);
    renderResult();
    $('copy').disabled = false;
  } catch (err) {
    $('error').textContent = err.message;
  } finally {
    $('generate').disabled = false;
  }
});

$('probe').addEventListener('click', () => {
  const params = query();
  params.delete('count');
  params.set('lang', window.veneerI18n.lang);
  window.open(`/probe?${params}`, '_blank', 'noopener');
});

$('copy').addEventListener('click', async () => {
  try {
    await navigator.clipboard.writeText(lastJson);
    $('copy').textContent = t('copied');
    setTimeout(() => { $('copy').textContent = t('copy'); }, 1500);
  } catch {
    $('error').textContent = t('copyFailed');
  }
});

$('auth-form').addEventListener('submit', async (event) => {
  event.preventDefault();
  $('auth-error').textContent = '';
  try {
    await api('/v1/session', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ token: $('token').value }),
    });
    $('token').value = '';
    $('auth').hidden = true;
    await loadCatalog();
  } catch (err) {
    $('auth-error').textContent = err.message;
  }
});

// Language switch re-renders text that was built in JavaScript.
document.addEventListener('veneer:lang', () => {
  renderCatalog();
  renderResult();
});

loadCatalog().catch((err) => { $('error').textContent = err.message; });
