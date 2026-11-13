// Media queries, WebGL capabilities, WebGPU presence and measurement-based font detection.
const mq = (q) => matchMedia(q).matches;
const g = { media: null, webgl: null, webgpu: null, fonts: {} };
g.media = {
  pointer: mq('(pointer: coarse)') ? 'coarse' : mq('(pointer: fine)') ? 'fine' : 'none',
  hover: mq('(hover: hover)') ? 'hover' : 'none',
  colorScheme: mq('(prefers-color-scheme: dark)') ? 'dark' : 'light',
  resolution: mq(`(resolution: ${devicePixelRatio}dppx)`) ? devicePixelRatio : null,
  deviceWidth: mq(`(device-width: ${screen.width}px)`) ? screen.width : null,
  deviceHeight: mq(`(device-height: ${screen.height}px)`) ? screen.height : null,
  colorGamut: mq('(color-gamut: rec2020)') ? 'rec2020' : mq('(color-gamut: p3)') ? 'p3' : 'srgb',
};
const glNames = [
  'MAX_TEXTURE_SIZE', 'MAX_CUBE_MAP_TEXTURE_SIZE', 'MAX_RENDERBUFFER_SIZE', 'MAX_VIEWPORT_DIMS', 'MAX_VERTEX_ATTRIBS',
  'MAX_VERTEX_UNIFORM_VECTORS', 'MAX_FRAGMENT_UNIFORM_VECTORS', 'MAX_VARYING_VECTORS', 'MAX_TEXTURE_IMAGE_UNITS',
  'MAX_VERTEX_TEXTURE_IMAGE_UNITS', 'MAX_COMBINED_TEXTURE_IMAGE_UNITS', 'ALIASED_LINE_WIDTH_RANGE', 'ALIASED_POINT_SIZE_RANGE',
  'SUBPIXEL_BITS', 'MAX_3D_TEXTURE_SIZE', 'MAX_ARRAY_TEXTURE_LAYERS', 'MAX_COLOR_ATTACHMENTS', 'MAX_DRAW_BUFFERS',
  'MAX_ELEMENTS_INDICES', 'MAX_ELEMENTS_VERTICES', 'MAX_FRAGMENT_INPUT_COMPONENTS', 'MAX_FRAGMENT_UNIFORM_BLOCKS',
  'MAX_FRAGMENT_UNIFORM_COMPONENTS', 'MAX_PROGRAM_TEXEL_OFFSET', 'MIN_PROGRAM_TEXEL_OFFSET', 'MAX_SERVER_WAIT_TIMEOUT',
  'MAX_TEXTURE_LOD_BIAS', 'MAX_TRANSFORM_FEEDBACK_INTERLEAVED_COMPONENTS', 'MAX_TRANSFORM_FEEDBACK_SEPARATE_ATTRIBS',
  'MAX_TRANSFORM_FEEDBACK_SEPARATE_COMPONENTS', 'MAX_UNIFORM_BLOCK_SIZE', 'MAX_UNIFORM_BUFFER_BINDINGS', 'MAX_VARYING_COMPONENTS',
  'MAX_VERTEX_OUTPUT_COMPONENTS', 'MAX_VERTEX_UNIFORM_BLOCKS', 'MAX_VERTEX_UNIFORM_COMPONENTS', 'MAX_COMBINED_UNIFORM_BLOCKS',
  'MAX_COMBINED_VERTEX_UNIFORM_COMPONENTS', 'MAX_COMBINED_FRAGMENT_UNIFORM_COMPONENTS', 'UNIFORM_BUFFER_OFFSET_ALIGNMENT',
];
const gl1 = pick(() => document.createElement('canvas').getContext('webgl'));
const gl2 = pick(() => document.createElement('canvas').getContext('webgl2'));
if (gl1) {
  const params = {};
  for (const name of glNames) {
    const ctx = typeof WebGLRenderingContext[name] === 'number' ? gl1 : gl2;
    if (!ctx) continue;
    const v = pick(() => ctx.getParameter(ctx[name]));
    params[name] = v !== null && typeof v === 'object' && v.length !== undefined ? [...v] : v;
  }
  const precision = {};
  for (const s of ['VERTEX_SHADER', 'FRAGMENT_SHADER']) {
    for (const p of ['LOW_FLOAT', 'MEDIUM_FLOAT', 'HIGH_FLOAT', 'LOW_INT', 'MEDIUM_INT', 'HIGH_INT']) {
      const f = pick(() => gl1.getShaderPrecisionFormat(gl1[s], gl1[p]));
      precision[`${s}.${p}`] = f ? [f.rangeMin, f.rangeMax, f.precision] : null;
    }
  }
  g.webgl = {
    params,
    precision,
    extensions: { webgl: pick(() => gl1.getSupportedExtensions()), webgl2: gl2 ? pick(() => gl2.getSupportedExtensions()) : null },
  };
}
g.webgpu = { present: 'gpu' in navigator, vendor: null, architecture: null };
if (navigator.gpu) {
  const adapter = await Promise.race([
    navigator.gpu.requestAdapter().catch(() => null),
    new Promise((resolve) => setTimeout(() => resolve(null), 3000)),
  ]);
  if (adapter && adapter.info) {
    g.webgpu.vendor = adapter.info.vendor;
    g.webgpu.architecture = adapter.info.architecture;
  }
}
// Keep this list equal to FONT_PROBES in src/extras/graphics.rs.
const fontProbes = [
  'Arial', 'Calibri', 'Segoe UI', 'Segoe UI Variable', 'Cascadia Code', 'Bahnschrift', 'Consolas', 'Cambria', 'Candara',
  'Helvetica Neue', 'Menlo', 'Monaco', 'Avenir', 'Futura', 'Gill Sans', 'Optima', 'Ubuntu', 'DejaVu Sans',
  'Liberation Sans', 'Noto Sans', 'Roboto', 'Cantarell', 'Arial Nova', 'Courier New',
];
const fontCtx = pick(() => document.createElement('canvas').getContext('2d'));
if (fontCtx) {
  const text = 'mmMwWLliI0fiflO&1';
  const width = (family) => {
    fontCtx.font = `72px ${family}`;
    return fontCtx.measureText(text).width;
  };
  const bases = ['monospace', 'serif', 'sans-serif'].map((b) => [b, width(b)]);
  for (const f of fontProbes) {
    g.fonts[f] = bases.some(([b, w]) => width(`"${f}", ${b}`) !== w);
  }
}
out.graphics = g;
