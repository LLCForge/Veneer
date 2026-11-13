'use strict';

// Default config inside the image: specs in /app/tests use `require('@llcforge/veneer-playwright')`.
module.exports = {
  testDir: './tests',
  reporter: 'list',
  use: { headless: true },
  projects: [
    { name: 'chromium', use: { browserName: 'chromium' } },
    { name: 'firefox', use: { browserName: 'firefox' } },
    { name: 'webkit', use: { browserName: 'webkit' } },
  ],
};
