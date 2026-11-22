#!/usr/bin/env node
const { spawnSync } = require('node:child_process');
const { randomBytes } = require('node:crypto');
const { readFileSync } = require('node:fs');
const http = require('node:http');
const { join } = require('node:path');
const binary = require('./native.js');

const page = readFileSync(join(__dirname, 'settings.html'));
const script = readFileSync(join(__dirname, 'settings-page.js'));
const token = randomBytes(32).toString('hex');
let origin;

function run(command, input) {
  const child = spawnSync(binary, [command], { input, encoding: 'utf8', windowsHide: true });
  if (child.error) throw child.error;
  if (child.status !== 0) throw new Error(child.stderr.trim() || '本机设置程序执行失败。');
  return child.stdout;
}

async function requestBody(req) {
  const parts = [];
  let length = 0;
  for await (const chunk of req) {
    length += chunk.length;
    if (length > 32 * 1024) throw new Error('设置内容过大。');
    parts.push(chunk);
  }
  return Buffer.concat(parts);
}

const server = http.createServer(async (req, res) => {
  res.setHeader('Cache-Control', 'no-store');
  res.setHeader('Referrer-Policy', 'no-referrer');
  res.setHeader('X-Content-Type-Options', 'nosniff');
  res.setHeader('Content-Security-Policy', "default-src 'none'; script-src 'self'; style-src 'unsafe-inline'; connect-src 'self'; frame-ancestors 'none'; form-action 'none'; base-uri 'none'");
  if (req.headers.host !== new URL(origin).host) {
    res.writeHead(403).end();
    return;
  }

  if (req.method === 'GET' && req.url === '/') {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' }).end(page);
    return;
  }
  if (req.method === 'GET' && req.url === '/settings-page.js') {
    res.writeHead(200, { 'Content-Type': 'text/javascript; charset=utf-8' }).end(script);
    return;
  }
  if (req.headers.authorization !== 'Bearer ' + token ||
      (req.method === 'POST' && req.headers.origin !== origin)) {
    res.writeHead(403).end();
    return;
  }

  res.setHeader('Content-Type', 'application/json; charset=utf-8');
  try {
    if (req.method === 'GET' && req.url === '/settings') {
      res.end(run('settings-read'));
    } else if (req.method === 'POST' && req.url === '/settings') {
      const input = await requestBody(req);
      run('settings-save', input);
      res.end(run('settings-read'));
    } else if (req.method === 'POST' && req.url === '/import') {
      run('settings-import');
      res.end(run('settings-read'));
    } else {
      res.writeHead(404).end();
    }
  } catch (error) {
    res.writeHead(400).end(JSON.stringify({ error: error.message }));
  }
});

server.on('error', error => {
  console.error('[codex-notify] ' + error.message);
  process.exitCode = 1;
});
server.listen(0, '127.0.0.1', () => {
  origin = 'http://127.0.0.1:' + server.address().port;
  console.log('在本机浏览器打开 ' + origin + '/#' + token);
  console.log('完成后关闭此终端，或按 Ctrl+C 结束设置。');
});
