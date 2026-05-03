#!/usr/bin/env bash
# Minimal benchmark of cross-env-rs vs upstream cross-env.
# Compares wrapper startup time, realistic node-launch time, and peak RSS.
#
# Requirements: hyperfine, npm/node, /usr/bin/time. Upstream cross-env is
# installed into a temp directory if not found.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BENCH_DIR="${BENCH_DIR:-/tmp/cer-bench}"
RUST_BIN="$ROOT/target/release/cross-env"
NODE="$(command -v node)"

if [ ! -x "$RUST_BIN" ]; then
  echo "Building cross-env-rs in release mode..."
  ( cd "$ROOT" && cargo build --release --quiet )
fi

if [ ! -x "$BENCH_DIR/node_modules/.bin/cross-env" ]; then
  echo "Installing upstream cross-env into $BENCH_DIR ..."
  mkdir -p "$BENCH_DIR"
  npm install --silent --no-audit --no-fund --prefix "$BENCH_DIR" cross-env >/dev/null
fi

UPSTREAM_BIN="$BENCH_DIR/node_modules/.bin/cross-env"

# Resolve upstream version for the report
UPSTREAM_VER="$(node -p "require('$BENCH_DIR/node_modules/cross-env/package.json').version")"
RUST_VER="$($RUST_BIN --version)"
HOST="$(uname -sm)"
NODE_VER="$($NODE --version)"

echo
echo "=========================================="
echo " cross-env-rs benchmarks"
echo "=========================================="
echo " host:         $HOST"
echo " node:         $NODE_VER"
echo " upstream:     cross-env@$UPSTREAM_VER"
echo " cross-env-rs: $RUST_VER"
echo "=========================================="

echo
echo "[A] Wrapper overhead (child = /usr/bin/true) — measures pure wrapper cost"
hyperfine --shell=none --warmup 10 --runs 100 \
  -n upstream     "$UPSTREAM_BIN FOO=bar /usr/bin/true" \
  -n cross-env-rs "$RUST_BIN FOO=bar /usr/bin/true" \
  --export-markdown /tmp/cer-bench-true.md

echo
echo "[B] Realistic (child = node -e 0) — typical usage pattern"
hyperfine --shell=none --warmup 10 --runs 50 \
  -n upstream     "$UPSTREAM_BIN FOO=bar $NODE -e 0" \
  -n cross-env-rs "$RUST_BIN FOO=bar $NODE -e 0" \
  --export-markdown /tmp/cer-bench-node.md

echo
echo "[C] Peak RSS (max of 5 runs, wrapper-only scenario)"
rss_mb() {
  local bin="$1"
  local max=0
  for _ in 1 2 3 4 5; do
    local v
    v=$(/usr/bin/time -l "$bin" FOO=bar /usr/bin/true 2>&1 | awk '/maximum resident/{print $1}')
    if [ "$v" -gt "$max" ]; then max=$v; fi
  done
  awk -v b="$max" 'BEGIN { printf "%.1f MB", b/1024/1024 }'
}
echo "  upstream:     $(rss_mb "$UPSTREAM_BIN")"
echo "  cross-env-rs: $(rss_mb "$RUST_BIN")"

echo
echo "[D] Binary / install footprint"
RUST_SIZE=$(stat -f "%z" "$RUST_BIN" 2>/dev/null || stat -c "%s" "$RUST_BIN")
UPSTREAM_PKG_SIZE=$(du -sk "$BENCH_DIR/node_modules/cross-env" | awk '{print $1*1024}')
awk -v r="$RUST_SIZE"     'BEGIN { printf "  cross-env-rs binary:     %.0f KB\n", r/1024 }'
awk -v u="$UPSTREAM_PKG_SIZE" 'BEGIN { printf "  upstream pkg (no Node):  %.0f KB (Node runtime ~30 MB extra)\n", u/1024 }'
