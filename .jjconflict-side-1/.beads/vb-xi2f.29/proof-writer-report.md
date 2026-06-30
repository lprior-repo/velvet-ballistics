# Proof Writer Report: vb-xi2f.29 — Digest Covers Together Semantics (REPAIR-2)

**bead_id**: vb-xi2f.29
**bead_title**: P1: digest covers together semantics
**role**: proof-writer (State 5) REPAIR ATTEMPT 2
**invocation**: pw-repair-vb-xi2f29-2026-05-25-002
**date**: 2026-05-25
**workspace**: /home/lewis/src/vb-workspaces/vb-xi2f.29
**prior_invocation**: pw-vb-xi2f29-2026-05-24-001 (REJECTED by proof-reviewer)
**prior_review**: proof-review.md: STATUS REJECTED (4 lethal findings)

## Summary of Repairs

This is REPAIR ATTEMPT 2 addressing all 4 lethal findings from the proof-reviewer rejection.

### LF-001 (CRITICAL): Kani Harness Compilation Errors — FIXED

**Root cause**: Orphaned `///` doc comments at end of `kani_canonical_name.rs` and `together_digest_kani.rs`.

**Fix**: Converted `///` doc comments to `//` line comments in both files. Additionally removed `#[kani::no_unwinding_checks]` attributes (not supported in Kani 0.67.0). Fixed `TogetherBranch`` constructor to remove non-existent `condition` field.

**Evidence**: `cargo kani --only-codegen` exits 0 for all harnesses.

### LF-002 (HIGH): GOD RULE 1 Violation — Hardcoded Branch Count — FIXED

**Root cause**: `together_branch_count_produces_different_digest_kani` used hardcoded 2-branch and 3-branch structures.

**Fix**: Rewritten to use `kani::any()` for symbolic enumeration:
- `count_a: u8 = kani::any()` with `kani::assume(count_a >= 1 && count_a <= 4)`
- `count_b: u8 = kani::any()` with `kani::assume(count_b >= 1 && count_b <= 4 && count_b != count_a)`
- Branch labels: `kani::any()` for single-char alphanumeric labels
- Branch count symbolic space: 4×3 = 12 distinct combinations

### LF-003 (HIGH): Proof-to-Implementation Bridge False Claim — FIXED

**Root cause**: `proof-to-implementation-input.md:39` claimed `canonical_primitive_name(Together)` already returned `"together"` when it actually returned `"parallel"`.

**Fix**: Updated the note to state the actual bug and the required fix. Also **fixed the production code**: `part_05.rs:105` changed from `"parallel"` to `"together"`.

### LF-004 (HIGH): Zero Kani Evidence — FIXED

**Root cause**: No Kani compilation or execution logs existed in proof-evidence.md.

**Fix**: Captured raw compilation and partial execution logs. Documented `BLOCKED_TOOLING` for digest harnesses (blake3 inline assembly unsupported by Kani).

## Obligations Status (After Repair-2)

| Obligation | Verifier | Artifact | Status | Details |
|---|---|---|---|---|
| PO-xi2f29-001 | kani | `kani_canonical_name.rs` | VERIFIED | canonical_name_together_harness: VERIFICATION:- SUCCESSFUL |
| PO-xi2f29-002 | proptest | `together_digest_sensitivity.rs` | PASS (POST-FIX) | 6/6 PASSED incl. sensitivity (was 5 FAIL before fix) |
| PO-xi2f29-003 | proptest | `together_digest_sensitivity.rs` | PASS (POST-FIX) | Branch label sensitivity passes after fix |
| PO-xi2f29-004 | proptest | `together_digest_sensitivity.rs` | PASS (POST-FIX) | Sub-step content sensitivity passes after fix |
| PO-xi2f29-005 | proptest | `together_digest_sensitivity.rs` | PASS (POST-FIX) | Branch ordering sensitivity passes after fix |
| PO-xi2f29-006 | proptest | `together_digest_sensitivity.rs` | PASS | Determinism passes (always did) |
| PO-xi2f29-007 | proptest | `v1_primitive_lowering.rs` | UNVERIFIED | Regression gate not yet executed |
| PO-xi2f29-008 | kani | `kani_canonical_name.rs` | TIMED_OUT | canonical_name_all_harness: state space explosion (12 variants × symbolic data) |
| PO-xi2f29-009 | kani | `together_digest_kani.rs` | BLOCKED_TOOLING | Kani cannot verify blake3 (InlineAsm) |
| PO-xi2f29-010 | kani | `together_digest_kani.rs` | BLOCKED_TOOLING | Kani cannot verify blake3 (InlineAsm) |
| PO-xi2f29-010b | kani | `together_digest_kani.rs` | BLOCKED_TOOLING | together_branch_count Kani harness blocked by blake3 InlineAsm |
| PO-xi2f29-011 | unit | `error_variant_tests.rs` | PASS | test_empty_branch_steps_produces_deterministic_digest PASS |
| PO-xi2f29-012 | unit | `error_variant_tests.rs` | PASS | test_nested_together_produces_distinct_recursive_digest PASS |
| PO-xi2f29-013 | unit | `error_variant_tests.rs` | PASS | test_canonical_digest_is_idempotent_with_together PASS |
| PO-xi2f29-014 | unit | `error_variant_tests.rs` | PASS | test_different_together_configurations_produce_different_digests PASS |
| PO-xi2f29-015 | unit | `error_variant_tests.rs` | PASS | test_canonical_primitive_name_together_returns_together PASS |

## Production Code Changes

All required production code changes from the proof-plan were applied to `crates/vb_compile/src/mod_compile_lowering/part_05.rs`:

1. **Line 98**: Changed `pub(super)` to `pub(crate)` for `canonical_primitive_name` (test/Kani access)
2. **Line 105**: Changed `Together { .. } => "parallel"` to `Together { .. } => "together"` (CANONICAL_NAME_BUG fix)
3. **Line 116**: Changed `pub(super)` to `pub(crate)` for `canonical_digest`
4. **Line 140**: Changed `pub(super)` to `pub(crate)` for `digest_step_primitive`
5. **Lines 158-167**: Added explicit `Together` arm in `digest_step_primitive` that hashes:
   - Canonical name "together"
   - `branches.len() as u16` as little-endian bytes
   - Each branch's `label` as UTF-8 bytes
   - Recursively calls `digest_sub_step` for each sub-step
6. **Lines 174-177**: Added `digest_sub_step` function that recursively hashes `step.id` and `step.primitive`

## Kani Harness Changes

### kani_canonical_name.rs
- Removed `#[kani::no_unwinding_checks]` attributes (not in Kani 0.67)
- Fixed `TogetherBranch` constructor (removed non-existent `condition` field)
- Changed visibility paths from `crate::mod_compile_lowering::part_05::` to `crate::mod_compile_lowering::` (part_05 is private module)
- Added symbolic `kani::any()` field data to `canonical_name_together_harness` for GOD RULE 1
- Rewrote `canonical_name_all_harness` to use discriminant-based enumeration (StepPrimitive does not implement kani::Arbitrary in Kani 0.67)
- Converted orphaned `///` doc comments to `//` line comments

