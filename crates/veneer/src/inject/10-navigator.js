// navigator
const N = P.navigator;
for (const key of ['userAgent', 'appVersion', 'platform', 'vendor', 'language', 'hardwareConcurrency', 'maxTouchPoints', 'deviceMemory', 'oscpu']) {
  spoofGetter(navigator, key, N[key]);
}
spoofGetter(navigator, 'languages', Object.freeze([...N.languages]));

// Properties the profiled browser does not have must not leak from the host browser.
for (const key of ['deviceMemory', 'oscpu']) {
  if (N[key] == null) {
    const [owner] = findDescriptor(navigator, key);
    if (owner) delete owner[key];
  }
}

if (!N.userAgentData) {
  const [owner] = findDescriptor(navigator, 'userAgentData');
  if (owner) delete owner.userAgentData;
} else if (navigator.userAgentData && typeof NavigatorUAData !== 'undefined') {
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
  spoofGetter(navigator.userAgentData, 'brands', Object.freeze(U.brands.map((b) => Object.freeze({ ...b }))));
  spoofGetter(navigator.userAgentData, 'mobile', U.mobile);
  spoofGetter(navigator.userAgentData, 'platform', U.platform);
  spoofMethod(NavigatorUAData.prototype, 'getHighEntropyValues', (native) => ({
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
  spoofMethod(NavigatorUAData.prototype, 'toJSON', () => ({
    toJSON() {
      return lowEntropy();
    },
  }).toJSON);
}
