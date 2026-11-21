const { spawnSync } = require('node:child_process');
const assert = require('node:assert/strict');
const { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync } = require('node:fs');
const { join, resolve } = require('node:path');

const root = join(__dirname, '..');
const manifest = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
const plugin = JSON.parse(readFileSync(join(root, '.codex-plugin', 'plugin.json'), 'utf8'));
assert.equal(plugin.version, manifest.version);
assert.equal(manifest.bin, undefined);
assert.equal(manifest.optionalDependencies, undefined);

const targetRoot = resolve(root, process.env.CARGO_TARGET_DIR || 'target');
const executable = process.platform === 'win32' ? 'codex-notify.exe' : 'codex-notify';
const binary = join(targetRoot, 'debug', executable);
const fixture = mkdtempSync(join(targetRoot, 'plugin-launcher-'));
try {
  const bin = join(fixture, 'bin');
  const launcher = join(bin, 'hook.js');
  const native = join(bin, 'native', process.platform + '-' + process.arch);
  mkdirSync(native, { recursive: true });
  copyFileSync(join(root, 'bin', 'hook.js'), launcher);
  copyFileSync(binary, join(native, executable));
  const env = {
    ...process.env,
    TELEGRAM_BOT_TOKEN: '',
    TELEGRAM_CHAT_ID: '',
    OPENILINK_HUB_URL: '',
    OPENILINK_HUB_TOKEN: '',
    HERMES_WEBHOOK_URL: '',
    HERMES_WEBHOOK_SECRET: '',
    BARK_SERVER_URL: '',
    CODEX_NOTIFY_CONFIG: join(fixture, 'missing-notify-config.json'),
  };

  const payload = JSON.stringify({
    hook_event_name: 'Stop',
    turn_id: 'stdin-forwarding-test',
    last_assistant_message: 'OK',
  });
  const result = spawnSync(process.execPath, [launcher], { cwd: fixture, input: payload, encoding: 'utf8', env });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, '');
  assert.equal(result.stderr, '');

  const invalid = spawnSync(process.execPath, [launcher], { cwd: fixture, input: 'not JSON', encoding: 'utf8', env });
  assert.equal(invalid.status, 1, invalid.stderr);
  assert.match(invalid.stderr, /Codex Hook JSON/);

  const legacy = spawnSync(binary, ['notify', payload], { cwd: fixture, encoding: 'utf8', env });
  assert.equal(legacy.status, 1, legacy.stderr);
  assert.match(legacy.stderr, /Codex Notify/);

  const version = spawnSync(binary, ['--version'], { cwd: fixture, encoding: 'utf8', env });
  assert.equal(version.status, 0, version.stderr);
  assert.equal(version.stdout.trim(), 'codex-notify ' + plugin.version);
} finally {
  rmSync(fixture, { recursive: true });
}
