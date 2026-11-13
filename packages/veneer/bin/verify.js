#!/usr/bin/env node
'use strict';

// Open a fingerprint in a real browser (Playwright), check it live, and visit fingerprint test sites.

const { parseArgs } = require('node:util');
const veneer = require('..');

const DEFAULT_SITES = [
  'https://abrahamjuliot.github.io/creepjs/',
  'https://browserleaks.com/javascript',
  'https://bot.sannysoft.com/',
];

const HELP = `Usage: veneer-verify [constraints] [--site <url>]... [--headless] [--json]

Constraints: --os --browser --kind --device --engine --gpu-vendor --locale --country --timezone --seed
             --min-version --max-version
Options:     --site <url>   page to open and check (repeatable; defaults to public fingerprint test sites)
             --headless     do not show the browser; exit after checking
             --json         print the profile and reports as JSON

Requires Playwright in your project: npm i -D playwright && npx playwright install`;

const CONSTRAINTS = ['os', 'browser', 'kind', 'device', 'engine', 'gpu-vendor', 'locale', 'country', 'timezone', 'seed', 'min-version', 'max-version'];

function parse() {
  const options = Object.fromEntries(CONSTRAINTS.map((k) => [k, { type: 'string' }]));
  options.site = { type: 'string', multiple: true };
  options.headless = { type: 'boolean' };
  options.json = { type: 'boolean' };
  options.help = { type: 'boolean', short: 'h' };
  const { values } = parseArgs({ options, strict: true });
  const constraints = {};
  for (const key of CONSTRAINTS) {
    if (values[key] === undefined) continue;
    const camel = key.replace(/-([a-z])/g, (_, c) => c.toUpperCase());
    constraints[camel] = key.endsWith('version') ? Number.parseInt(values[key], 10) : values[key];
  }
  for (const site of values.site ?? []) {
    const url = new URL(site);
    if (url.protocol !== 'https:' && url.protocol !== 'http:') throw new Error(`unsupported URL: ${site}`);
  }
  return { constraints, values };
}

function loadPlaywright() {
  try {
    return require(require.resolve('playwright', { paths: [process.cwd(), __dirname] }));
  } catch {
    throw new Error('Playwright is not installed. Run: npm i -D playwright && npx playwright install');
  }
}

function printReport(site, report) {
  const errors = report.issues.filter((i) => i.severity === 'error');
  console.log(`\n${site}\n  score ${report.score}/100, ${errors.length} error(s), ${report.issues.length - errors.length} warning(s)`);
  for (const issue of report.issues) console.log(`  ${issue.severity.padEnd(7)} ${issue.rule}: ${issue.message}`);
}

async function main() {
  const { constraints, values } = parse();
  if (values.help) {
    console.log(HELP);
    return 0;
  }
  const playwright = loadPlaywright();
  const profile = veneer.generate(constraints);
  const offline = veneer.validate(profile);
  const sites = values.site?.length ? values.site : DEFAULT_SITES;

  if (!values.json) {
    console.log(`${profile.device.id} · ${profile.os.name} ${profile.os.version} · ${profile.browser.name} ${profile.browser.version}`);
    console.log(`seed ${profile.seed} · offline score ${offline.score}/100`);
    console.log(profile.navigator.userAgent);
  }

  const { browser, context, page } = await veneer.launchWithProfile(playwright, profile, { headless: Boolean(values.headless) });
  const results = [];
  try {
    for (const [index, site] of sites.entries()) {
      const tab = index === 0 ? page : await veneer.newPage(context, profile);
      try {
        await tab.goto(site, { waitUntil: 'domcontentloaded', timeout: 60_000 });
        const report = await veneer.verifyPage(tab, profile);
        results.push({ site, report });
        if (!values.json) printReport(site, report);
      } catch (err) {
        results.push({ site, error: String(err.message ?? err) });
        if (!values.json) console.log(`\n${site}\n  could not check: ${err.message ?? err}`);
      }
    }
    if (values.json) console.log(JSON.stringify({ profile, offline, results }, null, 2));
    if (!values.headless) {
      console.log('\nBrowser is open for manual inspection. Close it to exit.');
      await new Promise((resolve) => browser.on('disconnected', resolve));
    }
  } finally {
    if (browser.isConnected()) await browser.close();
  }
  const failed = !offline.valid || results.some((r) => r.error || !r.report.valid);
  return failed ? 1 : 0;
}

main().then(
  (code) => { process.exitCode = code; },
  (err) => {
    console.error(`error: ${err.message ?? err}`);
    process.exitCode = 1;
  },
);
