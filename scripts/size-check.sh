#!/usr/bin/env bash
# Build the sortable-list example for wasm32, run it through wasm-bindgen
# and wasm-opt, then assert the gzipped binary fits inside a size budget.
#
# Local prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install --locked wasm-bindgen-cli@<lockfile version>
#   apt-get install binaryen   # or: brew install binaryen
#
# Environment variables:
#   BUDGET_GZIP_KB  override the gzipped-size budget (default: 400)
#   PKG             override the example package (default: example-sortable-list)
#
# CI: see .github/workflows/ci.yml `size-budget` job, which installs the
# tools via taiki-e/install-action before invoking this script.

set -euo pipefail

BUDGET_GZIP_KB=${BUDGET_GZIP_KB:-400}
PKG=${PKG:-example-sortable-list}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET_DIR="$REPO_ROOT/target/wasm32-unknown-unknown/release"
OUT_DIR="$REPO_ROOT/target/size-check"

cd "$REPO_ROOT"

echo "==> Building $PKG (release, wasm32)..."
cargo build -p "$PKG" --target wasm32-unknown-unknown --release

# Binary artifacts keep the package name verbatim (hyphens included).
# Library `.rlib`s would get underscores; binaries don't.
RAW_WASM="$TARGET_DIR/${PKG}.wasm"

if [ ! -f "$RAW_WASM" ]; then
    echo "::error::Expected $RAW_WASM not found after build."
    exit 1
fi

mkdir -p "$OUT_DIR"

# Force a stable output name so we don't have to guess wasm-bindgen's
# hyphen-vs-underscore convention.
OUT_NAME="bundle"

echo "==> Running wasm-bindgen..."
wasm-bindgen \
    --target web \
    --out-dir "$OUT_DIR" \
    --out-name "$OUT_NAME" \
    --no-typescript \
    "$RAW_WASM"

BG_WASM="$OUT_DIR/${OUT_NAME}_bg.wasm"
OPTIMIZED="$OUT_DIR/optimized.wasm"

echo "==> Running wasm-opt -Oz..."
# Modern rustc + wasm-bindgen emit wasm that uses post-MVP features.
# wasm-opt validates strictly, so we have to enable the matching set.
wasm-opt \
    -Oz \
    --enable-bulk-memory \
    --enable-bulk-memory-opt \
    --enable-mutable-globals \
    --enable-nontrapping-float-to-int \
    --enable-sign-ext \
    --enable-reference-types \
    --enable-multivalue \
    --strip-debug \
    --strip-producers \
    -o "$OPTIMIZED" "$BG_WASM"

# stat invocation differs between Linux (-c%s) and macOS (-f%z).
file_size() {
    stat -c%s "$1" 2>/dev/null || stat -f%z "$1"
}

RAW_BYTES=$(file_size "$BG_WASM")
OPT_BYTES=$(file_size "$OPTIMIZED")
GZIP_BYTES=$(gzip -c "$OPTIMIZED" | wc -c | tr -d ' ')

RAW_KB=$((RAW_BYTES / 1024))
OPT_KB=$((OPT_BYTES / 1024))
GZIP_KB=$((GZIP_BYTES / 1024))

echo
echo "================== Size report =================="
printf "  wasm-bindgen output: %7d bytes (%4d KB)\n" "$RAW_BYTES" "$RAW_KB"
printf "  wasm-opt -Oz output: %7d bytes (%4d KB)\n" "$OPT_BYTES" "$OPT_KB"
printf "  gzipped:             %7d bytes (%4d KB)\n" "$GZIP_BYTES" "$GZIP_KB"
printf "  budget (gzip):       %7s        (%4d KB)\n" "—" "$BUDGET_GZIP_KB"
echo "================================================="
echo

if [ "$GZIP_KB" -gt "$BUDGET_GZIP_KB" ]; then
    echo "::error::Gzipped wasm (${GZIP_KB} KB) exceeds budget (${BUDGET_GZIP_KB} KB)"
    echo
    echo "Options:"
    echo "  1. Reduce binary size (remove a dep, swap for a smaller one,"
    echo "     check 'cargo bloat' / 'twiggy top')."
    echo "  2. If the increase is justified, bump BUDGET_GZIP_KB in"
    echo "     .github/workflows/ci.yml with a note in the PR description."
    exit 1
fi

echo "OK — under budget."
