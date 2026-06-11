#!/usr/bin/env bash
# Package the ileap-cli skill into a zip that can be uploaded to Claude.ai
# (Settings > Capabilities > Skills) or unpacked into any agent's skills dir.
#
# Usage: scripts/package-skill.sh
# Output: dist/ileap-cli-skill.zip
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SKILL_SRC="$REPO_ROOT/.agents/skills/ileap-cli"
DIST="$REPO_ROOT/dist"
STAGING="$(mktemp -d)"
trap 'rm -rf "$STAGING"' EXIT

[ -f "$SKILL_SRC/SKILL.md" ] || { echo "error: $SKILL_SRC/SKILL.md not found" >&2; exit 1; }
[ -f "$SKILL_SRC/SCHEMAS.md" ] || { echo "error: $SKILL_SRC/SCHEMAS.md not found" >&2; exit 1; }

mkdir -p "$DIST"
cp -R "$SKILL_SRC" "$STAGING/ileap-cli"

if ls "$STAGING/ileap-cli/bin/ileap-"* >/dev/null 2>&1; then
  echo "bundled binaries:"
  ls -lh "$STAGING/ileap-cli/bin/"
else
  echo "warning: no prebuilt binaries in $SKILL_SRC/bin/ — the skill will" >&2
  echo "         require a Rust toolchain at runtime (not available on Claude.ai)." >&2
fi

rm -f "$DIST/ileap-cli-skill.zip"
(cd "$STAGING" && zip -qr "$DIST/ileap-cli-skill.zip" ileap-cli)
echo "wrote $DIST/ileap-cli-skill.zip ($(du -h "$DIST/ileap-cli-skill.zip" | cut -f1 | tr -d ' '))"
