// WebGL capability values for the profile's GPU class: limits, shader precision and extension lists.
// Extensions are only ever hidden, never invented: the list is the profile's list filtered to what the
// real context supports, so getExtension() agrees with getSupportedExtensions().
const part = (P, H) => {
  const W = P.graphics && P.graphics.webgl;
  if (!W) return;
  const PrecisionFormat = globalThis.WebGLShaderPrecisionFormat;
  const fake = new WeakMap();
  if (PrecisionFormat) {
    ['rangeMin', 'rangeMax', 'precision'].forEach((key, i) => {
      H.wrapGetter(PrecisionFormat.prototype, key, function (v) {
        const f = fake.get(this);
        return f ? f[i] : v;
      });
    });
  }
  const contexts = [
    [globalThis.WebGLRenderingContext, W.extensions.webgl],
    [globalThis.WebGL2RenderingContext, W.extensions.webgl2],
  ];
  for (const [Ctx, list] of contexts) {
    if (!Ctx) continue;
    const proto = Ctx.prototype;
    const values = new Map();
    for (const [name, value] of Object.entries(W.params)) {
      if (typeof Ctx[name] === 'number') values.set(Ctx[name], value);
    }
    H.spoofMethod(proto, 'getParameter', (native) => ({
      getParameter(pname) {
        const real = native.apply(this, arguments);
        if (real === null || real === undefined || !values.has(pname)) return real;
        const v = values.get(pname);
        return ArrayBuffer.isView(real) ? new real.constructor(v) : v;
      },
    }).getParameter);
    const precisionKeys = new Map();
    for (const s of ['VERTEX_SHADER', 'FRAGMENT_SHADER']) {
      for (const p of ['LOW_FLOAT', 'MEDIUM_FLOAT', 'HIGH_FLOAT', 'LOW_INT', 'MEDIUM_INT', 'HIGH_INT']) {
        precisionKeys.set(`${Ctx[s]}:${Ctx[p]}`, `${s}.${p}`);
      }
    }
    H.spoofMethod(proto, 'getShaderPrecisionFormat', (native) => ({
      getShaderPrecisionFormat(shadertype, precisiontype) {
        const format = native.apply(this, arguments);
        const key = precisionKeys.get(`${shadertype}:${precisiontype}`);
        if (format && key && W.precision[key]) fake.set(format, W.precision[key]);
        return format;
      },
    }).getShaderPrecisionFormat);
    const allowed = new Set(list.map((e) => e.toLowerCase()));
    H.spoofMethod(proto, 'getSupportedExtensions', (native) => ({
      getSupportedExtensions() {
        const real = native.apply(this, arguments);
        if (!Array.isArray(real)) return real;
        const have = new Set(real);
        return list.filter((e) => have.has(e));
      },
    }).getSupportedExtensions);
    H.spoofMethod(proto, 'getExtension', (native) => ({
      getExtension(name) {
        const ext = native.apply(this, arguments);
        return ext && !allowed.has(String(name).toLowerCase()) ? null : ext;
      },
    }).getExtension);
  }
};
part(P, veneerH, false);
veneerParts.push(part);
