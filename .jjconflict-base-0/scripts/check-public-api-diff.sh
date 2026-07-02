#!/usr/bin/env bash
set -euo pipefail

packages=(
  vb_compile
  vb_core
  vb_ipc
  vb_runtime
  vb_storage
  vb_validate
  velvet-ballistics
)

for package in "${packages[@]}"; do
  rustup run nightly-2026-04-28 cargo public-api \
    -p "${package}" \
    diff origin/main..HEAD \
    --all-features \
    --deny removed \
    --deny changed
done
