const pickAsync = async (fn) => {
  try {
    const value = await fn();
    return value === undefined ? null : value;
  } catch {
    return null;
  }
};
const has = (obj, key) => {
  for (let o = obj; o; o = Object.getPrototypeOf(o)) if (Object.getOwnPropertyDescriptor(o, key)) return true;
  return false;
};
const conn = pick(() => n.connection);
const mem = pick(() => performance.memory);
// Offset and wall-clock hour of the resolved timezone, computed through Intl only.
const zoneNow = (() => {
  const t = Date.now();
  const p = {};
  const f = new Intl.DateTimeFormat('en-US', { timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone, hourCycle: 'h23', year: 'numeric', month: 'numeric', day: 'numeric', hour: 'numeric', minute: 'numeric', second: 'numeric' });
  for (const { type, value } of f.formatToParts(t)) p[type] = value;
  const offset = Math.round((Date.UTC(+p.year, p.month - 1, +p.day, +p.hour, +p.minute, +p.second) - (t - (t % 1000))) / 60000);
  return { date: new Date(t), offset, hour: +p.hour };
})();
out.navapi = {
  mediaDevices: await pickAsync(() => n.mediaDevices.enumerateDevices().then((list) => list.map((d) => d.kind))),
  pdfViewerEnabled: pick(() => n.pdfViewerEnabled),
  plugins: pick(() => [...n.plugins].map((p) => p.name)),
  mimeTypes: pick(() => [...n.mimeTypes].map((m) => m.type)),
  connection: conn ? { effectiveType: conn.effectiveType, rtt: conn.rtt, downlink: conn.downlink, saveData: conn.saveData } : null,
  battery: await pickAsync(() =>
    n.getBattery().then((b) => ({
      charging: b.charging,
      chargingTime: Number.isFinite(b.chargingTime) ? b.chargingTime : null,
      dischargingTime: Number.isFinite(b.dischargingTime) ? b.dischargingTime : null,
      level: b.level,
    })),
  ),
  storageQuota: await pickAsync(() => n.storage.estimate().then((e) => e.quota)),
  jsHeapSizeLimit: mem ? mem.jsHeapSizeLimit : null,
  audio: await pickAsync(async () => {
    const ac = new AudioContext();
    const r = { sampleRate: ac.sampleRate, baseLatency: ac.baseLatency, baseLatencyFrames: Math.round(ac.baseLatency * ac.sampleRate) };
    await ac.close();
    return r;
  }),
  keyboardLayout: await pickAsync(() => n.keyboard.getLayoutMap().then((m) => Object.fromEntries(m))),
  voices: pick(() => speechSynthesis.getVoices().map((v) => ({ name: v.name, lang: v.lang, localService: v.localService, default: v.default }))),
  orientation: pick(() => ({ type: screen.orientation.type, angle: screen.orientation.angle })),
  windowOrientation: pick(() => window.orientation),
  window: { outerWidth: window.outerWidth, outerHeight: window.outerHeight, screenX: window.screenX, screenY: window.screenY },
  notificationPermission: pick(() => Notification.permission),
  notificationQuery: await pickAsync(() => n.permissions.query({ name: 'notifications' }).then((s) => s.state)),
  present: {
    bluetooth: has(n, 'bluetooth'),
    connection: has(n, 'connection'),
    getBattery: has(n, 'getBattery'),
    hid: has(n, 'hid'),
    keyboard: has(n, 'keyboard'),
    Notification: 'Notification' in window,
    'performance.memory': has(performance, 'memory'),
    'screen.orientation': has(screen, 'orientation'),
    serial: has(n, 'serial'),
    usb: has(n, 'usb'),
    userActivation: has(n, 'userActivation'),
    'window.orientation': 'orientation' in window,
  },
  timezoneConsistent: zoneNow.date.getTimezoneOffset() === -zoneNow.offset && zoneNow.date.getHours() === zoneNow.hour,
};
