#!/usr/bin/env bash
# check-cold-adapter-isolation: scans the four runtime-core boundary
# crates (vb_core, vb_runtime, vb_storage, vb_ipc) recursively for
# ACTIVE HTTP / JSON / YAML / adapter-only dependencies and tokenized
# `use` / `extern crate` imports. Companion to
# check-workspace-assertions.rs (which only checks
# [dependencies]/[dev-dependencies] + generated/) and to
# xtask::dependency_boundary (which is a manifest-only proptest
# harness). This scanner adds:
#   - source-level detection of tokenized `use` / `extern crate`
#     imports in `crates/<boundary>/**/*.rs`, including tests/benches/
#     examples,
#   - manifest alias detection via `package = "..."` in boundary
#     crate `Cargo.toml` dependency tables,
#   - file:line diagnostics on stderr,
#   - per-line `# allow-cold-adapter: <reason>` / `// allow-cold-adapter:
#     <reason>` allowlist markers,
#   - an expanded forbidden-token set covering the full HTTP/JSON/YAML
#     surface: serde_json, saphyr, saphyr-parser, serde-saphyr, reqwest,
#     hyper, axum, ureq, attohttpc, isahc.
#
# Master quote (velvet-ballistics-MASTER.md:62): "HTTP and JSON are
# excluded from the v1 runtime core. Any future adapter must be a
# separate cold-path adapter crate and must not enter vb_core,
# vb_runtime, vb_storage, or vb_ipc."
#
# Usage:
#   bash scripts/check-cold-adapter-isolation.sh                # full boundary scan
#   bash scripts/check-cold-adapter-isolation.sh <path>...      # targeted file(s)
#
# Exit 0 if active == 0, exit 1 otherwise.
set -euo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/crates" ]]; then
  echo "InvalidInvocation: run from repository root" >&2
  exit 64
fi

mkdir -p target/gate-tools
rustc --edition=2024 scripts/check-cold-adapter-isolation.rs \
  -o target/gate-tools/check-cold-adapter-isolation

if [[ $# -gt 0 ]]; then
  target/gate-tools/check-cold-adapter-isolation "$@"
else
  target/gate-tools/check-cold-adapter-isolation
fi
