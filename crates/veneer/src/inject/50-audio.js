// Audio: tiny deterministic offsets, applied once per channel buffer.
const AB = globalThis.AudioBuffer;
if (AB) {
  const touched = new WeakSet();
  spoofMethod(AB.prototype, 'getChannelData', (native) => ({
    getChannelData(channel) {
      const data = native.call(this, channel);
      if (!touched.has(data)) {
        touched.add(data);
        for (let i = 0; i < data.length; i += 100) {
          data[i] += ((hash(P.audio.seed, i * 8 + channel) & 0xffff) / 0xffff - 0.5) * 1e-7;
        }
      }
      return data;
    },
  }).getChannelData);
  spoofMethod(AB.prototype, 'copyFromChannel', (native) => ({
    copyFromChannel(destination, channelNumber) {
      try {
        this.getChannelData(channelNumber);
      } catch {
        // The native call below reports invalid arguments.
      }
      return native.apply(this, arguments);
    },
  }).copyFromChannel);
}
