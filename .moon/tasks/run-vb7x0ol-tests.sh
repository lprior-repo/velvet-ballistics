#!/usr/bin/env bash
set -euo pipefail

echo "=== Running vb_7x0ol policy dispatch tests ==="
cargo test --test vb_7x0ol_probe_registered_executor -- --show-output 2>&1 | tee .evidence/vb7x0ol-test-run.log
