# Veneer profile tester (Manifest V3 extension)

Apply a [Veneer](../veneer) profile to a tab in **your own** Chromium-based browser (Chrome, Edge, Brave …) to see
how a site behaves under that fingerprint. For testing your own sites; it is not published to any store.

## Install

1. `chrome://extensions` → enable **Developer mode** → **Load unpacked** → choose `packages/extension`.
2. Create a bundle (needs the npm package built, see `packages/veneer`):

   ```bash
   node packages/extension/bundle.js --os macos --locale tr-TR > bundle.json
   ```

   Flags: `--os --browser --kind --device --locale --timezone --country --seed` (the engine is always Chromium).
3. Open the site, click the extension, paste `bundle.json`, press **Apply to this tab**. The tab reloads with the
   profile. **Remove** detaches and reloads.

## How it works

The bundle is `{ profile: { seed, device, browser }, commands }`, where `commands` is
`veneer.cdpCommands(profile)`. The service worker attaches `chrome.debugger` (protocol 1.3) to the active tab and
sends the commands: user agent and client hints, `Accept-Language`, timezone, locale, device metrics, touch and the
init script. Chrome shows its "is being debugged" bar while the profile is active; that is expected and not hidden.

Safety:

- Only the seven commands `cdpCommands` produces are accepted (no `Runtime.evaluate`, no network or storage
  commands); at most 16 commands and 2 MB per bundle.
- Only the extension's own pages can send messages (`sender.id` check, no `externally_connectable`).
- Permissions: `debugger` and `storage` (session storage, to remember which tabs it attached to). No host permissions.
- Applying again starts a fresh debugger session, so the previous profile's overrides and init script are dropped.

The popup is in English and Turkish (`_locales`), following the browser language.

Tests: `npm test`. With `VENEER_E2E=1` and `VENEER_API`, Chromium (Playwright, `--load-extension`) loads the
extension, the test applies a bundle through the service worker, checks that a disallowed command is refused,
re-applies, and verifies the page with the collector. The popup's buttons are not driven by the test.
