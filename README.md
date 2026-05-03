# cross-env-rs

> Rust port of [cross-env](https://github.com/kentcdodds/cross-env). Drop-in replacement that ships native binaries for instant startup.

[![placeholder](https://img.shields.io/badge/status-placeholder-orange)](https://www.npmjs.com/package/cross-env-rs)

## Status

This repository hosts the in-progress port. **No release yet** — the npm packages currently published under `cross-env-rs` and `cross-env-rs-<platform>` are name-reservation placeholders (`0.0.1-placeholder.0`, `--tag placeholder`). The first usable release will be `0.1.0`.

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

## Roadmap

1. **Day 0 (current):** placeholder publish to reserve names.
2. **0.1.0:** core port — `KEY=VAL` parsing, `which`-based binary lookup, signal forwarding, `cross-env-shell`, conformance test suite ported from upstream Jest tests.
3. **CI:** minimal release-only `release-please` workflow; correctness checks live in local git hooks (no CI minutes spent on lint/test on every PR).
4. **0.2.x+:** performance polish, additional platforms (`win32-arm64`), broader edge-case coverage.

## Repository layout

```
crates/cross-env-rs/         # Rust crate (two binaries: cross-env, cross-env-shell)
npm/cross-env-rs/            # main npm package, JS shim that picks the right native binary
npm/cross-env-rs-<platform>/ # 7 platform-specific native binary packages
.githooks/                   # pre-commit and pre-push: fmt, clippy, cargo test
scripts/                     # local cross-build, benchmark, conformance helpers
.github/workflows/           # release-only workflow (no per-PR matrix)
```

## Contributing

Not yet open for contributions — the scaffold is incoming. Once `0.1.0` is released, see `CONTRIBUTING.md`.

## License

MIT — same as upstream `cross-env`. See [`LICENSE`](./LICENSE).

## Acknowledgements

This project is a port. All credit for the original design and decade of cross-platform tweaks goes to [Kent C. Dodds](https://github.com/kentcdodds) and the [cross-env contributors](https://github.com/kentcdodds/cross-env/graphs/contributors).
