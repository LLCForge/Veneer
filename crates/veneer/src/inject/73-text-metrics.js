// measureText: tiny deterministic offsets keyed on the canvas seed and the measured value, so equal
// widths stay equal (font-fallback comparisons still agree) while the exact numbers follow the seed.
const part = (P, H) => {
  const TM = globalThis.TextMetrics;
  if (!TM) return;
  const keys = ['width', 'actualBoundingBoxLeft', 'actualBoundingBoxRight', 'actualBoundingBoxAscent', 'actualBoundingBoxDescent'];
  keys.forEach((key, salt) => {
    H.wrapGetter(TM.prototype, key, (v) =>
      v === 0 || !Number.isFinite(v)
        ? v
        : v + ((H.hash(P.canvas.seed ^ (0x7e57 + salt), Math.round(v * 256) | 0) & 0xffff) / 0xffff - 0.5) * 2e-3,
    );
  });
};
part(P, veneerH, false);
veneerParts.push(part);
