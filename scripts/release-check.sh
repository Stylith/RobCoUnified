#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

echo "[release-check] cargo fmt --check"
cargo fmt --check

echo "[release-check] cargo check"
cargo check

echo "[release-check] cargo test"
cargo test
