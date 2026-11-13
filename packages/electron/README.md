# @llcforge/veneer-electron

Apply a [Veneer](../veneer) profile to an [Electron](https://www.electronjs.org) `webContents` with Electron's
built-in debugger (no extra dependencies, works in packaged apps).

```js
const { app, BrowserWindow, session } = require('electron');
const veneer = require('@llcforge/veneer');
const { applyToWebContents, applyToElectronSession, commandLineSwitches } = require('@llcforge/veneer-electron');

const profile = veneer.generate({ os: 'windows', browser: 'chrome', locale: 'tr-TR' });
for (const [name, value] of commandLineSwitches(profile)) app.commandLine.appendSwitch(name, value);

app.whenReady().then(async () => {
  applyToElectronSession(session.defaultSession, profile);
  const win = new BrowserWindow({ width: profile.screen.availWidth, height: profile.screen.availHeight });
  const detach = await applyToWebContents(win.webContents, profile); // before loadURL
  await win.loadURL('https://example.com');
});
```

- `applyToWebContents` attaches `webContents.debugger` (protocol 1.3) and sends `veneer.cdpCommands(profile)`:
  user agent + client hints, `Accept-Language`, timezone, locale, device metrics, touch and the init script. The
  overrides stay while the debugger is attached; call the returned function to detach. If a command fails, a
  debugger this call attached is detached again and the error is rethrown.
- `applyToElectronSession` sets the session-wide user agent and `Accept-Language` (workers, `net` requests).
- `commandLineSwitches` gives `veneer.chromiumArgs` as `appendSwitch` pairs (`lang`, `user-agent`, `window-size`,
  `force-device-scale-factor`); call it before `app.whenReady()`.

Only Chromium profiles are accepted. Automation markers are not hidden.

Tests: `npm test` runs with a stand-in `webContents`. Electron itself was not launched in these tests.
