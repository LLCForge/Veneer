'use strict';

const veneer = require('@llcforge/veneer');
const { test, expect } = require('../..');

test.describe('macOS profile', () => {
  test.use({ veneer: { os: 'macos', locale: 'tr-TR' } });

  test('page matches the profile and sends matching headers', async ({ page, veneerProfile }) => {
    expect(veneerProfile.os.id).toBe('macos');
    expect(veneerProfile.browser.engine).toBe('chromium');
    const sent = {};
    page.on('request', (r) => {
      if (r.isNavigationRequest()) Object.assign(sent, r.headers());
    });
    await page.goto(`${process.env.VENEER_API}/`);
    const report = await veneer.verifyPage(page, veneerProfile);
    expect(report.issues.filter((i) => i.severity === 'error')).toEqual([]);
    expect(sent['user-agent']).toBe(veneerProfile.navigator.userAgent);
    expect(sent['sec-ch-ua-platform']).toBe('"macOS"');
    expect(sent['accept-language']).toMatch(/^tr-TR/);
  });

  test('popups get the profile too', async ({ page, veneerProfile }) => {
    await page.goto(`${process.env.VENEER_API}/`);
    const [popup] = await Promise.all([page.waitForEvent('popup'), page.evaluate((u) => window.open(u), `${process.env.VENEER_API}/`)]);
    await popup.waitForLoadState();
    expect(await popup.evaluate(() => navigator.userAgent)).toBe(veneerProfile.navigator.userAgent);
    expect(await popup.evaluate(() => Intl.DateTimeFormat().resolvedOptions().timeZone)).toBe(veneerProfile.timezone);
  });
});

test.describe('without the option', () => {
  test('nothing is applied', async ({ veneerProfile }) => {
    expect(veneerProfile).toBeUndefined();
  });
});
