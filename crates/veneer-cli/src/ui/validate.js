'use strict';

// Validator page: posts the pasted JSON to /v1/check and renders the report. Text only, never HTML.
const $ = (id) => document.getElementById(id);
const t = (key) => window.veneerI18n.t(key);
const MAX_INPUT = 60000;
let last = null;

const show = (v) => (typeof v === 'string' ? v : JSON.stringify(v));

function cell(text, className) {
  const td = document.createElement('td');
  td.textContent = text;
  if (className) td.className = className;
  return td;
}

function render() {
  if (!last) return;
  const { report, format, missing } = last;
  $('score').textContent = `${t('score')} ${report.score}/100`;
  $('score').className = report.valid ? 'badge ok' : 'badge bad';
  $('summary').removeAttribute('data-i18n');
  $('summary').textContent = report.issues.length === 0
    ? `${t('noIssues')} · ${t('formatLabel')}: ${t(`format.${format}`)} · ${t('rulesVersion')} ${report.rulesVersion}`
    : `${report.issues.length} ${t('issuesFound')} · ${t('formatLabel')}: ${t(`format.${format}`)} · ${t('rulesVersion')} ${report.rulesVersion}`;
  $('missing').textContent = missing.length ? `${t('missingFields')}: ${missing.join(', ')}` : '';
  $('rows').replaceChildren(...report.issues.map((issue) => {
    const tr = document.createElement('tr');
    const fix = issue.fix
      ? `${issue.fix.action} ${issue.fix.path}${issue.fix.expected === undefined ? '' : ` = ${show(issue.fix.expected)}`}`
      : '';
    tr.append(
      cell(t(`severity.${issue.severity}`), issue.severity === 'error' ? 'bad' : ''),
      cell(issue.rule),
      cell(issue.hint ? `${issue.message} — ${issue.hint}` : issue.message),
      cell(fix),
    );
    return tr;
  }));
}

function options() {
  const country = $('country').value.trim().toUpperCase();
  const timezone = $('timezone').value.trim();
  const disable = $('disable').value.split(',').map((s) => s.trim()).filter(Boolean);
  const out = {};
  if (disable.length) out.disable = disable;
  if (country || timezone) out.geo = { ...(country && { country }), ...(timezone && { timezone }) };
  return out;
}

async function request(path, init) {
  const res = await fetch(path, { credentials: 'same-origin', ...init });
  let data = null;
  try {
    data = await res.json();
  } catch {
    // Non-JSON error body; reported by status below.
  }
  if (res.status === 401) throw new Error(t('needToken'));
  if (!res.ok) throw new Error((data && data.error) || `${t('requestFailed')} (${res.status})`);
  return data;
}

async function check(event) {
  event.preventDefault();
  $('error').textContent = '';
  const text = $('input').value;
  if (text.length > MAX_INPUT) {
    $('error').textContent = t('inputTooLong');
    return;
  }
  let input;
  try {
    input = JSON.parse(text);
  } catch {
    $('error').textContent = t('invalidJson');
    return;
  }
  try {
    last = await request('/v1/check', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ input, format: $('format').value, options: options() }),
    });
    render();
  } catch (err) {
    $('error').textContent = err.message;
  }
}

async function sample() {
  $('error').textContent = '';
  try {
    const data = await request('/v1/fingerprint', {});
    const profile = data.fingerprints[0].profile;
    // A visible contradiction so the report has something to show.
    profile.navigator.platform = profile.navigator.platform === 'Win32' ? 'MacIntel' : 'Win32';
    profile.timezone = 'America/New_York';
    $('input').value = JSON.stringify(profile, null, 2);
    $('format').value = 'veneer';
  } catch (err) {
    $('error').textContent = err.message;
  }
}

$('form').addEventListener('submit', check);
$('sample').addEventListener('click', sample);
document.addEventListener('veneer:lang', render);
