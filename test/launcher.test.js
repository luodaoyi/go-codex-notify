const { spawn, spawnSync } = require('node:child_process');
const assert = require('node:assert/strict');
const { join, resolve } = require('node:path');

const root = join(__dirname, '..');
const cli = join(root, 'bin', 'cli.js');
const targetRoot = process.env.CARGO_TARGET_DIR
  ? resolve(root, process.env.CARGO_TARGET_DIR)
  : join(root, 'target');
const binary = join(
  targetRoot,
  'debug',
  process.platform === 'win32' ? 'codex-notify.exe' : 'codex-notify'
);
const cleanEnv = {
  ...process.env,
  CODEX_NOTIFY_BINARY: binary,
  TELEGRAM_BOT_TOKEN: '',
  TELEGRAM_CHAT_ID: '',
  OPENILINK_HUB_URL: '',
  OPENILINK_HUB_TOKEN: '',
  HERMES_WEBHOOK_URL: '',
  HERMES_WEBHOOK_SECRET: '',
  BARK_SERVER_URL: '',
  CODEX_NOTIFY_CONFIG: join(root, 'testdata', 'missing-notify-config.json'),
};

const payload = JSON.stringify({
  hook_event_name: 'Stop',
  session_id: 'stdin-forwarding-test',
  last_assistant_message: 'OK',
});
const stdinResult = spawnSync(process.execPath, [cli], {
  cwd: root,
  input: payload,
  encoding: 'utf8',
  env: cleanEnv,
});
assert.equal(
  stdinResult.status,
  0,
  `stdin forwarding failed\nstdout:\n${stdinResult.stdout}\nstderr:\n${stdinResult.stderr}`
);
assert.equal(stdinResult.stdout, '');
assert.equal(stdinResult.stderr, '');

const argvResult = spawnSync(process.execPath, [cli, payload], {
  cwd: root,
  encoding: 'utf8',
  env: cleanEnv,
});
assert.equal(
  argvResult.status,
  0,
  `argv forwarding failed\nstdout:\n${argvResult.stdout}\nstderr:\n${argvResult.stderr}`
);
assert.equal(argvResult.stdout, '');
assert.equal(argvResult.stderr, '');

const versionResult = spawnSync(process.execPath, [cli, '--version'], {
  cwd: root,
  encoding: 'utf8',
  env: cleanEnv,
});
assert.equal(versionResult.status, 0, versionResult.stderr);
assert.match(versionResult.stdout, /^codex-notify \d+\.\d+\.\d+/);

const openStdinResult = new Promise((resolveResult, reject) => {
  const child = spawn(process.execPath, [cli, payload], {
    cwd: root,
    stdio: ['pipe', 'pipe', 'pipe'],
    env: cleanEnv,
  });
  let stdout = '';
  let stderr = '';
  child.stdout.setEncoding('utf8').on('data', (chunk) => { stdout += chunk; });
  child.stderr.setEncoding('utf8').on('data', (chunk) => { stderr += chunk; });
  const timeout = setTimeout(() => {
    child.kill();
    reject(new Error('argv invocation blocked while stdin remained open'));
  }, 3000);
  child.on('error', reject);
  child.on('exit', (status) => {
    clearTimeout(timeout);
    resolveResult({ status, stdout, stderr });
  });
});

openStdinResult
  .then((result) => {
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout, '');
    assert.equal(result.stderr, '');
  })
  .catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
