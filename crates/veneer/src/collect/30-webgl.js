const gl = pick(() => document.createElement('canvas').getContext('webgl'));
if (gl) {
  const ext = gl.getExtension('WEBGL_debug_renderer_info');
  out.webgl = {
    vendor: gl.getParameter(gl.VENDOR),
    renderer: gl.getParameter(gl.RENDERER),
    unmaskedVendor: ext ? gl.getParameter(ext.UNMASKED_VENDOR_WEBGL) : null,
    unmaskedRenderer: ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : null,
  };
}
