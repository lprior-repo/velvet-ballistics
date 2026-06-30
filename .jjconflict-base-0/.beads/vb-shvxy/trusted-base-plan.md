# Trusted Base Plan: vb-shvxy

## Overview

The trusted base consists of tooling, patterns, and infrastructure that proof obligations rely on without re-proving. Each entry identifies what is trusted, why, and what compensating evidence guards against trust misuse.

## Trusted Base Entries

### TB-001: Verus Registry-Driven Script Pattern
- **Source**: `scripts/verify-verus.sh` (lines 1-110)
- **What is trusted**: The pattern of registry-driven enumeration, per-target evidence capture, non-vacuous guard (zero targets → exit 1), and trust-boundary scanning.
- **Why trusted**: Verus infrastructure is already working and accepted as the canonical proof-script pattern in this repository.
- **Downstream use**: New lane scripts (Kani, Flux, proptest guard) should follow this template.
- **Compensating evidence**: Each new lane script is independently tested with real packages. The pattern is a design guide, not a pass-through.

### TB-002: Cargo Metadata Feature Resolution
- **Source**: `cargo metadata` output via `scripts/kani-list.sh` (line 31-44)
- **What is trusted**: `cargo metadata --no-deps --format-version 1` correctly reports declared features for each workspace package.
- **Why trusted**: Standard Cargo behavior. If it breaks, all crate builds break.
- **Downstream use**: Kani feature-gate validation (PO-003).
- **Compensating evidence**: Script validates JSON output with `python3 -m json.tool`. Undeclared features fail closed.

### TB-003: Xtask Loom Model Enumeration
- **Source**: `xtask/src/loom.rs` `LOOM_MODELS` const array (lines 17-26)
- **What is trusted**: The const array correctly enumerates known loom models. The `find_model` function resolves model names to file paths.
- **Why trusted**: Single source of truth for loom model inventory; already used in xtask loom command.
- **Downstream use**: Loom model listing (PO-011) and model execution (PO-010).
- **Compensating evidence**: Model execution independently verifies that listed models actually compile and run.

### TB-004: Prior vb-ttyc State 12 Blocker Evidence (Context Only)
- **Source**: `/home/lewis/isolated/velvet-ballistics-main-review/vb-ttyc/.beads/vb-ttyc/evidence/state12-attempt7/`
- **What is trusted**: The prior failure logs are accepted as accurate documentation of what was broken. They are NOT reused as fresh pass evidence.
- **Why trusted**: Blockers are self-evident (missing scripts, compilation failures, zero-test vacuity).
- **Downstream use**: Informs blocker classification and obligation design. PO-012 references these as negative examples.
- **Compensating evidence**: Fresh execution evidence is required for all new obligations. Contract clause C-011 explicitly prohibits reclassification of prior capped evidence as fresh pass evidence.

### TB-005: Moon CI Fuzz-Smoke Target Configuration
- **Source**: `.moon/tasks/all.yml` lines 452-470
- **What is trusted**: The moon fuzz-smoke task's use of `--target x86_64-unknown-linux-gnu` is the correct sanitizer-compatible configuration.
- **Why trusted**: Already committed and working in moon CI; prior verification-ledger lines 108-110 confirm GNU target passes where musl fails.
- **Downstream use**: Fuzz build obligations (PO-009) reference this as the canonical target triple.
- **Compensating evidence**: Direct `cargo fuzz build --target x86_64-unknown-linux-gnu` tested independently.

## Trusted Assumptions (No Entry Required, Noted for Review)

1. **Bash 5.x available**: All scripts use `#!/usr/bin/env bash` with `set -euo pipefail`. Assumed standard in CI and development environments.
2. **Python 3.x available**: `scripts/kani-list.sh` and `scripts/verify-verus.sh` use Python for JSON/metadata parsing. Assumed available.
3. **Cargo nightly-2026-04-28**: Repository governance pins this nightly. All cargo commands are assumed to run against this toolchain.
4. **Network access for crate downloads**: Cargo builds may need network. Not in scope for offline proof; assumed available.

## Trusted Boundaries NOT Crossed

- **Tool version output is NEVER trusted as behavior proof**: Version checks are `SetupHealth` only.
- **Wrapper scripts are initially untrusted**: Each wrapper is independently verified by tooling obligations before its output is classified as evidence.
- **Ambient workspace configuration**: `.cargo/config.toml` does not set `build.target`; obligations requiring a specific target triple must specify it explicitly to avoid ambient drift.
