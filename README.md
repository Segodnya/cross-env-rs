# cross-env-rs

> Rust port of [cross-env](https://github.com/kentcdodds/cross-env). Drop-in replacement that ships native binaries for instant startup.

[![npm](https://img.shields.io/npm/v/cross-env-rs)](https://www.npmjs.com/package/cross-env-rs)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

## Why

`cross-env` is one of the most-installed npm packages (~1M projects) and has been declared "complete" by its author. Its specification is stable and well-defined, which makes it an excellent candidate for a Rust port:

- **Faster cold start** — native binary, no Node startup overhead.
- **Drop-in compatibility** — installs binaries named `cross-env` and `cross-env-shell` into `node_modules/.bin/`, so existing `package.json` scripts keep working without changes.
- **Modern distribution** — platform-specific native packages via npm `optionalDependencies` (the same pattern as `esbuild`, `swc`, `biome`). No postinstall scripts, no network at install time.

## Supported platforms (first release)

| Platform              | npm package                          |
| --------------------- | ------------------------------------ |
| macOS arm64           | `cross-env-rs-darwin-arm64`          |
| macOS x64             | `cross-env-rs-darwin-x64`            |
| Linux x64 (glibc)     | `cross-env-rs-linux-x64-gnu`         |
| Linux x64 (musl)      | `cross-env-rs-linux-x64-musl`        |
| Linux arm64 (glibc)   | `cross-env-rs-linux-arm64-gnu`       |
| Linux arm64 (musl)    | `cross-env-rs-linux-arm64-musl`      |
| Windows x64           | `cross-env-rs-windows-x64`           |

> **Not yet supported**: `win32-arm64`. Tracked as a follow-up; if you need it, please open an issue.

## Benchmarks

Measured on macOS arm64 (Apple Silicon, M-series), Node 22, against upstream `cross-env@7.0.3`. Reproduce with `bash scripts/bench.sh`.

| Scenario                                              | upstream `cross-env` | `cross-env-rs`   | Improvement       |
| ----------------------------------------------------- | -------------------: | ---------------: | ----------------: |
| Wrapper overhead (`cross-env FOO=bar /usr/bin/true`)  |    43.0 ± 0.6 ms     |   2.2 ± 0.1 ms   | **~20× faster**   |
| Realistic (`cross-env FOO=bar node -e 0`)             |    73.6 ± 0.6 ms     |  33.4 ± 0.6 ms   | **~2.2× faster**  |
| Peak RSS (wrapper-only)                               |       46 MB          |     1.3 MB       | **~35× less**     |
| On-disk size                                          |   ~64 KB pkg + Node  |     338 KB       | self-contained    |

The 20× wrapper-only gap closes to ~2× when the child is itself a Node process, because Node's ~30 ms cold start is paid by both. The win surfaces clearly in CI scripts that fan out many `npm run`/`yarn` invocations: each one shaves ~40 ms.

Numbers will be tracked per release as performance evolves. Different CPU families and shell pipelines produce different absolute numbers — the relative gap is what matters.

## Repository layout

```
crates/cross-env-rs/             # Rust crate (two binaries: cross-env, cross-env-shell)
crates/test-fixtures/print-env/  # Test-only binary used by integration tests (publish = false)
npm/cross-env-rs/                # main npm package, JS shim that picks the right native binary
npm/cross-env-rs-<platform>/     # 7 platform-specific native binary packages
.githooks/                       # pre-commit and pre-push: fmt, clippy, cargo test
scripts/                         # local cross-build and benchmark helpers
.github/workflows/               # release-only workflow
```

## Development

After cloning, wire up the local git hooks once:

```sh
git config core.hooksPath .githooks
```

This enables:

- **`pre-commit`**: `cargo fmt --check` + `cargo test --workspace` + JS shim tests when present.
- **`pre-push`**: same as `pre-commit` plus `cargo clippy --workspace --all-targets -- -D warnings`.

Two test surfaces:

- **Rust** — `cargo test --workspace` covers the binary behaviour (parsing, expansion, executor adapters, end-to-end `cross-env` / `cross-env-shell` invocations). The integration suite uses a small workspace-internal fixture (`crates/test-fixtures/print-env`) that `escargot` builds on demand.
- **JS shim** — `node --test npm/cross-env-rs/test/*.test.js` covers `lib/run.js` (platform→package mapping, libc detection, error paths) by injecting fakes via the seam-based `run({ platform, arch, libc, spawn, exit, ... })` API. No real child process is spawned; no real fs/require lookups.

Both surfaces are wired into the git hooks, so `git commit` and `git push` run them automatically.

## Contributing

Not yet open for contributions. A `CONTRIBUTING.md` will be added later.

## License

MIT — same as upstream `cross-env`. See [`LICENSE`](./LICENSE).

## Acknowledgements

This project is a port. All credit for the original design and decade of cross-platform tweaks goes to [Kent C. Dodds](https://github.com/kentcdodds) and the [cross-env contributors](https://github.com/kentcdodds/cross-env/graphs/contributors).
