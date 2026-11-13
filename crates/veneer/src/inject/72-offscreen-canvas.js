// OffscreenCanvas: the same pixel noise as 40-canvas.js (same seed, same absolute positions), on
// getImageData and on a noised copy for convertToBlob, so both canvas kinds export identical bytes.
const part = (P, H) => {
  const OC = globalThis.OffscreenCanvas;
  const Ctx = globalThis.OffscreenCanvasRenderingContext2D;
  if (!OC || !Ctx) return;
  const noise = (data, x0, y0, width) => {
    for (let i = 0; i < data.length; i += 4) {
      if (data[i + 3] === 0) continue;
      const p = i >> 2;
      const h = H.hash(P.canvas.seed, Math.imul(y0 + Math.floor(p / width), 65536) + x0 + (p % width));
      if ((h & 31) !== 0) continue;
      data[i + ((h >>> 5) % 3)] ^= 1;
    }
  };
  const nativeGetImageData = Ctx.prototype.getImageData;
  const nativeGetContext = OC.prototype.getContext;
  H.spoofMethod(Ctx.prototype, 'getImageData', (native) => ({
    getImageData(sx, sy, sw, sh) {
      const image = native.apply(this, arguments);
      noise(image.data, Math.floor(sw < 0 ? sx + sw : sx), Math.floor(sh < 0 ? sy + sh : sy), image.width);
      return image;
    },
  }).getImageData);
  H.spoofMethod(OC.prototype, 'convertToBlob', (native) => ({
    convertToBlob() {
      let source = this;
      try {
        const { width, height } = this;
        if (width && height) {
          const copy = new OC(width, height);
          const ctx = nativeGetContext.call(copy, '2d');
          ctx.drawImage(this, 0, 0);
          const image = nativeGetImageData.call(ctx, 0, 0, width, height);
          noise(image.data, 0, 0, width);
          ctx.putImageData(image, 0, 0);
          source = copy;
        }
      } catch {
        // Detached, zero-sized or wrong receiver: the native call reports it.
      }
      return native.apply(source, arguments);
    },
  }).convertToBlob);
};
part(P, veneerH, false);
veneerParts.push(part);
