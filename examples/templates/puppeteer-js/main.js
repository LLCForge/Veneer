// Puppeteer + Veneer: Chromium profiles only (Puppeteer drives Chrome).
// Usage: npm install && npm start -- https://example.com/
import { createRequire } from 'node:module';
import puppeteer from 'puppeteer';

const require = createRequire(import.meta.url);
const veneer = require('@llcforge/veneer');

const url = process.argv[2] ?? 'https://example.com/';
if (!/^https?:\/\//.test(url)) {
  throw new Error('the target must be an http(s) URL');
}

const profile = veneer.generate({ os: 'windows', browser: 'chrome', locale: 'en-US' });
const { args } = veneer.webrtcSettings('chromium');
const browser = await puppeteer.launch({
  headless: true,
  args: [...args, `--window-size=${profile.viewport.width},${profile.viewport.height}`],
});

try {
  const page = await browser.newPage();
  await veneer.applyProfile(page, profile);
  const response = await page.goto(url, { waitUntil: 'load' });
  if (!response) throw new Error(`no response from ${url}`);
  // Puppeteer exposes headers as an object, so compareHeaders skips the order check.
  const headerReport = veneer.compareHeaders(profile, response.request().headers());
  const pageReport = await veneer.verifyPage(page, profile);
  console.log(`${profile.device.id} / ${profile.browser.name} ${profile.browser.version}`);
  console.log(`page:    valid=${pageReport.valid}`, pageReport.issues);
  console.log(`headers: valid=${headerReport.valid}`, headerReport.issues);
  process.exitCode = pageReport.valid && headerReport.valid ? 0 : 1;
} finally {
  await browser.close();
}
