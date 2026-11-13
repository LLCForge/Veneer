import { defineConfig } from '@playwright/test';

const port = Number(process.env.E2E_PORT ?? 8813);

// Each project runs only the profiles of its own engine (tagged @chromium, @firefox, @webkit).
export default defineConfig({
  testDir: '.',
  testMatch: '**/*.spec.mjs',
  timeout: 60_000,
  forbidOnly: !!process.env.CI,
  reporter: process.env.CI ? [['list'], ['github']] : 'list',
  projects: ['chromium', 'firefox', 'webkit'].map((name) => ({ name, grep: new RegExp(`@${name}(?!\\w)`) })),
  webServer: {
    command: 'node server.mjs',
    url: `http://127.0.0.1:${port}/health`,
    reuseExistingServer: !process.env.CI,
    env: { E2E_PORT: String(port) },
  },
});
