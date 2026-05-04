# cross-env-rs

> Rust port of [cross-env](https://github.com/kentcdodds/cross-env). Drop-in replacement that ships native binaries for instant startup.

## Install

```sh
npm install --save-dev cross-env-rs
```

The package installs `cross-env` and `cross-env-shell` binaries into `node_modules/.bin/`, so your existing `package.json` scripts work unchanged.

## Usage

```json
{
  "scripts": {
    "build": "cross-env NODE_ENV=production webpack",
    "test": "cross-env-shell FOO=bar 'jest --config $FOO.config.js'"
  }
}
```

Set environment variables before a command, cross-platform — same syntax as upstream `cross-env`. `cross-env-shell` runs the command through a shell (`sh -c` on Unix, `cmd /d /s /c` on Windows), enabling pipes and redirects.

Native binaries for macOS, Linux, and Windows on x64 and arm64. `win32-arm64` is not yet supported.

## Why

`cross-env` is one of the most-installed npm packages and has been declared "complete" by its author. `cross-env-rs` keeps the same UX while replacing the Node wrapper with a native Rust binary — measurably faster cold start (~20× wrapper-only, ~2× when the child is itself Node) and lower memory footprint.

## License

MIT — same as upstream `cross-env`.
