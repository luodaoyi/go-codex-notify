const token = location.hash.slice(1);
const form = document.querySelector('form');
const status = document.querySelector('#status');
const importButton = document.querySelector('#import');
const controls = document.querySelector('fieldset#settings');

async function request(path, method = 'GET', settings) {
  const response = await fetch(path, {
    method,
    headers: { Authorization: 'Bearer ' + token, 'Content-Type': 'application/json' },
    body: settings === undefined ? undefined : JSON.stringify(settings),
  });
  if (response.status === 403) throw new Error('设置链接已失效，请从终端重新打开。');
  const result = await response.json();
  if (!response.ok) throw new Error(result.error);
  return result;
}

function show(settings) {
  for (const input of form.querySelectorAll('input[name]')) {
    if (input.type === 'checkbox') input.checked = Boolean(settings?.[input.name]);
    else input.value = settings?.[input.name] ?? '';
  }
  importButton.hidden = settings !== null;
}

async function apply(operation, message) {
  controls.disabled = true;
  status.textContent = '正在处理…';
  try {
    show(await operation());
    status.textContent = message;
  } catch (error) {
    status.textContent = error.message;
  } finally {
    controls.disabled = false;
  }
}

async function load() {
  try {
    show(await request('/settings'));
    controls.disabled = false;
    status.textContent = '';
  } catch (error) {
    status.textContent = error.message;
  }
}

form.addEventListener('submit', event => {
  event.preventDefault();
  const settings = Object.fromEntries(new FormData(form));
  settings.ignore_subagent_notifications = form.elements.ignore_subagent_notifications.checked;
  apply(() => request('/settings', 'POST', settings), '已保存。下一轮通知会使用这些设置。');
});

importButton.addEventListener('click', () => {
  apply(() => request('/import', 'POST'), '已导入旧设置。请核对各渠道。');
});

load();
