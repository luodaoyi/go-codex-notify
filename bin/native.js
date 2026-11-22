const { join } = require('node:path');

module.exports = join(__dirname, 'native', process.platform + '-' + process.arch,
  process.platform === 'win32' ? 'codex-notify.exe' : 'codex-notify');
