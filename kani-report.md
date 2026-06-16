# Kani Verification Report — 2026-06-15

## Executive Summary

**21 Kani obligations assessed. 0 executed successfully. 21 FAIL_BLOCKED.**

All 21 planned Kani obligations are blocked by a systemic Kani API compatibility issue in `vb_core`. The harnesses were written for an older Kani version that supported `kani::assert_eq!` and variadic `kani::assert()` formatting. Kani 0.67.0 requires `kani::assert(condition, msg)` (exactly 2 arguments, no formatting) and has no `kani::assert_eq!` macro.

## Blocker: Kani API Mismatch

**Severity:** C1 — Blocks ALL Kani verification across entire workspace

**Root cause:** 364 occurrences of `kani::assert_eq!` in vb_core source files + variadic `kani::assert()` calls in `crates/vb_core/src/verification/kani/kani_parallel_in_flight.rs`.

**Kani 0.67.0 API (current):**
- `kani::assert(condition: bool, msg: &str)` — exactly 2 arguments
- No `kani::assert_eq!` macro
- No `kani::any_vec` function

**Harness API (expected by proofs):**
- `kani::assert_eq!(a, b, msg)` — 3-argument macro
- `kani::assert(cond, "format {}", arg)` — variadic formatting

**Error types observed:**
- `E0433`: `failed to resolve: could not find 'assert_eq' in 'kani'` (39 occurrences)
- `E0061`: `this function takes 2 arguments but 1 argument was supplied` (27 occurrences)
- `E0061`: `this function takes 2 arguments but 4 arguments were supplied` (4 occurrences)

**Impact:** Every `cargo kani` compilation across ALL packages fails with 70 errors from vb_core alone, because every package depends on vb_core transitively.

## Per-Obligation Results

### vb-fzgdn (10 obligations — timer wheel/shard lifecycle)

| ID | Proof ID | Evidence Command | Status | Blocker |
|----|----------|-----------------|--------|---------|
| RRO-vb-fzgdn-002 | POB-vb-fzgdn-002 | `cargo kani -p vb_runtime --harness ps_001_check` | FAIL_BLOCKED | Harness name `ps_001_check` does not exist. Actual: `ps_001_generation_starts_at_one` |
| RRO-vb-fzgdn-007 | POB-vb-fzgdn-007 | `cargo kani -p vb_runtime --harness ps_002_check` | FAIL_BLOCKED | Harness name `ps_002_check` does not exist. Actual: `ps_002_pending_timer_matches_exact_authority` |
| RRO-vb-fzgdn-012 | POB-vb-fzgdn-012 | `cargo kani -p vb_runtime --harness ps_003_check` | FAIL_BLOCKED | Harness name `ps_003_check` does not exist |
| RRO-vb-fzgdn-016 | POB-vb-fzgdn-016 | `cargo kani -p vb_runtime --harness ps_004_check` | FAIL_BLOCKED | Harness name `ps_004_check` does not exist |
| RRO-vb-fzgdn-020 | POB-vb-fzgdn-020 | `cargo kani -p vb_runtime --harness ps_005_check` | FAIL_BLOCKED | Harness name `ps_005_check` does not exist |
| RRO-vb-fzgdn-024 | POB-vb-fzgdn-024 | `cargo kani -p vb_runtime --harness ps_006_check` | FAIL_BLOCKED | Harness name `ps_006_check` does not exist |
| RRO-vb-fzgdn-029 | POB-vb-fzgdn-029 | `cargo kani -p vb_runtime --harness ps_007_check` | FAIL_BLOCKED | Harness name `ps_007_check` does not exist |
| RRO-vb-fzgdn-034 | POB-vb-fzgdn-034 | `cargo kani -p vb_runtime --harness ps_008_check` | FAIL_BLOCKED | Harness name `ps_008_check` does not exist |
| RRO-vb-fzgdn-038 | POB-vb-fzgdn-038 | `cargo kani -p vb_runtime --harness ps_009_check` | FAIL_BLOCKED | Harness name `ps_009_check` does not exist |
| RRO-vb-fzgdn-043 | POB-vb-fzgdn-043 | `cargo kani -p vb_runtime --harness ps_010_check` | FAIL_BLOCKED | Harness name `ps_010_check` does not exist |

**Additional issues for vb-fzgdn:**
- Evidence artifacts in `verification/kani/vb-fzgdn/PS-001-harness.rs` through `PS-010-harness.rs` exist (10 files)
- But `refinement_harness_refs` point to MISSING files in `crates/vb_runtime/tests/refinement/` (all 10 files missing)
- Actual harness functions exist in `crates/vb_runtime/src/verification/kani/vb_fzgdn_timer_harnesses.rs`

### vb-xi2f24 (11 obligations — compile reduce/IR proofs)

