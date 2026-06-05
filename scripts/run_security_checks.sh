#!/usr/bin/env bash
set -euo pipefail

echo "Running cargo-audit..."
if ! command -v cargo-audit >/dev/null 2>&1; then
  echo "cargo-audit not found, installing..."
  cargo install cargo-audit --locked
fi
cargo audit --json > audit.json || true
cat audit.json
if command -v jq >/dev/null 2>&1; then
  if ! jq -e '.vulnerabilities.list | length == 0' audit.json >/dev/null; then
    echo "Vulnerabilities found (see audit.json)"
    exit 1
  fi
else
  echo "jq not found; please inspect audit.json manually"
fi

echo "Running cargo-deny..."
if ! command -v cargo-deny >/dev/null 2>&1; then
  echo "cargo-deny not found, installing..."
  cargo install cargo-deny --locked
fi
cargo deny check -v --json > deny.json || true
cat deny.json
if command -v jq >/dev/null 2>&1; then
  if ! jq -e '.diagnostics | length == 0' deny.json >/dev/null; then
    echo "cargo-deny reported issues (see deny.json)"
    exit 1
  fi
else
  echo "jq not found; please inspect deny.json manually"
fi

echo "Security checks passed"
