// Playwright + Veneer: launch the engine that matches a profile, then check the page and the request headers.
// Usage: npm install && npx playwright install chromium && npm start -- https://example.com/
import { createRequire } from 'node:module';
import type * as Veneer from '@llcforge/veneer';
import * as playwright from 'playwright';

const require = createRequire(import.meta.url);
const veneer: typeof Veneer = require('@llcforge/veneer');

const url = process.argv[2] ?? 'https://example.com/';
if (!/^https?:\/\//.test(url)) {
  throw new Error('the target must be an http(s) URL');
}

const profile = veneer.generate({ os: 'windows', browser: 'chrome', locale: 'en-US' });
// Keep WebRTC from exposing local addresses (Chromium switch or Firefox prefs, per engine).
const { playwrightLaunchOptions } = veneer.webrtcSettings(profile.browser.engine);
const { browser, page } = await veneer.launchWithProfile<
  playwright.Browser,
  playwright.BrowserContext,
  playwright.Page
>(playwright, profile, { headless: true, ...playwrightLaunchOptions });

try {
  const response = await page.goto(url, { waitUntil: 'load' });
  if (!response) throw new Error(`no response from ${url}`);
  // headersArray() is sorted by name, not wire order; an object makes compareHeaders skip the order check.
  const sent = await response.request().allHeaders();

  const pageReport = await veneer.verifyPage(page, profile);
  const headerReport = veneer.compareHeaders(profile, sent);
  console.log(`${profile.device.id} / ${profile.browser.name} ${profile.browser.version}`);
  console.log(`page:    valid=${pageReport.valid}`, pageReport.issues);
  console.log(`headers: valid=${headerReport.valid}`, headerReport.issues);
  process.exitCode = pageReport.valid && headerReport.valid ? 0 : 1;
} finally {
  await browser.close();
}
