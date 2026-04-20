#!/usr/bin/env node

const { spawnSync } = require('node:child_process');
const { existsSync } = require('node:fs');
const { join } = require('node:path');

const ROOT = join(__dirname, '..');
const GO_MAIN = join(ROOT, 'main.go');
const HAS_LOCAL_SOURCE = existsSync(GO_MAIN);

(() => {
  try {
    const result = HAS_LOCAL_SOURCE ? runGoSource() : runBundledBinary();
    if (result.error) throw result.error;
    process.exit(result.status ?? 0);
  } catch (err) {
    console.error(`[go-codex-notify] ${err.message}`);
    process.exit(1);
  }
})();

function runGoSource() {
  return spawnSync('go', ['run', '.'], {
    cwd: ROOT,
    stdio: ['ignore', 'inherit', 'inherit'],
    env: process.env,
  });
}

function runBundledBinary() {
  const binaryPath = join(ROOT, '.bin', getArtifactName());
  if (!existsSync(binaryPath)) {
    throw new Error(
      `binary not found at ${binaryPath}. Run postinstall or execute: node scripts/install.js`
    );
  }

  return spawnSync(binaryPath, process.argv.slice(2), {
    stdio: ['ignore', 'inherit', 'inherit'],
    env: process.env,
  });
}

function getArtifactName() {
  const p = process.platform;
  const a = process.arch;
  const map = {
    'win32:x64': 'notify-telegram-windows-amd64.exe',
    'win32:arm64': 'notify-telegram-windows-arm64.exe',
    'linux:x64': 'notify-telegram-linux-amd64',
    'linux:arm64': 'notify-telegram-linux-arm64',
    'darwin:x64': 'notify-telegram-darwin-amd64',
    'darwin:arm64': 'notify-telegram-darwin-arm64',
  };
  const key = `${p}:${a}`;
  if (!map[key]) {
    throw new Error(`unsupported platform: ${key}`);
  }
  return map[key];
}
