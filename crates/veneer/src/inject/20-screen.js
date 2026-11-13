// screen
const S = P.screen;
for (const key of ['width', 'height', 'availWidth', 'availHeight', 'colorDepth', 'pixelDepth']) {
  spoofGetter(screen, key, S[key]);
}
spoofGetter(window, 'devicePixelRatio', S.devicePixelRatio);