| ID | Proof ID | Evidence Command | Status | Blocker |
|----|----------|-----------------|--------|---------|
| RRO-vb-xi2f24-001 | PO-WIDTH-MATCH-KANI-001 | `cargo kani -p vb_compile --harness check_reduce_body_width_parity` | FAIL_BLOCKED | Compilation fails (vb_core API mismatch) |
| RRO-vb-xi2f24-004 | PO-TRYFROMPARTS-KANI-001 | `cargo kani -p vb_compile --harness check_reduce_body_width_parity` | FAIL_BLOCKED | Compilation fails (vb_core API mismatch) |
| RRO-vb-xi2f24-006 | PO-OFFSET-KANI-001 | `cargo kani -p vb_compile --harness check_reduce_body_offset_distinctness` | FAIL_BLOCKED | Harness name mismatch: file has `check_offset_distinctness` |
| RRO-vb-xi2f24-009 | PO-OVERFLOW-KANI-001 | `cargo kani -p vb_compile --harness check_reduce_body_width_overflow` | FAIL_BLOCKED | Compilation fails (vb_core API mismatch) |
| RRO-vb-xi2f24-012 | PO-NESTED-FOREACH-KANI-001 | `cargo kani -p vb_compile --harness check_reduce_foreach_width_advance` | FAIL_BLOCKED | Harness name mismatch: file has `check_foreach_width_advance` |
| RRO-vb-xi2f24-015 | PO-CHAIN-KANI-001 | `cargo kani -p vb_compile --harness check_reduce_body_chain_integrity` | FAIL_BLOCKED | Harness name mismatch: file has `check_chain_integrity` |
| RRO-vb-xi2f24-018 | PO-NESTED-NEXT-KANI-001 | `cargo kani -p vb_compile --harness check_reduce_nested_next_correctness` | FAIL_BLOCKED | Compilation fails (vb_core API mismatch) |
| RRO-vb-xi2f24-021 | PO-REGRESSION-KANI-001 | `cargo kani -p vb_compile --harness check_reduce_single_step_equivalence` | FAIL_BLOCKED | Harness name mismatch: file has `check_single_step_equivalence_contract` |
| RRO-vb-xi2f24-023 | PO-EMPTY-KANI-001 | `cargo kani -p vb_compile --harness check_reduce_empty_body_rejection` | FAIL_BLOCKED | Compilation fails (vb_core API mismatch) |
| RRO-vb-xi2f24-025 | PO-NOPANIC-KANI-001 | `cargo kani -p vb_compile --harness check_reduce_lowering_no_panic` | FAIL_BLOCKED | Compilation fails (vb_core API mismatch) |
| RRO-vb-xi2f24-028 | PO-DIAGNOSTIC-KANI-001 | `cargo kani -p vb_compile --harness check_reduce_error_diagnostic_codes` | FAIL_BLOCKED | Compilation fails (vb_core API mismatch) |

**Additional issues for vb-xi2f24:**
- All 10 harness source files exist in `crates/vb_compile/src/mod_compile_lowering/kani_reduce_*.rs`
- Evidence artifacts in `verification/kani/vb-xi2f24/` DO NOT EXIST (directory missing)
- 5 of 11 obligations have harness name mismatches between evidence_command and actual function names

## Required Fixes (by proof-writer)

1. **C1 — Update all vb_core Kani harnesses to Kani 0.67.0 API:**
   - Replace `kani::assert_eq!(a, b, msg)` with `kani::assert(a == b, msg)`
   - Replace `kani::assert(cond, "fmt {}", arg)` with `kani::assert(cond, msg)` (remove formatting)
   - 364 occurrences across 51 files in vb_core, 70 errors in `kani_parallel_in_flight.rs` alone

2. **vb-fzgdn — Fix evidence_command harness names:**
   - Change `ps_001_check` → `ps_001_generation_starts_at_one` (or list all 3 functions)
   - Same pattern for ps_002 through ps_010

3. **vb-fzgdn — Fix refinement_harness_refs:**
   - Replace `crates/vb_runtime/tests/refinement/*.rs` with `crates/vb_runtime/src/verification/kani/vb_fzgdn_timer_harnesses.rs`

4. **vb-xi2f24 — Fix harness name mismatches:**
   - `check_reduce_body_offset_distinctness` → `check_offset_distinctness`
   - `check_reduce_foreach_width_advance` → `check_foreach_width_advance`
   - `check_reduce_body_chain_integrity` → `check_chain_integrity`
   - `check_reduce_single_step_equivalence` → `check_single_step_equivalence_contract`

5. **vb-xi2f24 — Generate evidence artifacts:**
   - Create `verification/kani/vb-xi2f24/` directory with Kani output reports

## Verified Obligations (from prior sessions)

| ID | Proof ID | Verifier | Status |
|----|----------|----------|--------|
| RRO-vb-e7tl-001 | VB-E7TL-001 | cargo-test+kani | verified |
| RRO-vb-e7tl-002 | VB-E7TL-002 | cargo-test | verified |
| RRO-vb-e7tl-003 | VB-E7TL-003 | cargo-test | verified |

3 obligations verified from prior vb-e7tl bead sessions.

## Kani Coverage Assessment (from KANI_GAP_ANALYSIS.md)

| Grade | Crates | Notes |
|-------|--------|-------|
| STRONG | vb_core, vb_runtime, vb_compile, vb_validate, vb_ipc, vb_benchmark, vb_verification | Has harnesses but ALL blocked by API mismatch |
| MIXED | vb_storage, vb_expr, vb_yaml, vb_proof_kernels, vb_cli | Mixed quality, API mismatch blocks execution |
| WEAK | vb_boundary_inventory | Minimal coverage |
| CRITICAL GAP | vb_queue_semantics | ZERO harnesses on pure queue state machine |
| LOW PRIORITY | vb_doc, vb_test_util | No harnesses needed |

KANI_GAP_ANALYSIS.md: `/home/lewis/src/velvet-ballistics/KANI_GAP_ANALYSIS.md`
