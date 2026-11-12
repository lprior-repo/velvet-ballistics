# Formal Verification Report — vb-xi2f.31 Repeat Digest

- **bead_id**: vb-xi2f.31
- **phase**: p12-formal-verifier
- **verifier_agent**: formal-verifier (deepseek-v4-pro)
- **date**: 2026-05-25
- **pipeline_state**: State 12 (formal verification execution)
- **workdir**: `/home/lewis/src/vb-workspaces/vb-xi2f.31`

## Gate Status

| Gate | Status | Evidence |
|---|---|---|
| Proof-plan review | APPROVED | proof-plan-review.md |
| Proof-to-rust bridge review | APPROVED (RETRY v2) | proof-to-rust-review.md |
| Contract registration | VERIFIED | 5 clauses in contracts/proof_obligations.yaml |
| Invariant registration | VERIFIED | 2 invariants in contracts/invariants.yaml |
| Source artifacts exist | VERIFIED | All 12 RRO source_refs point to reachable files |
| Test artifacts exist | VERIFIED | All test files exist and compile |
| Harness artifacts exist | VERIFIED | kani_digest_repeat.rs with 5 harness functions |
| Trusted base dispositions | ACCEPTED | 8 TBL entries, none pending |
| Holzman PASS | ✅ | 320 tests, 0 failures, cargo clippy clean |

## Obligation Execution Results

### PO-001: Kani — max_attempts consumed (RRO-VB-XI2F31-001)
- **Command**: `cargo kani --package vb_compile --harness kani_repeat_max_attempts_consumed`
- **Raw evidence**: Harness **compiles** (codegen passes). Verification **fails** at `blake3::Hasher::new()` → `__cpuid_count` InlineAsm (Kani 0.67 limitation).
- **Result**: **WAIVED** (FW-VB-XI2F31-001)
- **Compensating**: 13 unit tests, 10 integration tests, 3 proptests, 320 crate tests — all PASS.

### PO-002: Kani — body consumed (RRO-VB-XI2F31-002)
- **Command**: `cargo kani --package vb_compile --harness kani_repeat_body_consumed`
- **Raw evidence**: Same blake3 InlineAsm blocker. Harness compiles cleanly.
- **Result**: **WAIVED** (FW-VB-XI2F31-001)
- **Compensating**: Same as PO-001. Body differentiation verified by PO-009 (unit) and PO-006 (proptest).

### PO-003: Kani — different params → different digest (RRO-VB-XI2F31-003)
- **Command**: `cargo kani --package vb_compile --harness kani_repeat_different_params_different_digest`
- **Raw evidence**: Same blake3 InlineAsm blocker. Harness compiles cleanly.
- **Result**: **WAIVED** (FW-VB-XI2F31-001)
- **Compensating**: PO-008 (unit, max_attempts), PO-009 (unit, body), PO-006 (proptest, randomized), PO-011/PO-012 (integration, end-to-end).

### PO-004: Kani — both impls equivalent (RRO-VB-XI2F31-004)
- **Command**: `cargo kani --package vb_compile --harness kani_repeat_both_impls_equivalent`
- **Raw evidence**: Harness compiles but `compile/mod.rs` is unreachable dead code (not in module tree). Kani cannot import `compile/mod.rs::digest_step_primitive`.
- **Result**: **WAIVED** (FW-VB-XI2F31-002)
- **Compensating**: Cross-path unit test (`test_repeat_same_config_same_digest_cross_path`) and integration test (`test_repeat_digest_cross_path_equivalent`) both PASS.

### PO-005: Kani — Set/Finish preserved (RRO-VB-XI2F31-005)
- **Command**: `cargo kani --package vb_compile --harness kani_finish_set_digest_unchanged`
- **Raw evidence**: Same blake3 InlineAsm blocker. Harness compiles cleanly.
- **Result**: **WAIVED** (FW-VB-XI2F31-001)
- **Compensating**: PO-010 (unit idempotency), PO-007 (proptest idempotency), full crate 320 tests.

### PO-006: Proptest — different params → different digest (RRO-VB-XI2F31-006)
- **Command**: `cargo test --package vb_compile --test v1_primitive_lowering proptest_repeat_different_params_different_digest proptest_repeat_different_body_different_digest -- --nocapture`
- **Raw evidence**: `test result: ok. 2 passed; 0 failed; 15 filtered out; finished in 0.02s`
- **Result**: ✅ **PASS**

