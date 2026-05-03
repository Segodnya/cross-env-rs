#!/usr/bin/env bash
# Day 0 — publish all eight cross-env-rs placeholder packages.
#
# Each invocation will prompt for a fresh OTP because the npm account
# is configured with `auth-and-writes` 2FA. Keep your authenticator
# app open while running this script.
#
# Platform packages are published first; the umbrella `cross-env-rs`
# package goes last so that a hypothetical observer who races the
# publish window cannot install a main package whose optional deps
# would 404.
#
# Re-running is safe: npm rejects a duplicate version, so already-
# published packages are skipped (the script keeps going).

set -u
set -o pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NPM_DIR="$REPO_ROOT/npm"
DIST_TAG="placeholder"

PLATFORM_PKGS=(
  cross-env-rs-darwin-arm64
  cross-env-rs-darwin-x64
  cross-env-rs-linux-x64-gnu
  cross-env-rs-linux-x64-musl
  cross-env-rs-linux-arm64-gnu
  cross-env-rs-linux-arm64-musl
  cross-env-rs-win32-x64
)

publish_one() {
  local pkg="$1"
  local dir="$NPM_DIR/$pkg"

  if [ ! -f "$dir/package.json" ]; then
    echo "  ✗ skip: $dir/package.json not found"
    return 1
  fi

  echo
  echo "→ Publishing $pkg (tag=$DIST_TAG)"
  ( cd "$dir" && npm publish --tag "$DIST_TAG" --access public )
  local status=$?
  if [ $status -ne 0 ]; then
    echo "  ✗ $pkg failed (exit $status). Continuing — re-run the script later if needed."
  else
    echo "  ✓ $pkg published"
  fi
  return 0
}

echo "Logged in as: $(npm whoami)"
echo "Registry:     $(npm config get registry)"
echo "Working dir:  $NPM_DIR"
echo
echo "About to publish 8 placeholder packages with --tag $DIST_TAG."
echo "You will be prompted for an OTP for each package."
read -r -p "Continue? [y/N] " confirm
case "$confirm" in
  y|Y|yes|YES) ;;
  *) echo "Aborted."; exit 1 ;;
esac

for pkg in "${PLATFORM_PKGS[@]}"; do
  publish_one "$pkg"
done

publish_one "cross-env-rs"

echo
echo "Done. Verify with:"
echo "  npm view cross-env-rs dist-tags"
echo "  npm view cross-env-rs-darwin-arm64 dist-tags"
