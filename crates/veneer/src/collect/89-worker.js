// What a dedicated worker sees (the init script must reach worker scopes too).
out.worker = await new Promise((resolve) => {
  let worker;
  let url;
  const done = (value) => {
    clearTimeout(timer);
    if (worker) worker.terminate();
    if (url) URL.revokeObjectURL(url);
    resolve(value);
  };
  const timer = setTimeout(() => done(null), 3000);
  try {
    const code =
      'postMessage({ userAgent: navigator.userAgent, platform: navigator.platform, language: navigator.language, hardwareConcurrency: navigator.hardwareConcurrency });';
    url = URL.createObjectURL(new Blob([code], { type: 'text/javascript' }));
    worker = new Worker(url);
    worker.onmessage = (e) => done(e.data);
    worker.onerror = () => done(null);
  } catch {
    done(null);
  }
});
