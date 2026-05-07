#!/usr/bin/env node

const { createWriteStream, existsSync, mkdirSync, chmodSync, copyFileSync } = require('node:fs');
const { join, dirname } = require('node:path');
const { homedir, platform, arch } = require('node:os');
const https = require('node:https');

const pkg = require('../package.json');

const artifact = resolveArtifactName();
const versionTag = `v${pkg.version}`;
const url = `https://github.com/luodaoyi/go-codex-notify/releases/download/${versionTag}/${artifact}`;
const cachePath = join(homedir(), '.codex', 'go-codex-notify', pkg.version, artifact);
const localBinPath = join(__dirname, '..', '.bin', artifact);
const stableBinPath = join(homedir(), '.codex', 'bin', stableBinaryName());

(async () => {
  try {
    if (!existsSync(cachePath)) {
      await download(url, cachePath);
      ensureExecutable(cachePath);
    }

    mkdirSync(dirname(localBinPath), { recursive: true });
    copy(cachePath, localBinPath);
    ensureExecutable(localBinPath);

    mkdirSync(dirname(stableBinPath), { recursive: true });
    copy(cachePath, stableBinPath);
    ensureExecutable(stableBinPath);

    console.log(`[go-codex-notify] ready: ${stableBinPath}`);
  } catch (err) {
    console.error(`[go-codex-notify] install failed: ${err.message}`);
    process.exit(1);
  }
})();

function resolveArtifactName() {
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

function stableBinaryName() {
  return platform() === 'win32' ? 'go-codex-notify.exe' : 'go-codex-notify';
}

function download(url, target) {
  mkdirSync(dirname(target), { recursive: true });
  return new Promise((resolve, reject) => {
    const file = createWriteStream(target);
    https
      .get(url, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          file.close();
          return download(res.headers.location, target).then(resolve, reject);
        }
        if (res.statusCode !== 200) {
          file.close();
          return reject(new Error(`download failed: ${res.statusCode} ${url}`));
        }
        res.pipe(file);
        file.on('finish', () => {
          file.close(resolve);
        });
      })
      .on('error', (err) => {
        file.close();
        reject(err);
      });
  });
}

function copy(src, dst) {
  copyFileSync(src, dst);
}

function ensureExecutable(target) {
  if (platform() !== 'win32') {
    chmodSync(target, 0o755);
  }
}
