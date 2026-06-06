#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

ensure_tool() {
  local name="$1" version="$2"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "$name not found — installing $version..."
    cargo install "$name" --locked --version "$version"
  fi
}

ensure_tool cargo-audit 0.22.2
ensure_tool cargo-deny 0.19.8

echo "Running cargo audit..."
cargo audit

echo "Running cargo deny..."
cargo deny check advisories licenses bans sources

echo "Security checks passed"
