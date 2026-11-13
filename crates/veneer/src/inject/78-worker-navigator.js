// Inside workers only: the navigator and WebGL string values that 10-navigator.js and 30-webgl.js
// apply to the page (those modules are not replayed in workers).
const part = (P, H, inWorker) => {
  if (!inWorker) return;
  const N = P.navigator;
  const nav = globalThis.navigator;
  for (const key of ['userAgent', 'appVersion', 'platform', 'vendor', 'language', 'hardwareConcurrency', 'deviceMemory']) {
    H.spoofGetter(nav, key, N[key]);
  }
  H.spoofGetter(nav, 'languages', Object.freeze([...N.languages]));
  if (!N.userAgentData) {
    const [owner] = H.findDescriptor(nav, 'userAgentData');
    if (owner) delete owner.userAgentData;
  } else if (nav.userAgentData && globalThis.NavigatorUAData) {
    const U = N.userAgentData;
    const lowEntropy = () => ({ brands: U.brands.map((b) => ({ ...b })), mobile: U.mobile, platform: U.platform });
    const highEntropy = {
      architecture: U.architecture,
      bitness: U.bitness,
      model: U.model,
      platformVersion: U.platformVersion,
      uaFullVersion: U.uaFullVersion,
      wow64: U.wow64,
    };
    H.spoofGetter(nav.userAgentData, 'brands', Object.freeze(U.brands.map((b) => Object.freeze({ ...b }))));
    H.spoofGetter(nav.userAgentData, 'mobile', U.mobile);
    H.spoofGetter(nav.userAgentData, 'platform', U.platform);
    H.spoofMethod(globalThis.NavigatorUAData.prototype, 'getHighEntropyValues', (native) => ({
      getHighEntropyValues(hints) {
        return native.call(this, hints).then(() => {
          const out = lowEntropy();
          for (const hint of hints) {
            if (hint === 'fullVersionList') out.fullVersionList = U.fullVersionList.map((b) => ({ ...b }));
            else if (hint === 'formFactors') out.formFactors = [...U.formFactors];
            else if (Object.prototype.hasOwnProperty.call(highEntropy, hint)) out[hint] = highEntropy[hint];
          }
          return out;
        });
      },
    }).getHighEntropyValues);
    H.spoofMethod(globalThis.NavigatorUAData.prototype, 'toJSON', () => ({
      toJSON() {
        return lowEntropy();
      },
    }).toJSON);
  }
  const G = P.webgl;
  const params = new Map([
    [0x1f00, G.vendor],
    [0x1f01, G.renderer],
    [0x9245, G.unmaskedVendor ?? null],
    [0x9246, G.unmaskedRenderer ?? null],
  ]);
  for (const Ctx of [globalThis.WebGLRenderingContext, globalThis.WebGL2RenderingContext]) {
    if (!Ctx) continue;
    H.spoofMethod(Ctx.prototype, 'getParameter', (native) => ({
      getParameter(pname) {
        const real = native.apply(this, arguments);
        return params.has(pname) ? params.get(pname) : real;
      },
    }).getParameter);
  }
};
veneerParts.unshift(part);