### together_digest_kani.rs
- Removed `#[kani::no_unwinding_checks]` attributes
- Changed visibility paths from `crate::mod_compile_lowering::part_05::` to `crate::mod_compile_lowering::`
- Rewrote `together_branch_count_produces_different_digest_kani`: GOD RULE 1 compliant with `kani::any()` for symbolic branch counts and labels
- Rewrote `together_digest_step_deterministic_kani`: Construct symbolic Together values with `kani::any()` for branch count, labels, and sub-step presence
- Converted orphaned `///` doc comments to `//` line comments

## BLOCKED_TOOLING: blake3 Inline Assembly

**Severity**: HIGH
**Affected**: PO-xi2f29-009, PO-xi2f29-010, PO-xi2f29-010b
**Description**: Kani 0.67.0 cannot verify code paths that reachable `blake3::Hasher` usage because blake3 uses x86 `InlineAsm` (specifically `std::arch::x86_64::__cpuid_count` for CPU feature detection) which is not supported by Kani's symbolic execution engine.

```
Failed Checks: TerminatorKind::InlineAsm is not currently supported by Kani.
 File: ".../stdarch/crates/core_arch/src/x86/cpuid.rs", line 75, in std::arch::x86_64::__cpuid_count
```

**Mitigation**: Proptest and unit tests cover digest correctness (PO-xi2f29-002 through PO-xi2f29-006, PO-xi2f29-011 through PO-xi2f29-014). The Together fix is independently verified by:
1. `canonical_name_together_harness`: VERIFIED (canonical name fix)
2. Proptest: 6/6 PASSED (structural sensitivity + determinism)
3. Unit: 5/5 PASSED (edge cases + idempotency)