### PO-007: Proptest — idempotency preserved (RRO-VB-XI2F31-007)
- **Command**: `cargo test --package vb_compile --test v1_primitive_lowering proptest_equal_primitive_sources_compile_to_equal_digest_and_ir -- --nocapture`
- **Raw evidence**: `test result: ok. 1 passed; 0 failed; 16 filtered out; finished in 0.02s`
- **Result**: ✅ **PASS**

### PO-008: Unit test — max_attempts changes digest (RRO-VB-XI2F31-008)
- **Command**: `cargo test --package vb_compile --test digest_repeat_unit test_repeat_max_attempts_changes_digest test_repeat_max_attempts_changes_digest_compile_source -- --nocapture`
- **Raw evidence**: `test result: ok. 7 passed; 0 failed; 6 filtered out; finished in 0.00s` (full suite run)
- **Result**: ✅ **PASS**

### PO-009: Unit test — body changes digest (RRO-VB-XI2F31-009)
- **Command**: Covered by same test suite as PO-008 (`test_repeat_body_changes_digest` and variants)
- **Raw evidence**: All 13 unit tests pass in `digest_repeat_unit` suite.
- **Result**: ✅ **PASS**

### PO-010: Unit test — same config → same digest (RRO-VB-XI2F31-010)
- **Command**: Covered by same test suite as PO-008 (`test_repeat_same_config_same_digest` and variants)
- **Raw evidence**: All 13 unit tests pass. Idempotency tests pass.
- **Result**: ✅ **PASS**

### PO-011: Integration test — compile_workflow path (RRO-VB-XI2F31-011)
- **Command**: `cargo test --package vb_compile --test repeat_digest_integration test_compile_workflow_repeat -- --nocapture`
- **Raw evidence**: `test result: ok. 4 passed; 6 filtered out; finished in 0.00s` (compile_workflow subset)
- **Result**: ✅ **PASS**

### PO-012: Integration test — compile_source path (RRO-VB-XI2F31-012)
- **Command**: `cargo test --package vb_compile --test repeat_digest_integration test_compile_source_repeat -- --nocapture`
- **Raw evidence**: `test result: ok. 3 passed; 7 filtered out; finished in 0.00s` (compile_source subset)
- **Result**: ✅ **PASS**

## Full Crate Test Suite

```
Command: cargo test --package vb_compile
Result: 320 passed, 0 failed, 9 suites, 2.44s
```

## Summary

| Classification | Count | Obligations |
|---|---|---|
| ✅ PASS | 7 | PO-006, PO-007, PO-008, PO-009, PO-010, PO-011, PO-012 |
| 🟡 WAIVED | 5 | PO-001, PO-002, PO-003, PO-004, PO-005 |
| ❌ FAIL_LOCAL | 0 | — |
| ❌ FAIL_REGRESSION | 0 | — |
| ❌ FAIL_GLOBAL | 0 | — |
| **TOTAL** | **12** | All obligations closed |

## Waiver Summary

| Waiver | Obligations | Blocker | Compensating Evidence |
|---|---|---|---|
| FW-VB-XI2F31-001 | PO-001, PO-002, PO-003, PO-005 | BLOCKER-BLAKE3-INLINEASM (Kani 0.67 cannot model blake3 `__cpuid_count`) | 13 unit, 10 integration, 3 proptest (all PASS), 320 crate tests |
| FW-VB-XI2F31-002 | PO-004 | BLOCKER-COMPILE-MOD-UNREACHABLE (dead code) | 2 cross-path equivalence tests PASS |

## Implementation Verification

The Repeat fix is confirmed in reachable source:
- `crates/vb_compile/src/mod_compile_lowering/part_05.rs:158-165`: `Repeat { max_attempts, body }` match arm hashes both `b"repeat"` + `max_attempts.to_le_bytes()` + recursive body step hashing via `digest_step_primitive`.
- Each Kani harness has corresponding `#[kani::proof]` function in `kani_digest_repeat.rs`.
- GOD RULE 1 (no hardcoded shapes): verified — harnesses use `kani::any()` for symbolic inputs.
- GOD RULE 4 (fix implementation, not harness): verified — the Repeat arm was added to production code, not worked around.

## Artifacts Written

- `reports/formal-verification-report.md` (this file)
- `.beads/vb-xi2f.31/formal-waivers.jsonl` (2 formal waivers)
- `verification-ledger.jsonl` (12 new vb-xi2f.31 entries appended)
- `.beads/vb-xi2f.31/proof-obligations.planned.jsonl` (statuses updated)
