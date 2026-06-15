#!/usr/bin/env bash
# Package the ileap skill into a zip that can be uploaded to Claude.ai
# (Settings > Capabilities > Skills) or unpacked into any agent's skills dir.
#
# Usage: scripts/package-skill.sh
# Output: dist/ileap-skill.zip
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SKILL_SRC="$REPO_ROOT/.agents/skills/ileap"
DIST="$REPO_ROOT/dist"
STAGING="$(mktemp -d)"
trap 'rm -rf "$STAGING"' EXIT

[ -f "$SKILL_SRC/SKILL.md" ] || { echo "error: $SKILL_SRC/SKILL.md not found" >&2; exit 1; }
[ -f "$SKILL_SRC/SCHEMAS.md" ] || { echo "error: $SKILL_SRC/SCHEMAS.md not found" >&2; exit 1; }

mkdir -p "$DIST"
cp -R "$SKILL_SRC" "$STAGING/ileap"

if ls "$STAGING/ileap/bin/ileap-"* >/dev/null 2>&1; then
  echo "bundled binaries:"
  ls -lh "$STAGING/ileap/bin/"
else
  echo "error: no prebuilt binaries in $SKILL_SRC/bin/ — the skill uses" >&2
  echo "       prebuilt binaries exclusively. Run scripts/build-skill-binaries.sh first." >&2
  exit 1
fi

rm -f "$DIST/ileap-skill.zip"
(cd "$STAGING" && zip -qr "$DIST/ileap-skill.zip" ileap)
echo "wrote $DIST/ileap-skill.zip ($(du -h "$DIST/ileap-skill.zip" | cut -f1 | tr -d ' '))"
