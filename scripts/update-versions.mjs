#!/usr/bin/env node
// Refresh Chrome, Edge, Firefox (and the browsers that track them) in data/browsers/*.json from official release feeds.
// Usage: node scripts/update-versions.mjs [--dry-run]

import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const DIR = fileURLToPath(new URL('../crates/veneer/data/browsers/', import.meta.url));
const DRY_RUN = process.argv.includes('--dry-run');
const WEIGHTS = [75, 25];

async function getJson(url) {
  const res = await fetch(url, { signal: AbortSignal.timeout(20_000), headers: { accept: 'application/json' } });
  if (!res.ok) throw new Error(`${url}: HTTP ${res.status}`);
  return res.json();
}

const major = (v) => Number.parseInt(String(v).split('.')[0], 10);
const isVersion = (v) => typeof v === 'string' && /^\d+(\.\d+){1,3}$/.test(v);

function compare(a, b) {
  const pa = a.split('.').map(Number);
  const pb = b.split('.').map(Number);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d !== 0) return d;
  }
  return 0;
}

/** Newest full version for each of the two newest majors. */
function latestTwoMajors(versions) {
  const byMajor = new Map();
  for (const v of versions.filter(isVersion)) {
    const m = major(v);
    const current = byMajor.get(m);
    if (!current || compare(v, current) > 0) byMajor.set(m, v);
  }
  return [...byMajor.entries()].sort((a, b) => b[0] - a[0]).slice(0, 2).map(([, v]) => v);
}

async function chrome() {
  const data = await getJson('https://versionhistory.googleapis.com/v1/chrome/platforms/win/channels/stable/versions');
  return latestTwoMajors(data.versions.map((v) => v.version));
}

async function edge() {
  const products = await getJson('https://edgeupdates.microsoft.com/api/products');
  const stable = products.find((p) => p.Product === 'Stable');
  if (!stable) throw new Error('Edge feed has no Stable product');
  return latestTwoMajors(stable.Releases.filter((r) => r.Platform === 'Windows').map((r) => r.ProductVersion));
}

async function firefox() {
  const data = await getJson('https://product-details.mozilla.org/1.0/firefox_versions.json');
  const latest = data.LATEST_FIREFOX_VERSION;
  if (!isVersion(latest)) throw new Error(`unexpected Firefox version ${latest}`);
  return `${major(latest)}.0`;
}

/** New text for one browser file with its `versions` array replaced; other fields untouched. */
function withVersions(id, entries) {
  const file = `${DIR}${id}.json`;
  const text = readFileSync(file, 'utf8');
  const open = text.indexOf('"versions": [');
  if (open < 0) throw new Error(`${file}: no versions array`);
  const close = text.indexOf(']', open);
  const body = entries.map((e) => `    ${JSON.stringify(e)}`).join(',\n');
  const next = `${text.slice(0, open)}"versions": [\n${body}\n  ${text.slice(close)}`;
  JSON.parse(next);
  return [file, next];
}

const [chromeVersions, edgeVersions, firefoxFull] = await Promise.all([chrome(), edge(), firefox()]);
if (chromeVersions.length < 2 || edgeVersions.length < 1) throw new Error('release feeds returned too few versions');

const chromeEntries = chromeVersions.map((full, i) => ({ major: major(full), full, weight: WEIGHTS[i] }));
const chromeFullFor = (m) => chromeVersions.find((v) => major(v) === m) ?? `${m}.0.0.0`;
// chrome-ios and firefox-ios ship their own builds (Chrome Releases iOS posts, firefox-ios releases);
// desktop numbers would be wrong there, so they stay hand-maintained.
const updates = {
  chrome: chromeEntries,
  brave: chromeVersions.map((full, i) => ({ major: major(full), full: `${major(full)}.0.0.0`, weight: WEIGHTS[i] })),
  edge: edgeVersions.map((full, i) => ({
    major: major(full),
    full,
    chromium: major(full),
    chromiumFull: chromeFullFor(major(full)),
    weight: WEIGHTS[i] ?? 25,
  })),
  firefox: [{ major: major(firefoxFull), full: firefoxFull, weight: 100 }],
};

const writes = Object.entries(updates).map(([id, entries]) => withVersions(id, entries));
for (const [id, entries] of Object.entries(updates)) console.log(`${id.padEnd(12)} ${entries.map((e) => e.full).join(', ')}`);
if (DRY_RUN) {
  console.log('dry run: nothing written');
} else {
  for (const [file, text] of writes) writeFileSync(file, text);
  console.log(`updated ${writes.length} browser files`);
}
