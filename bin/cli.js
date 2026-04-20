#!/usr/bin/env node

const { spawnSync } = require('node:child_process');
const { existsSync, mkdirSync, chmodSync, createWriteStream, copyFileSync } = require('node:fs');
const { homedir, platform, arch } = require('node:os');
const { dirname, join } = require('node:path');
const https = require('node:https');

const pkg = require('../package.json');
const BIN_DIR = join(__dirname, '..', '.bin');
const EXECUTABLE = getExecutableName();
const BINARY_PATH = join(BIN_DIR, EXECUTABLE);

(async () => {
  try {
    ensureBinary();
    const result = spawnSync(BINARY_PATH, process.argv.slice(2), {
      stdio: ['inherit', 'inherit', 'inherit'],
      env: process.env,
    });
    if (result.error) throw result.error;
    process.exit(result.status ?? 0);
  } catch (err) {
    console.error(`[go-codex-notify] ${err.message}`);
    process.exit(1);
  }
})();

function ensureBinary() {
  if (existsSync(BINARY_PATH)) return;
  const cachePath = getCachePath();
  if (existsSync(cachePath)) {
    mkdirSync(dirname(BINARY_PATH), { recursive: true });
    copyFileSync(cachePath, BINARY_PATH);
    if (platform() !== 'win32') chmodSync(BINARY_PATH, 0o755);
    return;
  }
  throw new Error(
    `binary not found at ${BINARY_PATH}. Run postinstall or execute: node scripts/install.js`
  );
}

function getExecutableName() {
  const mapping = getArtifactName();
  return mapping;
}

function getArtifactName() {
  const p = platform();
  const a = arch();
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

function getCachePath() {
  return join(homedir(), '.codex', 'go-codex-notify', pkg.version, getArtifactName());
}