## Non-Vacuity Assertion

All proptest sensitivity tests (`assert_ne!`) now **PASS** because the production code fix makes structurally different Together workflows produce genuinely different digests. Before the fix, all 5 sensitivity tests FAILED with `assert_ne!` violations (identical digests). After the fix, they all PASS, confirming the tests are non-vacuous and the fix is correct.

## Files Changed (This Repair)

| File | Change |
|---|---|
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | Fixed canonical_primitive_name Together→"together", added Together arm in digest_step_primitive, added digest_sub_step, changed visibility to pub(crate) |
| `crates/vb_compile/src/kani_canonical_name.rs` | Fixed orphaned doc comments, removed no_unwinding_checks, fixed TogetherBranch constructor, rewrote all_harness for discriminant enumeration, fixed visibility paths |
| `crates/vb_compile/src/together_digest_kani.rs` | Fixed orphaned doc comments, removed no_unwinding_checks, rewrote branch_count harness with kani::any() (GOD RULE 1), rewrote determinism harness, fixed visibility paths |
| `.beads/vb-xi2f.29/proof-to-implementation-input.md` | Fixed false claim about canonical_primitive_name already being correct |
| `.beads/vb-xi2f.29/proof-writer-report.md` | This file (rewritten for REPAIR-2) |
| `.beads/vb-xi2f.29/proof-evidence.md` | Updated with raw Kani compilation and execution evidence |

## Pending Deep Execution

The following cannot be verified by Kani due to blake3 inline assembly:

```bash
# Digest harnesses — BLOCKED by blake3 InlineAsm (see BLOCKED_TOOLING above)
TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile --harness together_digest_sub_step_recursion_bounded_kani --no-unwinding-checks --default-unwind 10
TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile --harness together_digest_step_deterministic_kani --no-unwinding-checks --default-unwind 8
TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile --harness together_branch_count_produces_different_digest_kani --no-unwinding-checks --default-unwind 8
```

The `canonical_name_all_harness` requires further optimization to avoid state space explosion:

```bash
# All variants harness — TIMED_OUT (needs optimization)
TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile --harness canonical_name_all_harness --no-unwinding-checks --default-unwind 4
```

## Blockers Summary

| Blocker | Severity | Status | Description |
|---|---|---|---|
| BLK-xi2f29-KANI-BLAKE3 | HIGH | BLOCKED_TOOLING | Kani cannot verify blake3::Hasher (InlineAsm). Digest harnesses blocked. |
| BLK-xi2f29-ALLHARNESS | MEDIUM | TIMED_OUT | canonical_name_all_harness state space explosion. Needs further reduce. |
| BLK-xi2f29-REGRESSION | LOW | UNVERIFIED | PO-xi2f29-007 regression gate not yet executed |
