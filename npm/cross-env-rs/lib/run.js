'use strict';

const path = require('path');
const fs = require('fs');
const { spawnSync } = require('child_process');

const SUPPORTED = [
  'darwin-arm64',
  'darwin-x64',
  'linux-x64-gnu',
  'linux-x64-musl',
  'linux-arm64-gnu',
  'linux-arm64-musl',
  'windows-x64',
];

function detectLinuxLibc() {
  try {
    const report = process.report && process.report.getReport && process.report.getReport();
    if (report && report.header && report.header.glibcVersionRuntime) {
      return 'gnu';
    }
  } catch (_) { /* ignore */ }
  try {
    if (fs.existsSync('/etc/alpine-release')) return 'musl';
  } catch (_) { /* ignore */ }
  return 'gnu';
}

function detectPlatformPackage() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'darwin' && (arch === 'arm64' || arch === 'x64')) {
    return `cross-env-rs-darwin-${arch}`;
  }
  if (platform === 'linux' && (arch === 'x64' || arch === 'arm64')) {
    return `cross-env-rs-linux-${arch}-${detectLinuxLibc()}`;
  }
  if (platform === 'win32' && arch === 'x64') {
    return 'cross-env-rs-windows-x64';
  }
  return null;
}

function fail(message) {
  process.stderr.write(`cross-env-rs: ${message}\n`);
  process.exit(1);
}

function resolveBinary(mode) {
  const pkg = detectPlatformPackage();
  if (!pkg) {
    fail(
      `unsupported platform ${process.platform}-${process.arch}. ` +
      `Supported: ${SUPPORTED.join(', ')}.`,
    );
  }

  let pkgRoot;
  try {
    pkgRoot = path.dirname(require.resolve(`${pkg}/package.json`));
  } catch (_) {
    fail(
      `${pkg} is not installed. The native binary is shipped via optionalDependencies — ` +
      `if you used --no-optional or --omit=optional, reinstall without that flag.`,
    );
  }

  const exe = process.platform === 'win32' ? `${mode}.exe` : mode;
  const binPath = path.join(pkgRoot, 'bin', exe);
  if (!fs.existsSync(binPath)) {
    fail(`binary not found at ${binPath}. The platform package may be corrupted; try reinstalling.`);
  }
  return binPath;
}

module.exports = function run(mode) {
  const binary = resolveBinary(mode);
  const result = spawnSync(binary, process.argv.slice(2), { stdio: 'inherit' });
  if (result.error) {
    fail(`failed to spawn ${mode}: ${result.error.message}`);
  }
  if (result.signal) {
    process.kill(process.pid, result.signal);
    return;
  }
  process.exit(result.status == null ? 1 : result.status);
};
