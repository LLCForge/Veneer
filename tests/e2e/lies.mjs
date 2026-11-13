// Offline checks in the style of CreepJS "lies": each one compares two views of the same value that a real browser
// keeps consistent. Runs inside the page through page.evaluate, so it must stay self-contained.
export function findLies({ screen: expectedScreen, dpr }) {
  const lies = [];
  const toString = Function.prototype.toString;
  const isNative = (fn) => typeof fn === 'function' && /^function [\w$ ]*\(\) \{\s*\[native code\]\s*\}$/.test(toString.call(fn));

  function checkFunction(where, fn, expectedName) {
    if (!isNative(fn)) lies.push(`${where}: toString is not native`);
    if (fn.name !== expectedName) lies.push(`${where}: name is ${JSON.stringify(fn.name)}`);
    if (Object.prototype.hasOwnProperty.call(fn, 'prototype')) lies.push(`${where}: has a prototype property`);
    try {
      Reflect.construct(String, [], fn);
      lies.push(`${where}: is a constructor`);
    } catch {
      // Native getters and methods are not constructors.
    }
  }

  function checkGetters(proto, protoName, props) {
    for (const prop of props) {
      if (!(prop in proto)) continue;
      const desc = Object.getOwnPropertyDescriptor(proto, prop);
      if (!desc || typeof desc.get !== 'function') {
        lies.push(`${protoName}.${prop}: not an accessor on the prototype`);
        continue;
      }
      checkFunction(`${protoName}.${prop} getter`, desc.get, `get ${prop}`);
      try {
        desc.get.call(Object.create(proto));
        lies.push(`${protoName}.${prop}: getter accepts a foreign receiver`);
      } catch (err) {
        if (!(err instanceof TypeError)) lies.push(`${protoName}.${prop}: getter threw ${err?.name}`);
      }
    }
  }

  function checkMethods(proto, protoName, names) {
    for (const name of names) {
      if (proto && typeof proto[name] === 'function') checkFunction(`${protoName}.${name}`, proto[name], name);
    }
  }

  checkFunction('Function.prototype.toString', toString, 'toString');
  checkGetters(Navigator.prototype, 'Navigator', [
    'userAgent', 'appVersion', 'platform', 'vendor', 'language', 'languages', 'hardwareConcurrency',
    'deviceMemory', 'maxTouchPoints', 'oscpu', 'userAgentData', 'webdriver',
  ]);
  checkGetters(Screen.prototype, 'Screen', ['width', 'height', 'availWidth', 'availHeight', 'colorDepth', 'pixelDepth']);
  checkMethods(WebGLRenderingContext.prototype, 'WebGLRenderingContext', ['getParameter', 'getExtension']);
  checkMethods(globalThis.WebGL2RenderingContext?.prototype, 'WebGL2RenderingContext', ['getParameter']);
  checkMethods(HTMLCanvasElement.prototype, 'HTMLCanvasElement', ['toDataURL', 'toBlob', 'getContext']);
  checkMethods(CanvasRenderingContext2D.prototype, 'CanvasRenderingContext2D', ['getImageData']);
  checkMethods(globalThis.NavigatorUAData?.prototype, 'NavigatorUAData', ['getHighEntropyValues', 'toJSON']);

  const ownNavigator = Object.getOwnPropertyNames(navigator);
  if (ownNavigator.length) lies.push(`navigator has own properties: ${ownNavigator.join(', ')}`);
  if (navigator.language !== navigator.languages[0]) lies.push('navigator.language differs from languages[0]');
  if (screen.width !== expectedScreen.width || screen.height !== expectedScreen.height) lies.push('screen size differs from the profile');
  if (screen.availWidth > screen.width || screen.availHeight > screen.height) lies.push('available screen larger than screen');
  if (!matchMedia(`(device-width: ${screen.width}px) and (device-height: ${screen.height}px)`).matches) {
    lies.push('device-width/height media query disagrees with screen');
  }
  if (!matchMedia(`(resolution: ${dpr}dppx)`).matches || devicePixelRatio !== dpr) lies.push('resolution media query or devicePixelRatio disagrees with the profile');

  // A fresh same-origin iframe must report the same navigator as its parent.
  const frame = document.createElement('iframe');
  document.body.append(frame);
  const fnav = frame.contentWindow.navigator;
  for (const prop of ['userAgent', 'platform', 'hardwareConcurrency', 'deviceMemory', 'maxTouchPoints', 'vendor']) {
    if (fnav[prop] !== navigator[prop]) lies.push(`iframe navigator.${prop} differs`);
  }
  if (frame.contentWindow.screen.width !== screen.width) lies.push('iframe screen.width differs');
  frame.remove();

  return lies;
}

// A dedicated worker has its own navigator and does not run page init scripts; it must agree with the page.
export async function findWorkerLies(workerUrl) {
  const lies = [];
  const seen = await new Promise((resolve) => {
    const worker = new Worker(workerUrl);
    const timer = setTimeout(() => resolve(null), 5000);
    worker.onmessage = (e) => {
      clearTimeout(timer);
      worker.terminate();
      resolve(e.data);
    };
    worker.onerror = () => {
      clearTimeout(timer);
      resolve(null);
    };
  });
  if (!seen) {
    lies.push('worker did not report');
  } else {
    const page = {
      userAgent: navigator.userAgent,
      platform: navigator.platform,
      language: navigator.language,
      languages: [...navigator.languages],
      hardwareConcurrency: navigator.hardwareConcurrency,
      deviceMemory: navigator.deviceMemory ?? null,
      timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    };
    for (const [k, v] of Object.entries(page)) {
      if (JSON.stringify(seen[k]) !== JSON.stringify(v)) lies.push(`worker ${k} differs: ${JSON.stringify(seen[k])} vs ${JSON.stringify(v)}`);
    }
  }
  return lies;
}
