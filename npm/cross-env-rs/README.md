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

## How it works

The native binary is shipped via [`optionalDependencies`](https://docs.npmjs.com/cli/v10/configuring-npm/package-json#optionaldependencies) — the same pattern as `esbuild`, `swc`, and `biome`. npm picks the right one for your platform and CPU at install time. No postinstall scripts; no network calls during `npm install` other than the standard package fetches.

## Supported platforms

| Platform              | Package                          |
| --------------------- | -------------------------------- |
| macOS arm64           | `cross-env-rs-darwin-arm64`      |
| macOS x64             | `cross-env-rs-darwin-x64`        |
| Linux x64 (glibc)     | `cross-env-rs-linux-x64-gnu`     |
| Linux x64 (musl)      | `cross-env-rs-linux-x64-musl`    |
| Linux arm64 (glibc)   | `cross-env-rs-linux-arm64-gnu`   |
| Linux arm64 (musl)    | `cross-env-rs-linux-arm64-musl`  |
| Windows x64           | `cross-env-rs-windows-x64`       |

`win32-arm64` is not supported in the initial release. Track progress in the [repo](https://github.com/Segodnya/cross-env-rs).

## Why

`cross-env` is one of the most-installed npm packages and has been declared "complete" by its author. `cross-env-rs` keeps the same UX while replacing the Node wrapper with a native Rust binary — measurably faster cold start (~20× wrapper-only, ~2× when the child is itself Node) and lower memory footprint.

## License

MIT — same as upstream `cross-env`.
