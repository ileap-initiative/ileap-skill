#!/usr/bin/env bash
# Build prebuilt `ileap` binaries for the ileap skill bundle from the
# LOCAL repository source, and place them in ileap/bin/.
#
# The skill uses prebuilt binaries exclusively (no Rust toolchain at runtime),
# so this must be run before packaging the skill with scripts/package-skill.sh.
#
# Builds:
#   - ileap-Linux-x86_64 and ileap-Linux-aarch64 (static musl, via Docker)
#   - ileap-<host OS>-<host arch> natively, if cargo is available
#
# Usage: scripts/build-skill-binaries.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$REPO_ROOT/ileap/bin"
mkdir -p "$BIN_DIR"

build_linux() {
  local platform="$1" arch="$2" out
  out="$(mktemp -d)"
  echo "building Linux $arch from local source (this can take several minutes)..."
  # rustls (ring) needs a C compiler (gcc) but no OpenSSL.
  docker run --rm --platform "$platform" \
    -v "$REPO_ROOT:/src:ro" -v "$out:/out" \
    -e CARGO_TARGET_DIR=/build \
    rust:alpine \
    sh -c "apk add -q musl-dev gcc && \
           cargo install --path /src/cli --locked --root /out"
  cp "$out/bin/ileap" "$BIN_DIR/ileap-Linux-$arch"
  rm -rf "$out"
  echo "wrote $BIN_DIR/ileap-Linux-$arch"
}

build_linux linux/amd64 x86_64
build_linux linux/arm64 aarch64

if command -v cargo >/dev/null 2>&1; then
  echo "building native host binary..."
  cargo build --release --locked --manifest-path "$REPO_ROOT/cli/Cargo.toml"
  cp "$REPO_ROOT/target/release/ileap" "$BIN_DIR/ileap-$(uname -s)-$(uname -m)"
  echo "wrote $BIN_DIR/ileap-$(uname -s)-$(uname -m)"
else
  echo "note: cargo not found, skipping native host binary" >&2
fi

ls -lh "$BIN_DIR"
