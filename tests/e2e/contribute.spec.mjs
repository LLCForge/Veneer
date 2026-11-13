// The /contribute page of `veneer serve` builds an anonymous report locally and never sends it anywhere.
import { test, expect } from '@playwright/test';

const API = process.env.VENEER_API;
const ALLOWED = [
  'userAgent', 'appVersion', 'platform', 'vendor', 'languages', 'hardwareConcurrency', 'deviceMemory',
  'maxTouchPoints', 'oscpu', 'webdriver', 'devicePixelRatio', 'screen', 'webgl', 'userAgentData',
];

test('contribute page builds an anonymous report without sending it @chromium @firefox @webkit', async ({ page }) => {
  test.skip(!API, 'needs a running `veneer serve` at VENEER_API');
  const requests = [];
  page.on('request', (r) => requests.push({ method: r.method(), url: r.url() }));

  await page.goto(`${API}/contribute?lang=en`);
  await expect(page.locator('h1')).toHaveText('Contribute device data');
  await page.getByRole('button', { name: 'Create report' }).click();
  await expect(page.locator('#copy')).toBeEnabled();

  const report = JSON.parse(await page.locator('#report').textContent());
  expect(report.format).toBe('veneer-device-report');
  expect(report.version).toBe(1);
  expect(Object.keys(report.observed).sort()).toEqual([...ALLOWED].sort());
  expect(report.observed.userAgent).toBe(await page.evaluate(() => navigator.userAgent));
  expect(report.observed.screen.width).toBe(await page.evaluate(() => screen.width));
  const text = JSON.stringify(report);
  expect(text).not.toContain('timezone');
  expect(text).not.toContain(await page.evaluate(() => Intl.DateTimeFormat().resolvedOptions().timeZone));

  // Only same-origin GETs for the page's own assets; no report upload, no third parties.
  const origin = new URL(API).origin;
  expect(requests.filter((r) => r.method !== 'GET' || new URL(r.url).origin !== origin)).toEqual([]);

  await page.getByRole('button', { name: 'TR' }).click();
  await expect(page.locator('h1')).toHaveText('Cihaz verisi katkısı');
  expect(JSON.parse(await page.locator('#report').textContent())).toEqual(report);
});
