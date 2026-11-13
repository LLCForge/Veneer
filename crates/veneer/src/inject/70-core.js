// Graphics patches are written as `(P, H, inWorker) => {}` parts that use only their arguments and
// globals, so 89-workers.js can replay the same source inside Web Workers. `var` makes the registry
// and helper set visible to the later blocks of this script.
var veneerParts = [];
var veneerH = {
  spoofGetter,
  spoofMethod,
  disguise,
  findDescriptor,
  hash,
  // Replace an accessor's getter with `map(nativeValue)`, keeping the native receiver check.
  wrapGetter(owner, key, map) {
    const d = owner && Object.getOwnPropertyDescriptor(owner, key);
    if (!d || !d.get) return;
    const nativeGet = d.get;
    const holder = {
      get [key]() {
        return map.call(this, nativeGet.call(this));
      },
    };
    Object.defineProperty(owner, key, { ...d, get: disguise(Object.getOwnPropertyDescriptor(holder, key).get, `get ${key}`) });
  },
};
