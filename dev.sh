#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
echo "Starting CodeForge..."
cargo run --manifest-path Cargo.toml -p codeforge-app "$@"
