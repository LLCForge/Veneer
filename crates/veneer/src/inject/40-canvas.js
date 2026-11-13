// Canvas: flip the low bit of one channel on ~3% of opaque pixels, keyed by absolute pixel position.
const noisePixels = (data, x0, y0, width) => {
  for (let i = 0; i < data.length; i += 4) {
    if (data[i + 3] === 0) continue;
    const p = i >> 2;
    const h = hash(P.canvas.seed, Math.imul(y0 + Math.floor(p / width), 65536) + x0 + (p % width));
    if ((h & 31) !== 0) continue;
    data[i + ((h >>> 5) % 3)] ^= 1;
  }
};
const C2D = globalThis.CanvasRenderingContext2D;
if (C2D && globalThis.HTMLCanvasElement) {
  const nativeGetImageData = C2D.prototype.getImageData;
  spoofMethod(C2D.prototype, 'getImageData', (native) => ({
    getImageData(sx, sy, sw, sh) {
      const image = native.apply(this, arguments);
      noisePixels(image.data, Math.floor(sw < 0 ? sx + sw : sx), Math.floor(sh < 0 ? sy + sh : sy), image.width);
      return image;
    },
  }).getImageData);

  // Export a noised copy so the visible canvas is never modified.
  const noisyCopy = (canvas) => {
    try {
      const { width, height } = canvas;
      if (!width || !height) return canvas;
      const copy = document.createElement('canvas');
      copy.width = width;
      copy.height = height;
      const ctx = copy.getContext('2d');
      ctx.drawImage(canvas, 0, 0);
      const image = nativeGetImageData.call(ctx, 0, 0, width, height);
      noisePixels(image.data, 0, 0, width);
      ctx.putImageData(image, 0, 0);
      return copy;
    } catch {
      // Tainted or detached canvas: let the native call raise its own error.
      return canvas;
    }
  };
  const canvasProto = HTMLCanvasElement.prototype;
  spoofMethod(canvasProto, 'toDataURL', (native) => ({
    toDataURL() {
      return native.apply(this instanceof HTMLCanvasElement ? noisyCopy(this) : this, arguments);
    },
  }).toDataURL);
  spoofMethod(canvasProto, 'toBlob', (native) => ({
    toBlob(callback) {
      return native.apply(this instanceof HTMLCanvasElement ? noisyCopy(this) : this, arguments);
    },
  }).toBlob);
}
