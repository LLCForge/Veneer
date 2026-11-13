'use strict';

// Apply a Veneer profile to a remote Chromium (Browserless, a Docker container, a grid node, …).
//
//   BROWSER_WS=wss://<host>/?token=<token> node remote-browser.js https://example.com
//   BROWSER_WS=ws://127.0.0.1:9222/devtools/browser/<id> CLIENT=puppeteer node remote-browser.js https://example.com
//
// The endpoint usually carries a token, so it is read from the environment and never printed.
const veneer = require('@llcforge/veneer');

async function withPlaywright(endpoint, profile, url) {
  const { chromium } = require('playwright');
  const browser = await chromium.connectOverCDP(endpoint);
  try {
    // Remote browsers are Chromium, so newContext also applies the CDP header override to each page.
    const context = await veneer.newContext(browser, profile);
    const page = await veneer.newPage(context, profile);
    await page.goto(url);
    const report = await veneer.verifyPage(page, profile);
    await context.close();
    return report;
  } finally {
    await browser.close();
  }
}

async function withPuppeteer(endpoint, profile, url) {
  const puppeteer = require('puppeteer-core');
  const browser = await puppeteer.connect({ browserWSEndpoint: endpoint });
  try {
    const context = await browser.createBrowserContext();
    const page = await context.newPage();
    const session = await page.createCDPSession();
    await veneer.applyCdpCommands((method, params) => session.send(method, params), profile);
    await page.goto(url);
    const report = await veneer.verifyPage(page, profile);
    await context.close();
    return report;
  } finally {
    // disconnect, not close: the remote service owns the browser process.
    await browser.disconnect();
  }
}

async function main() {
  const endpoint = process.env.BROWSER_WS;
  const url = process.argv[2];
  if (!endpoint || !/^wss?:\/\//.test(endpoint)) throw new Error('set BROWSER_WS to a ws:// or wss:// endpoint');
  if (!url || !/^https?:\/\//.test(url)) throw new Error('usage: node remote-browser.js <http(s) url>');
  // Remote services run Chromium, so ask for a Chromium profile.
  const profile = veneer.generate({ engine: 'chromium', os: process.env.VENEER_OS || 'windows' });
  const run = process.env.CLIENT === 'puppeteer' ? withPuppeteer : withPlaywright;
  const report = await run(endpoint, profile, url);
  console.log(JSON.stringify({ seed: profile.seed, device: profile.device.id, valid: report.valid, issues: report.issues }));
  if (report.issues.some((i) => i.severity === 'error')) process.exitCode = 1;
}

if (require.main === module) {
  main().catch((err) => {
    console.error(String(err && err.message ? err.message : err));
    process.exitCode = 1;
  });
}

module.exports = { withPlaywright, withPuppeteer };
