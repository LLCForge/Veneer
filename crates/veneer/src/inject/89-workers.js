// Web Workers and SharedWorkers: the constructor gets a same-origin blob script that applies the
// registered parts first, then loads the original script (importScripts, or import() for module
// workers). Relative URLs inside the worker still resolve against the original script.
// Service Workers cannot be covered: navigator.serviceWorker.register() rejects blob: URLs.
// Shared workers are cached per document; two tabs no longer share one instance.
const makeCore = function () {
  const nativeToString = Function.prototype.toString;
  const disguised = new WeakMap();
  const disguise = (fn, name) => {
    disguised.set(fn, name);
    return fn;
  };
  const toString = {
    toString() {
      return disguised.has(this) ? `function ${disguised.get(this)}() { [native code] }` : nativeToString.call(this);
    },
  }.toString;
  Object.defineProperty(Function.prototype, 'toString', {
    ...Object.getOwnPropertyDescriptor(Function.prototype, 'toString'),
    value: disguise(toString, 'toString'),
  });
  const findDescriptor = (obj, key) => {
    for (let o = obj; o; o = Object.getPrototypeOf(o)) {
      const d = Object.getOwnPropertyDescriptor(o, key);
      if (d) return [o, d];
    }
    return [null, null];
  };
  const spoofGetter = (obj, key, value) => {
    if (value === undefined || value === null || !obj) return;
    const [owner, d] = findDescriptor(obj, key);
    if (!owner || !d.get) return;
    const nativeGet = d.get;
    const holder = {
      get [key]() {
        nativeGet.call(this);
        return value;
      },
    };
    Object.defineProperty(owner, key, { ...d, get: disguise(Object.getOwnPropertyDescriptor(holder, key).get, `get ${key}`) });
  };
  const spoofMethod = (owner, key, make) => {
    const d = owner && Object.getOwnPropertyDescriptor(owner, key);
    if (!d || typeof d.value !== 'function') return;
    Object.defineProperty(owner, key, { ...d, value: disguise(make(d.value), key) });
  };
  const hash = (seed, n) => {
    let t = (seed ^ Math.imul(n, 0x9e3779b1)) | 0;
    t = Math.imul(t ^ (t >>> 16), 0x85ebca6b);
    t = Math.imul(t ^ (t >>> 13), 0xc2b2ae35);
    return (t ^ (t >>> 16)) >>> 0;
  };
  const wrapGetter = (owner, key, map) => {
    const d = owner && Object.getOwnPropertyDescriptor(owner, key);
    if (!d || !d.get) return;
    const nativeGet = d.get;
    const holder = {
      get [key]() {
        return map.call(this, nativeGet.call(this));
      },
    };
    Object.defineProperty(owner, key, { ...d, get: disguise(Object.getOwnPropertyDescriptor(holder, key).get, `get ${key}`) });
  };
  return { spoofGetter, spoofMethod, disguise, findDescriptor, hash, wrapGetter };
};

// Runs in the worker after the parts: point location and relative URLs back at the original script.
const fixBase = function (H, base) {
  const url = new URL(base);
  const L = globalThis.WorkerLocation;
  if (L) {
    for (const key of ['href', 'origin', 'protocol', 'host', 'hostname', 'port', 'pathname', 'search', 'hash']) {
      H.spoofGetter(L.prototype, key, url[key]);
    }
    H.spoofMethod(L.prototype, 'toString', () => ({
      toString() {
        return url.href;
      },
    }).toString);
  }
  const resolve = (u) => {
    if (typeof u !== 'string' && !(u instanceof URL)) return u;
    try {
      return new URL(String(u), url).href;
    } catch {
      return u;
    }
  };
  // importScripts and fetch live on WorkerGlobalScope.prototype, not on the global object itself.
  const [scope] = H.findDescriptor(globalThis, 'importScripts');
  H.spoofMethod(scope, 'importScripts', (native) => ({
    importScripts() {
      return native.apply(this, [...arguments].map(resolve));
    },
  }).importScripts);
  H.spoofMethod(H.findDescriptor(globalThis, 'fetch')[0], 'fetch', (native) => ({
    fetch(input) {
      const args = [...arguments];
      args[0] = resolve(input);
      return native.apply(this, args);
    },
  }).fetch);
  const X = globalThis.XMLHttpRequest;
  if (X) {
    H.spoofMethod(X.prototype, 'open', (native) => ({
      open(method, u) {
        const args = [...arguments];
        args[1] = resolve(u);
        return native.apply(this, args);
      },
    }).open);
  }
};

