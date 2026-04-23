#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR/rust/hybrid-lab"

if [[ "${USE_WGPU:-0}" == "1" ]]; then
  cargo run --release --features wgpu-backend -- --depth 5 --branching 3 --hands 220 --actions 4 --iters 300 --align 64
else
  cargo run --release -- --depth 5 --branching 3 --hands 220 --actions 4 --iters 300 --align 64
fi
