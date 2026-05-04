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

// Detects glibc vs musl on Linux. Reads `process.report` (Node-internal,
// reliable when present) first, then falls back to checking
// /etc/alpine-release. Defaults to 'gnu' for unknown distros.
function detectLinuxLibc({ getReport = readGetReport, fileExists = fs.existsSync } = {}) {
  try {
    const report = getReport();
    if (report && report.header && report.header.glibcVersionRuntime) {
      return 'gnu';
    }
  } catch (_) { /* ignore */ }
  try {
    if (fileExists('/etc/alpine-release')) return 'musl';
  } catch (_) { /* ignore */ }
  return 'gnu';
}

function readGetReport() {
  return process.report && process.report.getReport && process.report.getReport();
}

// Pure: maps (platform, arch, libc) → platform package name, or null when
// unsupported. `libc` is consulted only on linux; pass null to let the shim
// auto-detect via detectLinuxLibc().
function detectPlatformPackage({ platform, arch, libc = null } = {}) {
  if (platform === 'darwin' && (arch === 'arm64' || arch === 'x64')) {
    return `cross-env-rs-darwin-${arch}`;
  }
  if (platform === 'linux' && (arch === 'x64' || arch === 'arm64')) {
    const resolvedLibc = libc != null ? libc : detectLinuxLibc();
    return `cross-env-rs-linux-${arch}-${resolvedLibc}`;
  }
  if (platform === 'win32' && arch === 'x64') {
    return 'cross-env-rs-windows-x64';
  }
  return null;
}

// Resolves the absolute path to the native binary for the given mode, or
// throws with a structured failure (caller decides how to surface it).
function resolveBinary({
  mode,
  platform,
  arch,
  libc,
  requireResolve,
  fileExists,
}) {
  const pkg = detectPlatformPackage({ platform, arch, libc });
  if (!pkg) {
    throw new ShimError(
      `unsupported platform ${platform}-${arch}. ` +
      `Supported: ${SUPPORTED.join(', ')}.`,
    );
  }

  let pkgRoot;
  try {
    pkgRoot = path.dirname(requireResolve(`${pkg}/package.json`));
  } catch (_) {
    throw new ShimError(
      `${pkg} is not installed. The native binary is shipped via optionalDependencies — ` +
      `if you used --no-optional or --omit=optional, reinstall without that flag.`,
    );
  }

  const exe = platform === 'win32' ? `${mode}.exe` : mode;
  const binPath = path.join(pkgRoot, 'bin', exe);
  if (!fileExists(binPath)) {
    throw new ShimError(
      `binary not found at ${binPath}. The platform package may be corrupted; try reinstalling.`,
    );
  }
  return binPath;
}

class ShimError extends Error {}

function run(mode, opts = {}) {
  const {
    platform = process.platform,
    arch = process.arch,
    libc = null,
    argv = process.argv.slice(2),
    spawn = spawnSync,
    exit = process.exit.bind(process),
    stderr = process.stderr,
    requireResolve = (id) => require.resolve(id),
    fileExists = fs.existsSync,
    sendSignal = (sig) => process.kill(process.pid, sig),
  } = opts;

  let binary;
  try {
    binary = resolveBinary({ mode, platform, arch, libc, requireResolve, fileExists });
  } catch (err) {
    if (err instanceof ShimError) {
      stderr.write(`cross-env-rs: ${err.message}\n`);
      return exit(1);
    }
    throw err;
  }

  const result = spawn(binary, argv, { stdio: 'inherit' });
  if (result.error) {
    stderr.write(`cross-env-rs: failed to spawn ${mode}: ${result.error.message}\n`);
    return exit(1);
  }
  if (result.signal) {
    return sendSignal(result.signal);
  }
  return exit(result.status == null ? 1 : result.status);
}

module.exports = run;
module.exports.run = run;
module.exports.detectPlatformPackage = detectPlatformPackage;
module.exports.detectLinuxLibc = detectLinuxLibc;
module.exports.SUPPORTED = SUPPORTED;
