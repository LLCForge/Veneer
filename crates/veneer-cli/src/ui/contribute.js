'use strict';

// Builds an anonymous device report in this browser. Nothing is sent anywhere: the visitor copies it into an issue.
const $ = (id) => document.getElementById(id);
const t = (key) => window.veneerI18n.t(key);

const MAX_STRING = 512;
const MAX_ITEMS = 64;

// Allowlist of collector fields: hardware and browser characteristics only. Timezone, canvas/audio results and
// anything not listed here never reach the report.
const FIELDS = {
  userAgent: 'string',
  appVersion: 'string',
  platform: 'string',
  vendor: 'string',
  languages: 'strings',
  hardwareConcurrency: 'number',
  deviceMemory: 'number',
  maxTouchPoints: 'number',
  oscpu: 'string',
  webdriver: 'boolean',
  devicePixelRatio: 'number',
  screen: { width: 'number', height: 'number', availWidth: 'number', availHeight: 'number', colorDepth: 'number', pixelDepth: 'number' },
  webgl: { vendor: 'string', renderer: 'string', unmaskedVendor: 'string', unmaskedRenderer: 'string' },
  userAgentData: {
    brands: [{ brand: 'string', version: 'string' }],
    mobile: 'boolean',
    platform: 'string',
    architecture: 'string',
    bitness: 'string',
    model: 'string',
    platformVersion: 'string',
    fullVersionList: [{ brand: 'string', version: 'string' }],
  },
};

const MEDIA = {
  pointer: ['none', 'coarse', 'fine'],
  'any-pointer': ['none', 'coarse', 'fine'],
  hover: ['none', 'hover'],
  'color-gamut': ['rec2020', 'p3', 'srgb'],
  'dynamic-range': ['high', 'standard'],
  orientation: ['portrait', 'landscape'],
};

/** Copy only what the schema allows, with type and size limits; everything else becomes null or is dropped. */
function pick(value, schema) {
  if (value === null || value === undefined) return null;
  if (schema === 'string') return typeof value === 'string' ? value.slice(0, MAX_STRING) : null;
  if (schema === 'number') return typeof value === 'number' && Number.isFinite(value) ? value : null;
  if (schema === 'boolean') return typeof value === 'boolean' ? value : null;
  if (schema === 'strings') {
    return Array.isArray(value) ? value.slice(0, MAX_ITEMS).filter((v) => typeof v === 'string').map((v) => v.slice(0, MAX_STRING)) : null;
  }
  if (Array.isArray(schema)) {
    return Array.isArray(value) ? value.slice(0, MAX_ITEMS).map((v) => pick(v, schema[0])) : null;
  }
  if (typeof value !== 'object') return null;
  return Object.fromEntries(Object.entries(schema).map(([k, s]) => [k, pick(value[k], s)]));
}

function media() {
  return Object.fromEntries(
    Object.entries(MEDIA).map(([feature, values]) => [feature, values.find((v) => matchMedia(`(${feature}: ${v})`).matches) ?? null]),
  );
}

async function buildReport() {
  if (typeof window.veneerCollect !== 'function') throw new Error(t('collectorMissing'));
  const observed = await window.veneerCollect();
  return {
    format: 'veneer-device-report',
    version: 1,
    secureContext: window.isSecureContext,
    observed: pick(observed, FIELDS),
    media: media(),
  };
}

let report = null;

function render() {
  if (!report) return;
  $('note').textContent = report.secureContext ? '' : t('contributeInsecure');
}

async function run() {
  $('error').textContent = '';
  $('run').disabled = true;
  try {
    report = await buildReport();
    const pre = $('report');
    pre.removeAttribute('data-i18n');
    pre.textContent = JSON.stringify(report, null, 2);
    $('copy').disabled = false;
    render();
  } catch (err) {
    $('error').textContent = err instanceof Error ? err.message : String(err);
  } finally {
    $('run').disabled = false;
  }
}

async function copy() {
  const text = $('report').textContent;
  try {
    await navigator.clipboard.writeText(text);
    $('copy').textContent = t('copied');
  } catch {
    // Clipboard API needs a secure context; select the text so the visitor can copy it by hand.
    const range = document.createRange();
    range.selectNodeContents($('report'));
    const selection = window.getSelection();
    selection.removeAllRanges();
    selection.addRange(range);
    $('error').textContent = t('copyFailed');
  }
}

document.addEventListener('veneer:lang', () => {
  render();
  $('copy').textContent = t('copy');
});
$('run').addEventListener('click', run);
$('copy').addEventListener('click', copy);
