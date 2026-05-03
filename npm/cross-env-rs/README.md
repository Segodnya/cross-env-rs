# cross-env-rs

> Rust port of [cross-env](https://github.com/kentcdodds/cross-env). Drop-in replacement that ships native binaries for instant startup.

## Status: placeholder

This is a **name-reservation placeholder**. The first real release (`0.1.0`) is in active development.

- Track progress and milestones: <https://github.com/Segodnya/cross-env-rs>
- Original `cross-env` (still works fine): <https://github.com/kentcdodds/cross-env>

## What this will be

When released, `cross-env-rs` will:

- Install the `cross-env` and `cross-env-shell` binaries into `node_modules/.bin/`, so existing `package.json` scripts keep working unchanged.
- Ship platform-native Rust binaries via `optionalDependencies` (the same pattern used by `esbuild`, `swc`, and `biome`) — no postinstall scripts, no network at install time.
- Match the upstream `cross-env` behaviour bit-for-bit on the test suite, including Windows `.cmd`/`.bat` lookup, signal forwarding, and exit codes.
- Start measurably faster than the Node-based original.

## Roadmap

See [`README.md` in the repo](https://github.com/Segodnya/cross-env-rs#readme) for the full plan and milestones.

## License

MIT — same as upstream `cross-env`.
