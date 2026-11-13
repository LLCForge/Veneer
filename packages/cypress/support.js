/* global Cypress, before */
// Import from cypress/support/e2e.js: require('@llcforge/veneer-cypress/support')
'use strict';

const initScript = Cypress.env('veneerInitScript');
const cdp = Cypress.env('veneerCdp');

if (typeof initScript === 'string' && initScript.length > 0) {
  Cypress.on('window:before:load', (win) => {
    // Indirect eval runs the script in the app window's global scope, before the app's own scripts.
    win.eval(initScript);
  });
}

if (Array.isArray(cdp) && cdp.length > 0) {
  before(() =>
    cdp.reduce(
      (chain, { method, params }) => chain.then(() => Cypress.automation('remote:debugger:protocol', { command: method, params })),
      Promise.resolve(),
    ),
  );
}
