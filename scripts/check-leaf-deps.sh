#!/usr/bin/env bash
# Fail if leaf profile pulls tokio or libp2p (tech-stack G3 / dep budget).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "Checking mycelia-leaf dependency tree..."
TREE="$(cargo tree -p mycelia-leaf --edges normal 2>/dev/null || cargo tree -p mycelia-leaf)"
if echo "$TREE" | grep -E '(^|[^a-z])tokio([^a-z]|$)' >/dev/null; then
  echo "ERROR: tokio found in mycelia-leaf dependency tree"
  echo "$TREE"
  exit 1
fi
if echo "$TREE" | grep -E 'libp2p' >/dev/null; then
  echo "ERROR: libp2p found in mycelia-leaf dependency tree"
  echo "$TREE"
  exit 1
fi
echo "OK: no tokio/libp2p in mycelia-leaf"
