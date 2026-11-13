// For each engine: launch it with Veneer profiles and check what pages, servers and fingerprinting code observe.
import { randomUUID } from 'node:crypto';
import { test, expect, chromium, firefox, webkit } from '@playwright/test';
import { veneer, BASE, FINGERPRINTJS, casesFor, label, open } from './fixtures.mjs';
import { findLies, findWorkerLies } from './lies.mjs';

const playwright = { chromium, firefox, webkit };

for (const engine of Object.keys(playwright)) {
  for (const constraints of casesFor(engine)) {
    test.describe(`${label(constraints)} @${engine}`, () => {
      let profile;
      let session;
      let id;

      test.beforeAll(async () => {
        profile = veneer.generate(constraints);
        expect(veneer.playwrightEngine(profile)).toBe(engine);
        session = await open(playwright, profile);
        id = randomUUID();
        await session.page.goto(`${BASE}/page/${id}`);
      });

      test.afterAll(async () => {
        await session?.browser.close();
      });

      test('collector sees no live errors', async () => {
        const report = await veneer.verifyPage(session.page, profile);
        expect(report.issues.filter((i) => i.severity === 'error')).toEqual([]);
      });

      test('navigation request headers match the profile', async () => {
        const res = await fetch(`${BASE}/seen/${id}`);
        expect(res.ok).toBe(true);
        const sent = await res.json();
        const expected = Object.fromEntries(veneer.navigationHeaders(profile).map((h) => [h.name, h.value]));
        expect(sent['user-agent']).toBe(expected['user-agent']);
        if (engine === 'chromium') {
          // Chromium gets the full header set through CDP Emulation.setUserAgentOverride.
          expect(sent['accept-language']).toBe(expected['accept-language']);
          for (const name of ['sec-ch-ua', 'sec-ch-ua-mobile', 'sec-ch-ua-platform']) {
            expect(sent[name], name).toBe(expected[name]);
          }
        } else {
          // Playwright's Firefox/WebKit only take one `locale`, so only the primary language is controlled.
          expect(sent['accept-language'].split(',')[0]).toBe(profile.navigator.languages[0]);
          expect(sent['sec-ch-ua']).toBeUndefined();
        }
      });

      test('FingerprintJS sees the profile', async () => {
        await session.page.addScriptTag({ path: FINGERPRINTJS });
        const c = await session.page.evaluate(async () => {
          const agent = await window.FingerprintJS.load({ monitoring: false });
          const { components } = await agent.get();
          return Object.fromEntries(Object.entries(components).map(([k, v]) => [k, 'error' in v ? { error: String(v.error) } : { value: v.value }]));
        });
        const nav = profile.navigator;
        const got = {
          platform: c.platform.value,
          vendor: c.vendor.value,
          hardwareConcurrency: c.hardwareConcurrency.value,
          deviceMemory: c.deviceMemory.value ?? null,
          maxTouchPoints: c.touchSupport.value.maxTouchPoints,
          colorDepth: c.colorDepth.value,
          timezone: c.timezone.value,
          language: c.languages.value[0][0],
          osCpu: c.osCpu.value ?? null,
          unmaskedVendor: c.webGlBasics.value?.vendorUnmasked ?? null,
          unmaskedRenderer: c.webGlBasics.value?.rendererUnmasked ?? null,
          uaPlatform: c.userAgentData.value?.platform ?? null,
          uaMobile: c.userAgentData.value?.mobile ?? null,
        };
        const want = {
          platform: nav.platform,
          vendor: nav.vendor,
          hardwareConcurrency: nav.hardwareConcurrency,
          deviceMemory: nav.deviceMemory ?? null,
          maxTouchPoints: nav.maxTouchPoints,
          colorDepth: profile.screen.colorDepth,
          timezone: profile.timezone,
          language: nav.languages[0],
          osCpu: nav.oscpu ?? null,
          unmaskedVendor: profile.webgl.unmaskedVendor ?? null,
          unmaskedRenderer: profile.webgl.unmaskedRenderer ?? null,
          uaPlatform: nav.userAgentData?.platform ?? null,
          uaMobile: nav.userAgentData?.mobile ?? null,
        };
        expect(got).toEqual(want);
        // Safari 17+ hides the screen size from FingerprintJS on purpose; elsewhere it must be the profile's.
        if (c.screenResolution.value) {
          expect([...c.screenResolution.value].sort((a, b) => b - a)).toEqual(
            [profile.screen.width, profile.screen.height].sort((a, b) => b - a),
          );
        }
        const failed = Object.entries(c).filter(([, v]) => 'error' in v).map(([k, v]) => `${k}: ${v.error}`);
        expect(failed).toEqual([]);
      });

      test('no CreepJS-style lies in the page and iframes', async () => {
        const lies = await session.page.evaluate(findLies, {
          screen: { width: profile.screen.width, height: profile.screen.height },
          dpr: profile.screen.devicePixelRatio,
        });
        expect(lies).toEqual([]);
      });

      test('worker navigator agrees with the page', async () => {
        const lies = await session.page.evaluate(findWorkerLies, `${BASE}/worker.js`);
        expect(lies).toEqual([]);
      });
    });
  }
}
