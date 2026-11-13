// Platform objects pages cannot construct (plugins, media devices, voices): fakes built on the
// real prototypes, whose accessors serve fake data and fall back to the native getter otherwise.
const X = P.navapi;
if (X) {
  const fakes = new WeakMap();
  const fake = (proto, data) => {
    const obj = Object.create(proto);
    fakes.set(obj, data);
    return obj;
  };
  const serveGetters = (proto, keys) => {
    for (const key of keys) {
      const d = proto && Object.getOwnPropertyDescriptor(proto, key);
      if (!d || !d.get) continue;
      const nativeGet = d.get;
      const holder = {
        get [key]() {
          const f = fakes.get(this);
          return f ? f[key] : nativeGet.call(this);
        },
      };
      Object.defineProperty(proto, key, { ...d, get: disguise(Object.getOwnPropertyDescriptor(holder, key).get, `get ${key}`) });
    }
  };
  // Array-likes: own indexed (enumerable) and named (non-enumerable) properties, like the real ones.
  const fakeList = (proto, items, nameOf, extra = {}) => {
    const byName = new Map(items.map((it) => [nameOf(it), it]));
    const list = fake(proto, { ...extra, length: items.length, items, byName });
    items.forEach((it, i) => Object.defineProperty(list, i, { value: it, enumerable: true, configurable: true }));
    for (const [name, it] of byName) Object.defineProperty(list, name, { value: it, configurable: true });
    return list;
  };
  const serveListMethods = (proto) => {
    spoofMethod(proto, 'item', (native) => ({
      item(index) {
        const f = fakes.get(this);
        return f ? f.items[index >>> 0] ?? null : native.apply(this, arguments);
      },
    }).item);
    spoofMethod(proto, 'namedItem', (native) => ({
      namedItem(name) {
        const f = fakes.get(this);
        return f ? f.byName.get(String(name)) ?? null : native.apply(this, arguments);
      },
    }).namedItem);
  };

  // navigator.plugins / mimeTypes: the HTML standard's fixed PDF list, or empty.
  if (typeof X.pdfViewerEnabled === 'boolean' && typeof PluginArray !== 'undefined') {
    spoofGetter(navigator, 'pdfViewerEnabled', X.pdfViewerEnabled);
    const want = X.pdfViewerEnabled ? 5 : 0;
    if (navigator.plugins.length !== want) {
      const PDF = 'Portable Document Format';
      const TYPES = ['application/pdf', 'text/pdf'];
      const mime = (type, enabledPlugin) => fake(MimeType.prototype, { type, suffixes: 'pdf', description: PDF, enabledPlugin });
      const names = want ? ['PDF Viewer', 'Chrome PDF Viewer', 'Chromium PDF Viewer', 'Microsoft Edge PDF Viewer', 'WebKit built-in PDF'] : [];
      const plugins = names.map((name) => {
        const holder = { plugin: null };
        const mimes = TYPES.map((t) => mime(t, null));
        holder.plugin = fakeList(Plugin.prototype, mimes, (m) => fakes.get(m).type, { name, filename: 'internal-pdf-viewer', description: PDF });
        for (const m of mimes) fakes.get(m).enabledPlugin = holder.plugin;
        return holder.plugin;
      });
      const pluginArray = fakeList(PluginArray.prototype, plugins, (p) => fakes.get(p).name);
      const mimeArray = fakeList(MimeTypeArray.prototype, want ? TYPES.map((t) => mime(t, plugins[0])) : [], (m) => fakes.get(m).type);
      serveGetters(PluginArray.prototype, ['length']);
      serveGetters(MimeTypeArray.prototype, ['length']);
      serveGetters(Plugin.prototype, ['name', 'filename', 'description', 'length']);
      serveGetters(MimeType.prototype, ['type', 'suffixes', 'description', 'enabledPlugin']);
      for (const proto of [PluginArray.prototype, MimeTypeArray.prototype, Plugin.prototype]) serveListMethods(proto);
      spoofMethod(PluginArray.prototype, 'refresh', (native) => ({
        refresh() {
          return fakes.has(this) ? undefined : native.apply(this, arguments);
        },
      }).refresh);
      spoofGetter(navigator, 'plugins', pluginArray);
      spoofGetter(navigator, 'mimeTypes', mimeArray);
    }
  }

  // enumerateDevices before permission: one entry per kind, empty ids and labels.
  if (Array.isArray(X.mediaDevices) && typeof MediaDevices !== 'undefined' && typeof MediaDeviceInfo !== 'undefined') {
    const Input = typeof InputDeviceInfo !== 'undefined' ? InputDeviceInfo.prototype : MediaDeviceInfo.prototype;
    serveGetters(MediaDeviceInfo.prototype, ['deviceId', 'kind', 'label', 'groupId']);
    spoofMethod(MediaDeviceInfo.prototype, 'toJSON', (native) => ({
      toJSON() {
        const f = fakes.get(this);
        return f ? { deviceId: f.deviceId, kind: f.kind, label: f.label, groupId: f.groupId } : native.apply(this, arguments);
      },
    }).toJSON);
    if (Input !== MediaDeviceInfo.prototype) {
      spoofMethod(Input, 'getCapabilities', (native) => ({
        getCapabilities() {
          return fakes.has(this) ? {} : native.apply(this, arguments);
        },
      }).getCapabilities);
    }
    spoofMethod(MediaDevices.prototype, 'enumerateDevices', (native) => ({
      enumerateDevices() {
        return native.apply(this, arguments).then((list) => {
          // Once a capture permission exposes real labels, the real list is the right answer.
          if (list.some((d) => d.label)) return list;
          return X.mediaDevices.map(
            (kind) =>
              list.find((d) => d.kind === kind) ||
              fake(kind.endsWith('input') ? Input : MediaDeviceInfo.prototype, { deviceId: '', kind, label: '', groupId: '' }),
          );
        });
      },
    }).enumerateDevices);
  }

  // speechSynthesis.getVoices: the profile's list; utterances still get a real voice to speak with.
  if (Array.isArray(X.voices) && typeof SpeechSynthesis !== 'undefined' && typeof SpeechSynthesisVoice !== 'undefined') {
    const voices = X.voices.map((v) =>
      fake(SpeechSynthesisVoice.prototype, { voiceURI: v.name, name: v.name, lang: v.lang, localService: v.localService, default: v.default }),
    );
    serveGetters(SpeechSynthesisVoice.prototype, ['voiceURI', 'name', 'lang', 'localService', 'default']);
    const nativeGetVoices = SpeechSynthesis.prototype.getVoices;
    spoofMethod(SpeechSynthesis.prototype, 'getVoices', (native) => ({
      getVoices() {
        native.apply(this, arguments);
        return voices.slice();
      },
    }).getVoices);
    const U = typeof SpeechSynthesisUtterance !== 'undefined' && SpeechSynthesisUtterance.prototype;
    const d = U && Object.getOwnPropertyDescriptor(U, 'voice');
    if (d && d.get && d.set) {
      const chosen = new WeakMap();
      const holder = {
        get voice() {
          const v = d.get.call(this);
          return chosen.has(this) ? chosen.get(this) : v;
        },
        set voice(value) {
          if (fakes.has(value)) {
            const lang = fakes.get(value).lang;
            const real = nativeGetVoices.call(speechSynthesis);
            d.set.call(this, real.find((r) => r.lang === lang) || null);
            chosen.set(this, value);
          } else {
            d.set.call(this, value);
            chosen.delete(this);
          }
        },
      };
      const h = Object.getOwnPropertyDescriptor(holder, 'voice');
      Object.defineProperty(U, 'voice', { ...d, get: disguise(h.get, 'get voice'), set: disguise(h.set, 'set voice') });
    }
  }
}
