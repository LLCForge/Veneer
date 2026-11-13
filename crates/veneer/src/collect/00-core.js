const pick = (fn) => {
  try {
    const value = fn();
    return value === undefined ? null : value;
  } catch {
    return null;
  }
};
const n = navigator;
const out = {
  userAgent: n.userAgent,
  appVersion: n.appVersion,
  platform: n.platform,
  vendor: n.vendor,
  language: n.language,
  languages: [...n.languages],
  hardwareConcurrency: n.hardwareConcurrency,
  deviceMemory: pick(() => n.deviceMemory),
  maxTouchPoints: n.maxTouchPoints,
  oscpu: pick(() => n.oscpu),
  webdriver: pick(() => n.webdriver),
  screen: {
    width: screen.width,
    height: screen.height,
    availWidth: screen.availWidth,
    availHeight: screen.availHeight,
    colorDepth: screen.colorDepth,
    pixelDepth: screen.pixelDepth,
  },
  devicePixelRatio: window.devicePixelRatio,
  timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
  webgl: null,
  userAgentData: null,
};
