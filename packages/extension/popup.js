'use strict';

const $ = (id) => document.getElementById(id);
const t = (key, sub) => chrome.i18n.getMessage(key, sub) || key;

for (const el of document.querySelectorAll('[data-i18n]')) el.textContent = t(el.dataset.i18n);
document.documentElement.lang = chrome.i18n.getUILanguage();

function show(text, isError) {
  $('status').textContent = text;
  $('status').className = isError ? 'error' : '';
}

async function activeTabId() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (!tab || !Number.isInteger(tab.id)) throw new Error(t('noTab'));
  return tab.id;
}

async function send(message) {
  const res = await chrome.runtime.sendMessage(message);
  if (!res || !res.ok) throw new Error(res && res.error ? res.error : t('failed'));
}

$('apply').addEventListener('click', async () => {
  try {
    let bundle;
    try {
      bundle = JSON.parse($('bundle').value);
    } catch {
      throw new Error(t('badJson'));
    }
    const commands = Array.isArray(bundle) ? bundle : bundle && bundle.commands;
    await send({ type: 'apply', tabId: await activeTabId(), commands });
    const p = bundle && bundle.profile;
    show(t('applied', [p ? `${p.device} / ${p.browser}` : '']), false);
  } catch (err) {
    show(String(err.message || err), true);
  }
});

$('detach').addEventListener('click', async () => {
  try {
    await send({ type: 'detach', tabId: await activeTabId() });
    show(t('detached'), false);
  } catch (err) {
    show(String(err.message || err), true);
  }
});
