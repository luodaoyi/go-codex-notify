const { spawnSync } = require('node:child_process');
const { readFileSync } = require('node:fs');
const binary = require('./native.js');

try {
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
