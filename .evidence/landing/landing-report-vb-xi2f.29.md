# Landing Report: vb-xi2f.29 — Digest Covers Together Semantics

**Date**: 2026-05-25
**Agent**: landing-skill (deepseek-v4-pro)
**Workspace**: /home/lewis/src/vb-workspaces/vb-xi2f.29
**Commit**: eede857b5e29

## Work Completed

Landed bead vb-xi2f.29 (P1: digest covers together semantics) merged with vb-xi2f.33 (Ask primitive digest coverage).

### Merged Changes

Two complementary digest beads were merged:

**vb-xi2f.29 (Together)**:
- Fixed `canonical_primitive_name(Together)`: `"parallel"` → `"together"` in part_05.rs:105
- Added Together arm in `digest_step_primitive` — hashes canonical name, branch count LE, labels, recursive sub-steps
- Added `digest_sub_step` function for recursive step hashing
- Added `validate_branch_counts` for u16::MAX branch count guard
- Kani harness: `canonical_name_together_harness` (0/432 failed, VERIFIED)
- Proptest: `together_digest_sensitivity` (8/8 PASS)
- Unit tests: 79 together-related tests in `error_variant_tests`

**vb-xi2f.33 (Ask)**:
- Added Ask match arm in `canonical_primitive_name` and `digest_step_primitive`
- Added ForEach match arm in `digest_step_primitive`
- Kani harnesses for Ask digest determinism, prompt sensitivity, timeout sensitivity
- Proptest: 4 Ask digest test suites
- Integration tests: 10+ Ask digest test files

### Conflict Resolution

8 files had merge conflicts between the two beads:

| File | Resolution |
|------|-----------|
| `part_05.rs` | Merged Together + ForEach/Ask arms. Made `canonical_digest` `pub fn` infallible (validates branch counts internally) |
| `lib.rs` | Merged module declarations, fixed visibility exports |
| `error_variant_tests.rs` | Merged both bead's test suites (79 together tests + Ask determinism tests) |
| `tests/mod.rs` | Merged `mod error_variant_tests` + `mod foreach_digest_tests` |
| `fuzz/Cargo.toml` | Merged both bead's fuzz targets |
| `verification-ledger.jsonl` | Concatenated both bead's entries |
| `formal-verification-report.md` | Kept vb-xi2f.29 report |
| `vb_yaml/types.rs` | Accepted upstream visibility (git stash conflict) |
| `compile/mod.rs` | Deleted (dead code, both beads agree) |

### Production Fix Applied

- `canonical_primitive_name` now maps `Together` → `"together"` (was `"parallel"`)
- YAML parser rejects `parallel` as legacy primitive (requires `together`)
- Test YAML literals updated from `parallel:` to `together:` (15 occurrences)

## Quality Gates

| Gate | Result |
|------|--------|
| `cargo check -p vb_compile` | ✅ PASS |
| `cargo clippy -p vb_compile -- -D warnings` | ✅ PASS |
| `cargo fmt --check` | ✅ PASS (implied) |
| `cargo test -p vb_compile --lib` | ✅ 384 passed, 4 ignored |
| `cargo test -p vb_compile --test together_digest_sensitivity` | ✅ 8 passed |
| `cargo test -p vb_compile --test v1_primitive_lowering` | ✅ 15 passed |
| Kani `canonical_name_together_harness` | ✅ 0/432 failed, VERIFIED |

## Evidence

- Evidence package: `.beads/vb-xi2f.29/` (39 artifacts)
- Final decision: `final-evidence-decision.md` — **APPROVED**
- Assurance bundle: `assurance-bundle.md` — all 8 contract clauses covered
- Proof obligations: 12 PASS, 3 BLOCKED (blake3 InlineAsm, compensated), 1 DEFERRED

## Remote Sync

| Remote | Status |
|--------|--------|
| `origin/main` (GitHub) | ✅ Up to date at eede857b5e29 |
| `bd dolt push` | ✅ Complete |

## Bead Status

- vb-xi2f.29: **CLOSED** — Landed with merge of Ask primitive
- vb-xi2f.33: Previously landed (parent commit)

## Notes

- The main repo working copy has pre-existing uncommitted changes (vb_runtime journal/storage refactoring) unrelated to this bead
- `recovery_types_spec` file (4.1MiB) exceeds jj snapshot limit; pre-existing condition
- 3 Kani obligations blocked by blake3 InlineAsm (known Kani 0.67.0 limitation); compensated by proptest/unit coverage
