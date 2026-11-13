'use strict';

// Turkish and English interface text. Elements with data-i18n="key" get their text from here.
(() => {
  const STRINGS = {
    tr: {
      validateTitle: 'Doğrulayıcı',
      validateHelp: 'Bir profil, gerçek tarayıcı toplayıcı çıktısı ya da başka bir aracın JSON çıktısını yapıştırın; tutarlılık raporunu görün.',
      validateLink: 'Doğrulayıcı',
      contributeLink: 'Katkı',
      inputLabel: 'Profil JSON',
      formatLabel: 'Biçim',
      'format.auto': 'Otomatik',
      'format.veneer': 'Veneer profili',
      'format.collector': 'Toplayıcı çıktısı (gerçek tarayıcı)',
      'format.fingerprint-suite': 'fingerprint-suite',
      'format.flat': 'Düz JSON',
      countryLabel: 'Bağlantı ülkesi (ör. TR)',
      geoTimezoneLabel: 'Bağlantı saat dilimi (ör. Europe/Istanbul)',
      disableLabel: 'Kapatılacak kurallar (virgülle, ör. fonts.*)',
      check: 'Doğrula',
      sample: 'Örnek profil yükle',
      validateEmpty: 'Henüz bir profil doğrulanmadı.',
      severity: 'Önem',
      'severity.error': 'Hata',
      'severity.warning': 'Uyarı',
      rule: 'Kural',
      message: 'Açıklama',
      fix: 'Düzeltme',
      noIssues: 'Sorun bulunmadı',
      issuesFound: 'sorun bulundu',
      rulesVersion: 'Kural sürümü',
      missingFields: 'Girdide olmayan, türetilen alanlar',
      invalidJson: 'Girdi geçerli bir JSON değil.',
      inputTooLong: 'Girdi 60.000 karakterden uzun.',
      skip: 'İçeriğe geç',
      subtitle: 'Tutarlı tarayıcı parmak izleri',
      catalogLink: 'API kataloğu',
      authTitle: 'Erişim anahtarı',
      authHelp: 'Bu sunucu bir anahtarla korunuyor. Anahtar yalnızca HttpOnly bir çereze yazılır.',
      tokenLabel: 'Anahtar',
      login: 'Giriş',
      constraints: 'Kısıtlar',
      os: 'İşletim sistemi',
      browser: 'Tarayıcı',
      kind: 'Cihaz sınıfı',
      device: 'Cihaz',
      any: 'Herhangi',
      locale: 'Dil (ör. tr veya tr,en)',
      country: 'Ülke (ör. TR)',
      seed: 'Seed (boş: rastgele)',
      count: 'Adet (1-100)',
      generate: 'Üret',
      tryHere: 'Bu tarayıcıda dene',
      result: 'Sonuç',
      copy: 'JSON kopyala',
      copied: 'Kopyalandı',
      empty: 'Henüz bir parmak izi üretilmedi.',
      needToken: 'Bu işlem için erişim anahtarı gerekiyor.',
      requestFailed: 'İstek başarısız',
      copyFailed: 'Panoya kopyalanamadı.',
      score: 'Puan',
      'kind.desktop': 'Masaüstü',
      'kind.laptop': 'Dizüstü',
      'kind.phone': 'Telefon',
      'kind.tablet': 'Tablet',
      'kind.console': 'Konsol',
      'kind.tv': 'Televizyon',
      'kind.vr': 'VR',
      'kind.handheld': 'El konsolu (PC)',
      probeTitle: 'Canlı kontrol',
      probeHelp: 'Parmak izi bu sayfaya uygulandı; tarayıcının bildirdiği değerler profille karşılaştırılıyor.',
      probeNote: 'Normal bir tarayıcıda yalnızca JavaScript tarafından görülen değerler değişir. HTTP başlıkları ve saat dilimi için Playwright/Puppeteer yardımcılarını ya da npx veneer-verify komutunu kullanın.',
      back: 'Geri dön',
      checking: 'Kontrol ediliyor…',
      field: 'Alan',
      expected: 'Beklenen',
      observed: 'Gözlenen',
      status: 'Durum',
      match: 'Eşleşiyor',
      differ: 'Farklı',
      probeFailed: 'Kontrol tamamlanamadı',
      collectorMissing: 'Toplayıcı yüklenemedi (erişim anahtarı gerekiyor olabilir).',
      language: 'Dil',
      contributeTitle: 'Cihaz verisi katkısı',
      contributeSubtitle: 'Konsol, TV, VR ve uygulama içi tarayıcı verilerini doğrulamamıza yardım edin.',
      contributeWhatTitle: 'Neler toplanır',
      contributeWhat: 'Yalnızca donanım ve tarayıcı özellikleri: user-agent, platform, çekirdek ve bellek, dokunma noktaları, ekran ve piksel oranı, WebGL üreticisi ve işlemcisi, client hints, desteklenen medya özellikleri.',
      contributeNot: 'Toplanmayanlar: IP adresi, çerezler, saat dilimi, kimlikler, canvas/ses parmak izleri ve serbest metin. Sunucu hiçbir şey saklamaz ve hiçbir yere göndermez; rapor yalnızca bu sayfada oluşur.',
      contributeHow: "Raporu kopyalayıp GitHub'daki cihaz raporu şablonuna yapıştırın. Göndermeden önce içeriği kendiniz okuyun.",
      contributeRun: 'Raporu oluştur',
      contributeIssue: "GitHub'da cihaz raporu aç",
      contributeReport: 'Rapor',
      contributeEmpty: 'Henüz rapor oluşturulmadı.',
      contributeInsecure: 'Sayfa HTTPS veya localhost üzerinden açılmadığı için tarayıcı client hints (userAgentData) ve panoya kopyalamayı gizliyor. Rapor yine de geçerlidir; gerekirse metni elle seçip kopyalayın.',
    },
    en: {
      validateTitle: 'Validator',
      validateHelp: 'Paste a profile, real-browser collector output or JSON from another tool and see the consistency report.',
      validateLink: 'Validator',
      contributeLink: 'Contribute',
      inputLabel: 'Profile JSON',
      formatLabel: 'Format',
      'format.auto': 'Automatic',
      'format.veneer': 'Veneer profile',
      'format.collector': 'Collector output (real browser)',
      'format.fingerprint-suite': 'fingerprint-suite',
      'format.flat': 'Flat JSON',
      countryLabel: 'Connection country (e.g. TR)',
      geoTimezoneLabel: 'Connection timezone (e.g. Europe/Istanbul)',
      disableLabel: 'Rules to disable (comma-separated, e.g. fonts.*)',
      check: 'Validate',
      sample: 'Load a sample profile',
      validateEmpty: 'No profile validated yet.',
      severity: 'Severity',
      'severity.error': 'Error',
      'severity.warning': 'Warning',
      rule: 'Rule',
      message: 'Message',
      fix: 'Fix',
      noIssues: 'No issues found',
      issuesFound: 'issues found',
      rulesVersion: 'Rules version',
      missingFields: 'Fields missing from the input (derived)',
      invalidJson: 'The input is not valid JSON.',
      inputTooLong: 'The input is longer than 60,000 characters.',
      skip: 'Skip to content',
      subtitle: 'Coherent browser fingerprints',
      catalogLink: 'API catalog',
      authTitle: 'Access token',
      authHelp: 'This server is protected by a token. It is stored only in an HttpOnly cookie.',
      tokenLabel: 'Token',
      login: 'Sign in',
      constraints: 'Constraints',
      os: 'Operating system',
      browser: 'Browser',
      kind: 'Device class',
      device: 'Device',
      any: 'Any',
      locale: 'Language (e.g. tr or tr,en)',
      country: 'Country (e.g. TR)',
      seed: 'Seed (empty: random)',
      count: 'Count (1-100)',
      generate: 'Generate',
      tryHere: 'Try in this browser',
      result: 'Result',
      copy: 'Copy JSON',
      copied: 'Copied',
      empty: 'No fingerprint generated yet.',
      needToken: 'This action needs an access token.',
      requestFailed: 'Request failed',
      copyFailed: 'Could not copy to the clipboard.',
      score: 'Score',
      'kind.desktop': 'Desktop',
      'kind.laptop': 'Laptop',
      'kind.phone': 'Phone',
      'kind.tablet': 'Tablet',
      'kind.console': 'Console',
      'kind.tv': 'TV',
      'kind.vr': 'VR',
      'kind.handheld': 'Handheld PC',
      probeTitle: 'Live check',
      probeHelp: 'The fingerprint was applied to this page; values reported by the browser are compared with the profile.',
      probeNote: 'In a regular browser only JavaScript-visible values change. For HTTP headers and timezone, use the Playwright/Puppeteer helpers or npx veneer-verify.',
      back: 'Back',
      checking: 'Checking…',
      field: 'Field',
      expected: 'Expected',
      observed: 'Observed',
      status: 'Status',
      match: 'Match',
      differ: 'Differs',
      probeFailed: 'Check could not complete',
      collectorMissing: 'Collector could not load (an access token may be required).',
      language: 'Language',
      contributeTitle: 'Contribute device data',
      contributeSubtitle: 'Help us verify data for consoles, TVs, VR headsets and in-app browsers.',
      contributeWhatTitle: 'What is collected',
      contributeWhat: 'Only hardware and browser characteristics: user agent, platform, cores and memory, touch points, screen and pixel ratio, WebGL vendor and renderer, client hints, supported media features.',
      contributeNot: 'Not collected: IP address, cookies, timezone, identifiers, canvas/audio fingerprints and free text. The server stores nothing and sends nothing anywhere; the report exists only on this page.',
      contributeHow: 'Copy the report and paste it into the device report template on GitHub. Read it yourself before submitting.',
      contributeRun: 'Create report',
      contributeIssue: 'Open a device report on GitHub',
      contributeReport: 'Report',
      contributeEmpty: 'No report created yet.',
      contributeInsecure: 'This page is not served over HTTPS or localhost, so the browser hides client hints (userAgentData) and clipboard access. The report is still valid; select and copy the text by hand if needed.',
    },
  };

  const KEY = 'veneer.lang';
  const fromUrl = new URLSearchParams(location.search).get('lang');
  let stored = null;
  try {
    stored = localStorage.getItem(KEY);
  } catch {
    // Storage blocked: fall back to the browser language.
  }
  const preferred = (navigator.languages && navigator.languages[0]) || 'en';
  let lang = [fromUrl, stored].find((l) => l === 'tr' || l === 'en') || (preferred.toLowerCase().startsWith('tr') ? 'tr' : 'en');

  const t = (key) => STRINGS[lang][key] ?? STRINGS.en[key] ?? key;

  function apply() {
    document.documentElement.lang = lang;
    for (const el of document.querySelectorAll('[data-i18n]')) el.textContent = t(el.dataset.i18n);
    for (const button of document.querySelectorAll('[data-lang]')) {
      button.setAttribute('aria-pressed', String(button.dataset.lang === lang));
    }
    document.dispatchEvent(new CustomEvent('veneer:lang'));
  }

  function set(next) {
    if (next !== 'tr' && next !== 'en') return;
    lang = next;
    try {
      localStorage.setItem(KEY, lang);
    } catch {
      // Not persisted; the choice still applies to this page.
    }
    apply();
  }

  window.veneerI18n = { t, set, get lang() { return lang; } };
  document.addEventListener('DOMContentLoaded', () => {
    for (const button of document.querySelectorAll('[data-lang]')) {
      button.addEventListener('click', () => set(button.dataset.lang));
    }
    apply();
  });
})();