const partSources = veneerParts.map((p) => `(${nativeToString.call(p)})(P, H, true);`).join('\n');
// `inline` is the source of a classic blob: worker. Embedding it avoids importScripts(blob:), which a
// page CSP without blob: in script-src blocks inside the worker.
const boot = (href, isModule, inline) =>
  `(() => {\n'use strict';\nconst P = ${JSON.stringify(P)};\nconst H = (${nativeToString.call(makeCore)})();\n` +
  `try {\n${partSources}\n(${nativeToString.call(fixBase)})(H, ${JSON.stringify(href)});\n} catch {}\n})();\n` +
  (inline !== undefined
    ? `${inline}\n`
    : isModule
      ? `await import(${JSON.stringify(href)});\n`
      : `importScripts(${JSON.stringify(href)});\n`);

// A blob URL created by this document can be read synchronously; otherwise undefined.
const readBlobSource = (href) => {
  try {
    const xhr = new XMLHttpRequest();
    xhr.open('GET', href, false);
    xhr.send();
    return xhr.status === 200 || xhr.status === 0 ? xhr.responseText : undefined;
  } catch {
    return undefined;
  }
};

const blobs = new Map();
const wrapWorker = (name) => {
  const Native = globalThis[name];
  if (typeof Native !== 'function' || !globalThis.Blob || !globalThis.URL || !globalThis.location) return;
  const Wrapped = new Proxy(Native, {
    construct(target, args, newTarget) {
      const nt = newTarget === Wrapped ? target : newTarget;
      let url;
      try {
        url = new URL(String(args[0]), globalThis.location.href);
      } catch {
        return Reflect.construct(target, args, nt);
      }
      // Cross-origin and data: workers keep their native behaviour (errors, opaque origin).
      if (url.origin !== globalThis.location.origin || url.protocol === 'data:') return Reflect.construct(target, args, nt);
      const options = args[1];
      const isModule = options !== null && typeof options === 'object' && options.type === 'module';
      const key = `${name}|${isModule}|${url.href}`;
      let blobUrl = blobs.get(key);
      if (!blobUrl) {
        const classicBlob = url.protocol === 'blob:' && !isModule;
        const inline = classicBlob ? readBlobSource(url.href) : undefined;
        if (classicBlob && inline === undefined) return Reflect.construct(target, args, nt);
        blobUrl = URL.createObjectURL(new Blob([boot(url.href, isModule, inline)], { type: 'text/javascript' }));
        // Page blob URLs are usually single-use; caching them would pin their source.
        if (!classicBlob) blobs.set(key, blobUrl);
      }
      try {
        return Reflect.construct(target, [blobUrl, ...args.slice(1)], nt);
      } catch {
        // A CSP without blob: in worker-src: run the original script unpatched.
        return Reflect.construct(target, args, nt);
      }
    },
  });
  disguise(Wrapped, name);
  const ctor = Object.getOwnPropertyDescriptor(Native.prototype, 'constructor');
  if (ctor) Object.defineProperty(Native.prototype, 'constructor', { ...ctor, value: Wrapped });
  const global = Object.getOwnPropertyDescriptor(globalThis, name);
  if (global) Object.defineProperty(globalThis, name, { ...global, value: Wrapped });
};
wrapWorker('Worker');
wrapWorker('SharedWorker');
