// WebGL: masked strings always, unmasked strings (or null when the device exposes none).
const G = P.webgl;
const params = new Map([
  [0x1f00, G.vendor],
  [0x1f01, G.renderer],
  [0x9245, G.unmaskedVendor ?? null],
  [0x9246, G.unmaskedRenderer ?? null],
]);
for (const Ctx of [globalThis.WebGLRenderingContext, globalThis.WebGL2RenderingContext]) {
  if (!Ctx) continue;
  spoofMethod(Ctx.prototype, 'getParameter', (native) => ({
    getParameter(pname) {
      const real = native.call(this, pname);
      return params.has(pname) ? params.get(pname) : real;
    },
  }).getParameter);
}
