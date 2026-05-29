const { spawnSync } = require('node:child_process');
const assert = require('node:assert/strict');
const { join } = require('node:path');

const root = join(__dirname, '..');
const cli = join(root, 'bin', 'cli.js');
const payload = JSON.stringify({
  hook_event_name: 'Stop',
  session_id: 'stdin-forwarding-test',
  model: 'gpt-test',
  last_assistant_message: 'OK',
});

const result = spawnSync(process.execPath, [cli], {
  cwd: root,
  input: payload,
  encoding: 'utf8',
  env: {
    ...process.env,
    TELEGRAM_BOT_TOKEN: '',
    TELEGRAM_CHAT_ID: '',
    OPENILINK_HUB_URL: '',
    OPENILINK_HUB_TOKEN: '',
    HERMES_WEBHOOK_URL: '',
    HERMES_WEBHOOK_SECRET: '',
    BARK_SERVER_URL: '',
    CODEX_NOTIFY_CONFIG: join(root, 'testdata', 'missing-notify-config.json'),
  },
});

assert.equal(result.status, 0, `cli failed\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
assert.equal(result.stdout, '');
assert.equal(result.stderr, '');

const notifyPayload = JSON.stringify({
  type: 'agent-turn-complete',
  'thread-id': 'argv-forwarding-test',
  'turn-id': 'turn-1',
  cwd: root,
  'input-messages': ['确认 notify 参数转发'],
  'last-assistant-message': 'OK',
});

const argvResult = spawnSync(process.execPath, [cli, notifyPayload], {
  cwd: root,
  encoding: 'utf8',
  env: {
    ...process.env,
    TELEGRAM_BOT_TOKEN: '',
    TELEGRAM_CHAT_ID: '',
    OPENILINK_HUB_URL: '',
    OPENILINK_HUB_TOKEN: '',
    HERMES_WEBHOOK_URL: '',
    HERMES_WEBHOOK_SECRET: '',
    BARK_SERVER_URL: '',
    CODEX_NOTIFY_CONFIG: join(root, 'testdata', 'missing-notify-config.json'),
  },
});

assert.equal(
  argvResult.status,
  0,
  `cli argv forwarding failed\nstdout:\n${argvResult.stdout}\nstderr:\n${argvResult.stderr}`
);
assert.equal(argvResult.stdout, '');
assert.equal(argvResult.stderr, '');
