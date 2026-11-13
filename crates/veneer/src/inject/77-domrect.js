// DOMRect sub-pixel noise (at most 1e-4 px), keyed on the canvas seed and the value itself: equal
// values stay equal, zero stays zero (visibility checks), and layout itself is never touched.
const rectSeed = P.canvas.seed ^ 0x2d3c;
const jitter = (v, salt) =>
  v === 0 || !Number.isFinite(v) ? v : v + ((hash(rectSeed ^ salt, Math.round(v * 64) | 0) & 0xff) / 0xff - 0.5) * 2e-4;
const touch = (r) => {
  if (r) {
    r.x = jitter(r.x, 1);
    r.y = jitter(r.y, 2);
    r.width = jitter(r.width, 3);
    r.height = jitter(r.height, 4);
  }
  return r;
};
for (const C of [globalThis.Element, globalThis.Range]) {
  if (!C) continue;
  spoofMethod(C.prototype, 'getBoundingClientRect', (native) => ({
    getBoundingClientRect() {
      return touch(native.apply(this, arguments));
    },
  }).getBoundingClientRect);
  spoofMethod(C.prototype, 'getClientRects', (native) => ({
    getClientRects() {
      const list = native.apply(this, arguments);
      for (let i = 0; i < list.length; i += 1) touch(list[i]);
      return list;
    },
  }).getClientRects);
}
