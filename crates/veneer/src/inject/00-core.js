// Patched functions report themselves as native code.
const nativeToString = Function.prototype.toString;
const disguised = new WeakMap();
const disguise = (fn, name) => {
  disguised.set(fn, name);
  return fn;
};
const toString = {
  toString() {
    return disguised.has(this)
      ? `function ${disguised.get(this)}() { [native code] }`
      : nativeToString.call(this);
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

// Redefine an accessor where the browser defines it, keeping the native receiver check.
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
  const get = Object.getOwnPropertyDescriptor(holder, key).get;
  Object.defineProperty(owner, key, { ...d, get: disguise(get, `get ${key}`) });
};

// `make` receives the native method and returns a method-shorthand function with the native arity.
const spoofMethod = (owner, key, make) => {
  const d = owner && Object.getOwnPropertyDescriptor(owner, key);
  if (!d || typeof d.value !== 'function') return;
  Object.defineProperty(owner, key, { ...d, value: disguise(make(d.value), key) });
};

// Stateless 32-bit mixer: same seed and input always give the same noise.
const hash = (seed, n) => {
  let t = (seed ^ Math.imul(n, 0x9e3779b1)) | 0;
  t = Math.imul(t ^ (t >>> 16), 0x85ebca6b);
  t = Math.imul(t ^ (t >>> 13), 0xc2b2ae35);
  return (t ^ (t >>> 16)) >>> 0;
};
