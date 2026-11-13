// Values on existing objects: connection, battery, storage quota, heap limit, audio latency,
// keyboard layout, orientation, window geometry, notification permission; removal of APIs the
// profile's browser does not have. navigator.webdriver is deliberately left alone.
const X = P.navapi;
if (X) {
  const drop = (obj, key) => {
    const [owner] = findDescriptor(obj, key);
    if (owner) delete owner[key];
  };
  const getterFrom = (proto, key, compute) => {
    const d = proto && Object.getOwnPropertyDescriptor(proto, key);
    if (!d || !d.get) return;
    const nativeGet = d.get;
    const holder = {
      get [key]() {
        return compute.call(this, nativeGet.call(this));
      },
    };
    Object.defineProperty(proto, key, { ...d, get: disguise(Object.getOwnPropertyDescriptor(holder, key).get, `get ${key}`) });
  };

  const C = X.connection;
  if (C && navigator.connection) {
    for (const key of ['effectiveType', 'rtt', 'downlink', 'saveData']) spoofGetter(navigator.connection, key, C[key]);
  }

  const B = X.battery;
  if (B && typeof BatteryManager !== 'undefined') {
    const proto = BatteryManager.prototype;
    spoofGetter(proto, 'charging', B.charging);
    spoofGetter(proto, 'chargingTime', B.chargingTime ?? Infinity);
    spoofGetter(proto, 'dischargingTime', B.dischargingTime ?? Infinity);
    spoofGetter(proto, 'level', B.level);
  }

  if (typeof X.storageQuota === 'number' && typeof StorageManager !== 'undefined') {
    spoofMethod(StorageManager.prototype, 'estimate', (native) => ({
      estimate() {
        return native.apply(this, arguments).then((e) => {
          e.quota = X.storageQuota;
          return e;
        });
      },
    }).estimate);
  }

  if (typeof X.jsHeapSizeLimit === 'number' && performance.memory) {
    spoofGetter(Object.getPrototypeOf(performance.memory), 'jsHeapSizeLimit', X.jsHeapSizeLimit);
  }

  const A = X.audio;
  if (A && typeof AudioContext !== 'undefined') {
    getterFrom(AudioContext.prototype, 'baseLatency', function () {
      return typeof A.baseLatency === 'number' ? A.baseLatency : A.baseLatencyFrames / this.sampleRate;
    });
  }

  // getLayoutMap: the locale's layout, iterated in the order Chromium itself uses.
  const K = X.keyboardLayout;
  if (K && typeof KeyboardLayoutMap !== 'undefined') {
    const proto = KeyboardLayoutMap.prototype;
    const sizeGet = Object.getOwnPropertyDescriptor(proto, 'size').get;
    const nativeKeys = proto.keys;
    // Chromium's iteration order for these codes, observed on Windows.
    const ORDER = 'KeyK,KeyG,Digit2,Digit0,KeyV,KeyA,Backquote,KeyL,IntlBackslash,Quote,KeyW,Digit8,KeyM,KeyH,Period,Digit7,Digit1,KeyP,KeyD,KeyF,KeyO,KeyQ,KeyC,KeyN,BracketLeft,KeyZ,KeyY,Digit3,Digit6,Digit5,KeyX,Slash,Backslash,Comma,Minus,Digit4,KeyB,KeyT,Digit9,KeyS,KeyI,KeyU,Equal,KeyJ,Semicolon,KeyR,BracketRight,KeyE'.split(',');
    let layout = null;
    const map = (self) => {
      sizeGet.call(self);
      if (!layout) {
        const seen = [...nativeKeys.call(self)];
        const order = [...(seen.length ? seen : ORDER), ...Object.keys(K)];
        layout = new Map(order.filter((k, i) => k in K && order.indexOf(k) === i).map((k) => [k, K[k]]));
      }
      return layout;
    };
    getterFrom(proto, 'size', function () {
      return map(this).size;
    });
    const methods = {
      get(key) {
        return map(this).get(key);
      },
      has(key) {
        return map(this).has(key);
      },
      keys() {
        return map(this).keys();
      },
      values() {
        return map(this).values();
      },
      entries() {
        return map(this).entries();
      },
      forEach(callback) {
        const self = this;
        map(this).forEach((v, k) => callback.call(arguments[1], v, k, self));
      },
    };
    const iterIsEntries = proto[Symbol.iterator] === proto.entries;
    for (const key of Object.keys(methods)) spoofMethod(proto, key, () => methods[key]);
    if (iterIsEntries) Object.defineProperty(proto, Symbol.iterator, { ...Object.getOwnPropertyDescriptor(proto, Symbol.iterator), value: proto.entries });
  }

  const O = X.orientation;
  if (O && screen.orientation) {
    spoofGetter(screen.orientation, 'type', O.type);
    spoofGetter(screen.orientation, 'angle', O.angle);
  }
  if (O && !X.absent.includes('window.orientation')) {
    if (findDescriptor(window, 'orientation')[0]) {
      spoofGetter(window, 'orientation', O.angle);
    } else {
      // Desktop hosts have no window.orientation; mobile browsers expose it as a window accessor.
      const holder = { get orientation() { return O.angle; } };
      const get = disguise(Object.getOwnPropertyDescriptor(holder, 'orientation').get, 'get orientation');
      Object.defineProperty(window, 'orientation', { get, set: undefined, enumerable: true, configurable: true });
    }
  }

  const W = X.window;
  if (W) {
    for (const [key, value] of [
      ['outerWidth', W.outerWidth],
      ['outerHeight', W.outerHeight],
      ['screenX', W.screenX],
      ['screenY', W.screenY],
      ['screenLeft', W.screenX],
      ['screenTop', W.screenY],
    ]) {
      spoofGetter(window, key, value);
    }
  }

  if (X.notificationPermission && typeof Notification !== 'undefined') {
    spoofGetter(Notification, 'permission', X.notificationPermission);
  }

  for (const name of X.absent) {
    if (name === 'performance.memory') drop(performance, 'memory');
    else if (name === 'screen.orientation') drop(screen, 'orientation');
    else if (name === 'window.orientation') {
      drop(window, 'orientation');
      drop(window, 'onorientationchange');
    } else if (name === 'Notification') delete window.Notification;
    else drop(navigator, name);
  }
}
