'use strict';

// Tests for lib/run.js — platform→package mapping, libc detection, error paths.
// Tests inject seams into `run()`; no real child process is spawned and no
// real fs/require lookups are performed.

const { test } = require('node:test');
const assert = require('node:assert/strict');

const {
  run,
  detectPlatformPackage,
  detectLinuxLibc,
  SUPPORTED,
} = require('../lib/run.js');

// Builds a fresh capture bag so tests don't share mutable state.
function captures() {
  const stderrBuf = [];
  const exitCalls = [];
  const sendSignalCalls = [];
  return {
    exitCalls,
    sendSignalCalls,
    stderr: { write: (s) => stderrBuf.push(s) },
    exit: (c) => { exitCalls.push(c); },
    sendSignal: (s) => { sendSignalCalls.push(s); },
    stderrText: () => stderrBuf.join(''),
  };
}

test('linux + libc=gnu maps to *-linux-<arch>-gnu', () => {
  assert.equal(
    detectPlatformPackage({ platform: 'linux', arch: 'x64', libc: 'gnu' }),
    'cross-env-rs-linux-x64-gnu',
  );
  assert.equal(
    detectPlatformPackage({ platform: 'linux', arch: 'arm64', libc: 'gnu' }),
    'cross-env-rs-linux-arm64-gnu',
  );
});

test('linux + libc=musl maps to *-linux-<arch>-musl', () => {
  assert.equal(
    detectPlatformPackage({ platform: 'linux', arch: 'x64', libc: 'musl' }),
    'cross-env-rs-linux-x64-musl',
  );
  assert.equal(
    detectPlatformPackage({ platform: 'linux', arch: 'arm64', libc: 'musl' }),
    'cross-env-rs-linux-arm64-musl',
  );
});

test('detectLinuxLibc returns gnu when process.report exposes glibc version', () => {
  const result = detectLinuxLibc({
    getReport: () => ({ header: { glibcVersionRuntime: '2.31' } }),
    fileExists: () => false,
  });
  assert.equal(result, 'gnu');
});

test('detectLinuxLibc returns musl when /etc/alpine-release exists', () => {
  const result = detectLinuxLibc({
    getReport: () => null,
    fileExists: (p) => p === '/etc/alpine-release',
  });
  assert.equal(result, 'musl');
});

test('detectLinuxLibc defaults to gnu when neither probe matches', () => {
  const result = detectLinuxLibc({
    getReport: () => null,
    fileExists: () => false,
  });
  assert.equal(result, 'gnu');
});

test('detectLinuxLibc tolerates throwing probes (defaults to gnu)', () => {
  const result = detectLinuxLibc({
    getReport: () => { throw new Error('boom'); },
    fileExists: () => { throw new Error('boom'); },
  });
  assert.equal(result, 'gnu');
});

test('darwin platforms ignore libc', () => {
  assert.equal(
    detectPlatformPackage({ platform: 'darwin', arch: 'arm64', libc: 'musl' }),
    'cross-env-rs-darwin-arm64',
  );
  assert.equal(
    detectPlatformPackage({ platform: 'darwin', arch: 'x64' }),
    'cross-env-rs-darwin-x64',
  );
});

test('detectPlatformPackage returns null for unknown platforms', () => {
  assert.equal(detectPlatformPackage({ platform: 'haiku', arch: 'x64' }), null);
  assert.equal(detectPlatformPackage({ platform: 'sunos', arch: 'x64' }), null);
});

test('detectPlatformPackage returns null for win32-arm64 (not yet shipped)', () => {
  assert.equal(detectPlatformPackage({ platform: 'win32', arch: 'arm64' }), null);
});

test('detectPlatformPackage returns null for darwin-ia32', () => {
  assert.equal(detectPlatformPackage({ platform: 'darwin', arch: 'ia32' }), null);
});

test('run() exits 1 with a clear error when platform is unsupported', () => {
  const c = captures();
  run('cross-env', {
    platform: 'haiku',
    arch: 'x64',
    stderr: c.stderr,
    exit: c.exit,
    spawn: () => assert.fail('spawn must not be reached on unsupported platform'),
    requireResolve: () => assert.fail('requireResolve must not be reached'),
    fileExists: () => assert.fail('fileExists must not be reached'),
  });
  assert.deepEqual(c.exitCalls, [1]);
  const text = c.stderrText();
  assert.match(text, /^cross-env-rs:/);
  assert.match(text, /unsupported platform haiku-x64/);
  assert.ok(
    text.includes(SUPPORTED.join(', ')),
    'error message should list every supported platform pair',
  );
});

