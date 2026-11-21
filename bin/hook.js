const { spawnSync } = require('node:child_process');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');

try {
  const { platform, arch } = process;
  const executable = platform === 'win32' ? 'codex-notify.exe' : 'codex-notify';
  const binary = join(__dirname, 'native', platform + '-' + arch, executable);
  const result = spawnSync(binary, ['hook'], {
    input: readFileSync(0),
    stdio: ['pipe', 'inherit', 'inherit'],
    windowsHide: true,
  });
  if (result.error) throw result.error;
  if (result.signal) throw new Error('native hook exited with signal ' + result.signal);
  process.exitCode = result.status;
} catch (error) {
  console.error('[codex-notify] ' + error.message);
  process.exitCode = 1;
}
