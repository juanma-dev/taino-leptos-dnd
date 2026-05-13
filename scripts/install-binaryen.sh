#!/usr/bin/env bash
# One-shot installer: latest binaryen release into ~/.local/share/binaryen,
# with a symlink to ~/.cargo/bin/wasm-opt (which is already on PATH).
#
# Run this when the apt-shipped binaryen is too old for the flags
# `scripts/size-check.sh` passes (notably `--enable-bulk-memory-opt`,
# added in binaryen 119).
#
# No sudo. Idempotent: re-run to upgrade.

set -euo pipefail

ARCH=$(uname -m)
if [ "$ARCH" != "x86_64" ]; then
    echo "Unsupported arch: $ARCH (this script only handles x86_64-linux)" >&2
    exit 1
fi

# Read latest release tag from the GitHub API. Use python (always present on
# Ubuntu) to parse JSON, avoiding fragile quote-mangling with awk/cut.
LATEST=$(curl -fsSL https://api.github.com/repos/WebAssembly/binaryen/releases/latest \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["tag_name"])')

if [ -z "$LATEST" ]; then
    echo "Failed to read latest binaryen tag." >&2
    exit 1
fi

URL="https://github.com/WebAssembly/binaryen/releases/download/${LATEST}/binaryen-${LATEST}-x86_64-linux.tar.gz"
echo "==> Installing binaryen ${LATEST}"
echo "    from: ${URL}"

mkdir -p "$HOME/.local/share" "$HOME/.cargo/bin"
rm -rf "$HOME/.local/share/binaryen" "$HOME/.local/share/binaryen-tmp"
mkdir -p "$HOME/.local/share/binaryen-tmp"

TARBALL=$(mktemp -t binaryen.XXXX.tar.gz)
trap 'rm -f "$TARBALL"' EXIT
curl -fSL --progress-bar "$URL" -o "$TARBALL"

tar -xzf "$TARBALL" -C "$HOME/.local/share/binaryen-tmp" --strip-components=1
mv "$HOME/.local/share/binaryen-tmp" "$HOME/.local/share/binaryen"

ln -sf "$HOME/.local/share/binaryen/bin/wasm-opt" "$HOME/.cargo/bin/wasm-opt"

echo
echo "==> Installed:"
which wasm-opt
wasm-opt --version
echo
echo "==> Flag support sanity check:"
wasm-opt --help 2>&1 | grep -E "enable-bulk-memory" || true
