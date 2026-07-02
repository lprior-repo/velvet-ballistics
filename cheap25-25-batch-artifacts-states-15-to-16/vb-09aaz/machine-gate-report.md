# Machine Gate Report — vb-09aaz

> Bead-level summary of machine-executed gates. Sourced from `.beads/vb-09aaz/evidence/*.log` and the formal-verification-report.md gate tables.

- bead_id: `vb-09aaz`
- state: 12 (formal-verification) — synthesized for state-14 evidence-packaging gate consumption
- workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-09aaz`

## Cargo Gates (User-Narrowed Scope)

| Gate | Command | Exit | Evidence |
|------|---------|------|----------|
| Rust unit tests — `batch_index_key` | `cargo test -p vb_storage --lib batch_index_key` | 0 | `state12-batch_index_key.log` — 2 passed, 1529 filtered |
| Rust unit tests — `t_append_event` | `cargo test -p vb_storage --lib t_append_event` | 0 | `state12-t_append_event.log` — 10 passed, 1521 filtered |
| Rust unit tests — `batch` (full) | `cargo test -p vb_storage --lib batch` | 0 | `state12-batch.log` — 195 passed, 1336 filtered |
| Cargo build | `cargo build -p vb_storage` | 0 | 4 crates compiled, 4.67s |

## Verus Gates

| Gate | Command | Exit | Evidence |
|------|---------|------|----------|
| Verus spec PS-008 (WEAK_EXTERN) | `verus --crate-type=lib verification/verus/vb-vzcuf-PS-008.rs` | 0 | `state12-verus-PS-008.log` — 19 verified, 0 errors |
| Verus spec PS-009 (WEAK_EXTERN) | `verus --crate-type=lib verification/verus/vb-vzcuf-PS-009.rs` | 0 | `state12-verus-PS-009.log` — 22 verified, 0 errors |
| Verus production-binding gate (GOD RULE 2) | `bash scripts/check-verus-production-binding.sh` | 0 | `state12-check-verus-production-binding.log` — 0 VACUUM, 71 WEAK_EXTERN |
| Production-inner drift gate | `bash scripts/check-production-inner-drift.sh` | 1 (FAIL_GLOBAL: 12 unrelated findings, zero in vb-09aaz blast radius) | `state12-production-inner-drift.log` |
| Verus global registry | `bash scripts/verify-verus.sh` | 1 (FAIL_GLOBAL: pre-existing Verus toolchain internal error on `recovery_verification.rs`) | `state12-verify-verus.log` |

## Lint and Format Gates (Prior State Evidence)

| Gate | Command | Result | Evidence |
|------|---------|--------|----------|
| Clippy | `cargo clippy -p vb_storage` | No issues found | `.beads/vb-09aaz/evidence/vb_storage-clippy.txt` |
| Format | `cargo fmt --check -p vb_storage` | exit=0 | `.beads/vb-09aaz/evidence/vb_storage-fmt.txt` |
| Build (full) | `cargo build` (workspace) | 0 crates compiled | `.beads/vb-09aaz/evidence/vb_storage-check.txt` |
| Build (workspace) | `cargo build` (workspace) | 0 crates compiled | `.beads/vb-09aaz/evidence/workspace-check.txt` |

## Full Test Suite (Prior State Evidence)

| Gate | Command | Result | Evidence |
|------|---------|--------|----------|
| All vb_storage tests | `cargo test -p vb_storage` | 1672 passed (17 suites, 10.50s) | `.beads/vb-09aaz/evidence/vb_storage-full-tests.txt` |

## Status

`STATUS: PASS` — all user-narrowed cargo gates and the two Verus spec gate files pass. The two FAIL_GLOBAL classifications (drift gate, verify-verus.sh) are pre-existing workspace-wide failures unrelated to vb-09aaz's call-graph blast radius and do not block bead closure per the formal-verifier skill rule "Existing unrelated global failures: classify honestly; do not turn them into proof success".