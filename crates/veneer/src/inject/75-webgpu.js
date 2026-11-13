// WebGPU: remove navigator.gpu and the GPU* interfaces where the profile's browser has no WebGPU;
// otherwise report the profile GPU's adapter vendor and architecture. Limits and features stay native.
const part = (P, H) => {
  const G = P.graphics && P.graphics.webgpu;
  if (!G) return;
  if (!G.present) {
    const [owner] = H.findDescriptor(globalThis.navigator, 'gpu');
    if (owner) delete owner.gpu;
    for (const key of Object.getOwnPropertyNames(globalThis)) {
      if (!/^GPU([A-Z]|$)/.test(key) && key !== 'WGSLLanguageFeatures') continue;
      try {
        delete globalThis[key];
      } catch {
        // Non-configurable in this engine; leave it.
      }
    }
    return;
  }
  const Info = globalThis.GPUAdapterInfo;
  if (!Info) return;
  if (G.vendor) H.spoofGetter(Info.prototype, 'vendor', G.vendor);
  if (G.architecture) H.spoofGetter(Info.prototype, 'architecture', G.architecture);
};
part(P, veneerH, false);
veneerParts.push(part);
