// Timezone fallback for launches that did not set the browser's timezone (Playwright's
// timezoneId or CDP Emulation.setTimezoneOverride do it natively and make this a no-op).
// Covers Intl defaults, getTimezoneOffset, local-time getters and Date string forms; Date
// setters and the local-time Date constructor keep the host zone.
const TZ = P.timezone;
const NativeDTF = Intl.DateTimeFormat;
let supported = false;
try {
  supported = typeof TZ === 'string' && new NativeDTF('en-US', { timeZone: TZ }).resolvedOptions().timeZone !== undefined;
} catch {
  supported = false;
}
if (supported && new NativeDTF().resolvedOptions().timeZone !== TZ) {
  const parts = new NativeDTF('en-US', {
    timeZone: TZ,
    hourCycle: 'h23',
    year: 'numeric',
    month: 'numeric',
    day: 'numeric',
    hour: 'numeric',
    minute: 'numeric',
    second: 'numeric',
    era: 'short',
  });
  const nativeGetTime = Date.prototype.getTime;
  // Minutes east of UTC at instant t in the profile's zone.
  const offsetAt = (t) => {
    const p = {};
    for (const { type, value } of parts.formatToParts(t)) p[type] = value;
    const year = p.era === 'BC' || p.era === 'B' ? 1 - Number(p.year) : Number(p.year);
    const local = Date.UTC(year, Number(p.month) - 1, Number(p.day), Number(p.hour), Number(p.minute), Number(p.second));
    return Math.round((local - (t - (((t % 1000) + 1000) % 1000))) / 60000);
  };
  const shifted = (date) => {
    const t = nativeGetTime.call(date);
    return Number.isNaN(t) ? null : new Date(t + offsetAt(t) * 60000);
  };
  const withZone = (options) => (options === undefined ? { timeZone: TZ } : options !== null && typeof options === 'object' && options.timeZone === undefined ? { ...options, timeZone: TZ } : options);

  const DTF = new Proxy(NativeDTF, {
    construct(target, args, newTarget) {
      return Reflect.construct(target, [args[0], withZone(args[1])], newTarget);
    },
    apply(target, self, args) {
      return Reflect.apply(target, self, [args[0], withZone(args[1])]);
    },
  });
  disguise(DTF, 'DateTimeFormat');
  Object.defineProperty(NativeDTF.prototype, 'constructor', { ...Object.getOwnPropertyDescriptor(NativeDTF.prototype, 'constructor'), value: DTF });
  Object.defineProperty(Intl, 'DateTimeFormat', { ...Object.getOwnPropertyDescriptor(Intl, 'DateTimeFormat'), value: DTF });

  const D = Date.prototype;
  for (const key of ['toLocaleString', 'toLocaleDateString', 'toLocaleTimeString']) {
    spoofMethod(D, key, (native) => ({
      [key](locales, options) {
        return native.call(this, locales, withZone(options));
      },
    })[key]);
  }
  spoofMethod(D, 'getTimezoneOffset', (native) => ({
    getTimezoneOffset() {
      const t = nativeGetTime.call(this);
      return Number.isNaN(t) ? native.call(this) : -offsetAt(t);
    },
  }).getTimezoneOffset);
  for (const key of ['FullYear', 'Month', 'Date', 'Day', 'Hours', 'Minutes', 'Seconds', 'Milliseconds']) {
    const utc = D[`getUTC${key}`];
    spoofMethod(D, `get${key}`, (native) => ({
      [`get${key}`]() {
        const s = shifted(this);
        return s ? utc.call(s) : native.call(this);
      },
    })[`get${key}`]);
  }
  const pad = (n, w = 2) => String(Math.abs(n)).padStart(w, '0');
  const zoneName = new NativeDTF(undefined, { timeZone: TZ, timeZoneName: 'long' });
  const DAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
  const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  const datePart = (s) => {
    const y = s.getUTCFullYear();
    return `${DAYS[s.getUTCDay()]} ${MONTHS[s.getUTCMonth()]} ${pad(s.getUTCDate())} ${y < 0 ? '-' : ''}${pad(y, 4)}`;
  };
  const timePart = (date, s) => {
    const off = offsetAt(nativeGetTime.call(date));
    const name = (zoneName.formatToParts(date).find((p) => p.type === 'timeZoneName') || {}).value;
    return `${pad(s.getUTCHours())}:${pad(s.getUTCMinutes())}:${pad(s.getUTCSeconds())} GMT${off < 0 ? '-' : '+'}${pad(Math.trunc(off / 60))}${pad(off % 60)}${name ? ` (${name})` : ''}`;
  };
  const strings = {
    toString(s, date) {
      return `${datePart(s)} ${timePart(date, s)}`;
    },
    toDateString(s) {
      return datePart(s);
    },
    toTimeString(s, date) {
      return timePart(date, s);
    },
  };
  for (const key of Object.keys(strings)) {
    spoofMethod(D, key, (native) => ({
      [key]() {
        const s = shifted(this);
        return s ? strings[key](s, this) : native.call(this);
      },
    })[key]);
  }
}
