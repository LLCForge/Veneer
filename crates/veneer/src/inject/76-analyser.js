// AnalyserNode: tiny deterministic offsets from the audio seed, keyed by frequency bin.
const Analyser = globalThis.AnalyserNode;
if (Analyser) {
  const salt = P.audio.seed ^ 0x5a17;
  spoofMethod(Analyser.prototype, 'getFloatFrequencyData', (native) => ({
    getFloatFrequencyData(array) {
      const result = native.apply(this, arguments);
      for (let i = 0; i < array.length; i += 1) {
        if (Number.isFinite(array[i])) array[i] += ((hash(salt, i) & 0xffff) / 0xffff - 0.5) * 2e-4;
      }
      return result;
    },
  }).getFloatFrequencyData);
  spoofMethod(Analyser.prototype, 'getByteFrequencyData', (native) => ({
    getByteFrequencyData(array) {
      const result = native.apply(this, arguments);
      for (let i = 0; i < array.length; i += 1) {
        const h = hash(salt, i);
        if (array[i] > 0 && array[i] < 255 && (h & 31) === 0) array[i] += h & 32 ? 1 : -1;
      }
      return result;
    },
  }).getByteFrequencyData);
}
