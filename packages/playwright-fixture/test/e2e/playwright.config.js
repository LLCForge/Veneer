'use strict';

// Real-browser check: VENEER_API=http://127.0.0.1:<port> npx playwright test -c test/e2e
module.exports = {
  testDir: __dirname,
  testMatch: '*.spec.js',
  reporter: 'list',
  projects: [{ name: 'chromium', use: { browserName: 'chromium' } }],
};
