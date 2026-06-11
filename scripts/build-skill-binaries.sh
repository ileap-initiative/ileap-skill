#!/usr/bin/env bash
# Build static Linux binaries for the ileap-cli skill bundle using Docker,
# and place them in .agents/skills/ileap-cli/bin/.
#
# These are the binaries the skill uses in sandboxed agent environments
# (e.g. Claude.ai) where no Rust toolchain is available.
#
# Usage: scripts/build-skill-binaries.sh [git-url]
#   git-url defaults to https://github.com/sine-fdn/ileap-cli-test
set -euo pipefail

GIT_URL="${1:-https://github.com/sine-fdn/ileap-cli-test}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$REPO_ROOT/.agents/skills/ileap-cli/bin"
mkdir -p "$BIN_DIR"

build() {
  local platform="$1" arch="$2" out
  out="$(mktemp -d)"
  echo "building $arch (this can take several minutes)..."
  docker run --rm --platform "$platform" -v "$out:/out" -e OPENSSL_STATIC=1 rust:alpine \
    sh -c "apk add -q musl-dev git pkgconfig openssl-dev openssl-libs-static && \
           cargo install --git $GIT_URL --locked ileap-cli --root /out"
  cp "$out/bin/ileap" "$BIN_DIR/ileap-Linux-$arch"
  rm -rf "$out"
  echo "wrote $BIN_DIR/ileap-Linux-$arch"
}

build linux/amd64 x86_64
build linux/arm64 aarch64
ls -lh "$BIN_DIR"