test('run() exits 1 with optionalDependencies hint when platform package is missing', () => {
  const c = captures();
  run('cross-env', {
    platform: 'darwin',
    arch: 'arm64',
    stderr: c.stderr,
    exit: c.exit,
    requireResolve: () => {
      const e = new Error('cannot find module');
      e.code = 'MODULE_NOT_FOUND';
      throw e;
    },
    fileExists: () => true,
    spawn: () => assert.fail('spawn must not be reached when package is missing'),
  });
  assert.deepEqual(c.exitCalls, [1]);
  const text = c.stderrText();
  assert.match(text, /cross-env-rs-darwin-arm64 is not installed/);
  assert.match(text, /optionalDependencies/);
});

test('run() exits 1 with corruption hint when binary file is missing', () => {
  const c = captures();
  run('cross-env', {
    platform: 'darwin',
    arch: 'arm64',
    stderr: c.stderr,
    exit: c.exit,
    requireResolve: (id) => '/fake/node_modules/' + id,
    fileExists: () => false,
    spawn: () => assert.fail('spawn must not be reached when binary is missing'),
  });
  assert.deepEqual(c.exitCalls, [1]);
  const text = c.stderrText();
  assert.match(text, /binary not found at/);
  assert.match(text, /platform package may be corrupted/);
});

test('run() spawns the resolved binary, forwards argv, exits with child status', () => {
  const c = captures();
  let spawnCall = null;
  run('cross-env', {
    platform: 'darwin',
    arch: 'arm64',
    argv: ['FOO=bar', 'echo', 'hi'],
    stderr: c.stderr,
    exit: c.exit,
    requireResolve: (id) => '/pkgroot/' + id,
    fileExists: () => true,
    spawn: (bin, args, opts) => {
      spawnCall = { bin, args, opts };
      return { status: 0 };
    },
  });
  assert.ok(spawnCall.bin.endsWith('/cross-env'), `mode → bin path, got ${spawnCall.bin}`);
  assert.deepEqual(spawnCall.args, ['FOO=bar', 'echo', 'hi']);
  assert.deepEqual(spawnCall.opts, { stdio: 'inherit' });
  assert.deepEqual(c.exitCalls, [0]);
});

test('run() exits 1 with stderr message when spawn returns an error', () => {
  const c = captures();
  run('cross-env', {
    platform: 'darwin',
    arch: 'arm64',
    stderr: c.stderr,
    exit: c.exit,
    requireResolve: (id) => '/pkgroot/' + id,
    fileExists: () => true,
    spawn: () => ({ error: new Error('ENOENT') }),
  });
  assert.deepEqual(c.exitCalls, [1]);
  assert.match(c.stderrText(), /failed to spawn cross-env: ENOENT/);
});

test('run() forwards a child signal via sendSignal and does NOT call exit', () => {
  const c = captures();
  run('cross-env', {
    platform: 'darwin',
    arch: 'arm64',
    stderr: c.stderr,
    exit: c.exit,
    sendSignal: c.sendSignal,
    requireResolve: (id) => '/pkgroot/' + id,
    fileExists: () => true,
    spawn: () => ({ signal: 'SIGTERM' }),
  });
  assert.deepEqual(c.sendSignalCalls, ['SIGTERM']);
  assert.deepEqual(c.exitCalls, []);
});

test('run() exits 1 when child status is null and no signal is present', () => {
  const c = captures();
  run('cross-env', {
    platform: 'darwin',
    arch: 'arm64',
    stderr: c.stderr,
    exit: c.exit,
    requireResolve: (id) => '/pkgroot/' + id,
    fileExists: () => true,
    spawn: () => ({ status: null }),
  });
  assert.deepEqual(c.exitCalls, [1]);
});

test('run() picks the .exe suffix on win32', () => {
  const c = captures();
  let spawnedBin = null;
  run('cross-env', {
    platform: 'win32',
    arch: 'x64',
    stderr: c.stderr,
    exit: c.exit,
    requireResolve: (id) => 'C:\\fake\\' + id,
    fileExists: () => true,
    spawn: (bin) => { spawnedBin = bin; return { status: 0 }; },
  });
  assert.ok(
    spawnedBin.endsWith('cross-env.exe'),
    `expected .exe suffix on win32, got ${spawnedBin}`,
  );
});

test('run() picks bare mode name on non-win32 platforms', () => {
  const c = captures();
  let spawnedBin = null;
  run('cross-env-shell', {
    platform: 'linux',
    arch: 'x64',
    libc: 'gnu',
    stderr: c.stderr,
    exit: c.exit,
    requireResolve: (id) => '/fake/' + id,
    fileExists: () => true,
    spawn: (bin) => { spawnedBin = bin; return { status: 0 }; },
  });
  assert.ok(
    spawnedBin.endsWith('/cross-env-shell'),
    `expected non-suffixed bin on linux, got ${spawnedBin}`,
  );
  assert.ok(
    !spawnedBin.endsWith('.exe'),
    'should not append .exe on linux',
  );
});
