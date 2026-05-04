# cross-env-rs

> Rust port of [cross-env](https://github.com/kentcdodds/cross-env). Drop-in replacement that ships native binaries for instant startup.

[![npm](https://img.shields.io/npm/v/cross-env-rs)](https://www.npmjs.com/package/cross-env-rs)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

## Status

`cross-env-rs@0.1.1` is live on npm with binary provenance for all 7 platform packages. The core port is functional — `KEY=VAL` parsing, `which`-based binary lookup, `cross-env-shell`, exit-code propagation, Unix signal forwarding. See [Compatibility](#compatibility) for the per-feature matrix. The conformance test suite (ported from upstream Jest tests) is the active work track.

## Why

`cross-env` is one of the most-installed npm packages (~1M projects) and has been declared "complete" by its author. Its specification is stable and well-defined, which makes it an excellent candidate for a Rust port:

- **Faster cold start** — native binary, no Node startup overhead.
- **Drop-in compatibility** — installs binaries named `cross-env` and `cross-env-shell` into `node_modules/.bin/`, so existing `package.json` scripts keep working without changes.
- **Modern distribution** — platform-specific native packages via npm `optionalDependencies` (the same pattern as `esbuild`, `swc`, `biome`). No postinstall scripts, no network at install time.

## Goals

- Match upstream `cross-env` behaviour bit-for-bit on its test suite (including Windows `.cmd`/`.bat` lookup, signal forwarding, exit-code propagation).
- Measurably faster startup than the Node-based original (target: ≥2× via `hyperfine`).
- Zero-postinstall, offline-friendly install path.
- MIT-licensed, like upstream.

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

## Compatibility

This package is a port — every release is graded against upstream `cross-env` behaviour. Each row below is one observable feature; status moves to ✅ only when an automated test exists and passes.

**Legend:** ✅ implemented & tested · ⚠️ partial or platform gap · ❓ likely works, not yet test-verified · ❌ not yet implemented.

### CLI parsing

| # | Feature | Status | Test |
| - | ------- | :----: | ---- |
| 1 | `KEY=VAL` pairs | ✅ | `row_01_kv_pair_passes_var` |
| 2 | Multiple env vars before command | ✅ | `row_02_multiple_kv_pairs` |
| 3 | Empty value (`FOO=`) | ✅ | `row_03_empty_value_is_set_but_empty` |
| 4 | Value contains `=` (`FOO=a=b`) | ✅ | `row_04_value_contains_equals` |
| 5 | `--` argument terminator | ✅ | `row_05_*` |
| 6 | `--version` / `--help` flags | ✅ | `row_06_*` |

### Variable expansion

| # | Feature | Status | Test |
| - | ------- | :----: | ---- |
| 7 | `$VAR` / `${VAR}` substitution | ✅ | `row_07_*` |
| 8 | `%VAR%` (Windows-style) auto-translate | ❌ | — |
| 9 | PATH-list separator `:` ↔ `;` auto-translate | ❌ | — |

### Process execution

| #  | Feature | Status | Test |
| -- | ------- | :----: | ---- |
| 10 | Exit code propagation | ✅ | `row_10_exit_code_propagation` |
| 11 | Signal-killed exit code (128 + sig) | ⚠️ | `row_11_*_unix` |
| 12 | SIGINT / SIGTERM forwarding to child | ✅ | `row_12_pgroup_signal_kills_child_unix` |
| 13 | Stdio inheritance (stdin/stdout/stderr) | ✅ | `row_13_stdin_inherits_through_cross_env` |
| 14 | Parent env passthrough + per-call override | ✅ | `row_14_*` |
| 15 | `cross-env` (no shell) vs `cross-env-shell` | ✅ | `row_15_*` |

> **Row 11 caveat:** Unix paths return `128 + signal` as upstream does. Windows currently returns `1` for any abnormal termination — tracked as a follow-up.

> **Row 12 caveat:** signal delivery follows OS process-group semantics (matching upstream Node `cross-env`). Terminal `Ctrl+C` reaches both wrapper and child. Programmatic `kill <wrapper-pid>` (without targeting the group) does not propagate to the child — the wrapper dies, the child is orphaned. Use `kill -- -<wrapper-pgid>` for programmatic group kills.

### Platform

| #  | Feature | Status | Test |
| -- | ------- | :----: | ---- |
| 16 | Windows `.cmd` / `.bat` / `.ps1` + PATHEXT | ❓ | — |
| 17 | musl/glibc autodispatch (JS shim) | ❓ | manual smoke only |

### JS shim

| #  | Feature | Status | Test |
| -- | ------- | :----: | ---- |
| 18 | Unsupported platform error message | ❓ | — |

Each row will be backed by an automated test in `crates/cross-env-rs/tests/integration.rs` (Rust binary behaviour) or `npm/cross-env-rs/test/shim.test.js` (JS shim). Rule: a PR without a test does not move a row out of ❓.

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

## Roadmap

1. **0.1.x (current):** core port shipped — `KEY=VAL` parsing, `which`-based binary lookup, `cross-env-shell`, exit-code propagation, Unix signal forwarding. Published to npm under `cross-env-rs` with provenance for 7 platform packages.
2. **Conformance track (active):** integration tests in `crates/cross-env-rs/tests/integration.rs` covering each row of the [Compatibility](#compatibility) matrix; close the ❌ gaps (variable expansion, `--` terminator, `--version`/`--help`) and verify all ❓ rows.
3. **CI:** release-only `release-please` workflow; correctness checks live in local git hooks (no CI minutes spent on lint/test on every PR).
4. **Future:** Windows signal-forwarding parity (row 11), `win32-arm64` platform package, performance polish.

## Repository layout

```
crates/cross-env-rs/             # Rust crate (two binaries: cross-env, cross-env-shell)
crates/test-fixtures/print-env/  # Test-only binary used by integration tests (publish = false)
npm/cross-env-rs/                # main npm package, JS shim that picks the right native binary
npm/cross-env-rs-<platform>/     # 7 platform-specific native binary packages
.githooks/                       # pre-commit and pre-push: fmt, clippy, cargo test
scripts/                         # local cross-build, benchmark, conformance helpers
.github/workflows/               # release-only workflow (no per-PR matrix)
```

## Development

After cloning, wire up the local git hooks once:

```sh
git config core.hooksPath .githooks
```

This enables:

- **`pre-commit`**: `cargo fmt --check` + `cargo test --workspace` + JS shim tests when present.
- **`pre-push`**: same as `pre-commit` plus `cargo clippy --workspace --all-targets -- -D warnings`.

Tests are pure Rust — `cargo test --workspace` is enough. The integration suite uses a small workspace-internal fixture (`crates/test-fixtures/print-env`) that `escargot` builds on demand.

## Contributing

Not yet open for contributions. Once the conformance track lands, see `CONTRIBUTING.md`.

## License

MIT — same as upstream `cross-env`. See [`LICENSE`](./LICENSE).

## Acknowledgements

This project is a port. All credit for the original design and decade of cross-platform tweaks goes to [Kent C. Dodds](https://github.com/kentcdodds) and the [cross-env contributors](https://github.com/kentcdodds/cross-env/graphs/contributors).
