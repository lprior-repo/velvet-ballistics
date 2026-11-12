# Proof Review: Rounds 1-3 Fix Verification

**Reviewer:** proof-reviewer agent  
**Date:** 2026-05-23  
**Scope:** Cross-cutting verification of all fixes applied across Rounds 1-3 for velvet-ballistics  
**Status:** REJECTED

---

## Executive Summary

| Check | Status | Severity |
|-------|--------|----------|
| 1. Stale files deleted | **PASS** | — |
| 2. `pub mod kani` in vb_compile | **PASS** | — |
| 3. `forbid(unsafe_code)` in vb_proof_kernels | **PASS** | — |
| 4. budget_bounded.rs compiles with Verus | **PASS** | — |
| 5. Fuzz stubs call actual functions | **PASS** | — |
| 6. kani-list.json shows 170 harnesses | **PASS** | — |
| 7. GOD RULE 1 FIX headers in specified Kani files | **FAIL** | HIGH |
| 8. TypeOK in ALL TLA+ files | **FAIL** | CRITICAL |
| 9. Orphaned loom/proptest files deleted | **PASS** | — |
| 10. `forbid(unsafe_code)` in vb_boundary_inventory and vb_benchmark | **PASS** | — |

**Overall: 7 PASS, 2 FAIL, 1 PARTIAL. REJECTED due to critical TLA+ TypeOK gap and Kani file path mismatch.**

---

## Detailed Findings

### 1. Stale files deleted — PASS

**Evidence:**
```
$ find /home/lewis/src/velvet-ballistics -name "contracts_proptest.rs" -o -name "contracts_kani_harness.rs" -o -name "kani_taint.rs"
(no output)
```

All three stale files (`contracts_proptest.rs`, `contracts_kani_harness.rs`, `kani_taint.rs`) are confirmed absent from the repository.

---

### 2. vb_compile lib.rs has `pub mod kani;` — PASS

**Evidence:**
```
crates/vb_compile/src/lib.rs:48: pub mod kani;
```

Present at line 48, gated behind `#[cfg(kani)]`.

---

### 3. vb_proof_kernels has `#![forbid(unsafe_code)]` — PASS

**Evidence:**
```
crates/vb_proof_kernels/src/lib.rs:1: #![forbid(unsafe_code)]
```

Present at line 1.

---

### 4. budget_bounded.rs compiles with Verus — PASS

**Evidence:**
```
$ verus --crate-type lib verification/verus/budget_bounded.rs
verification results:: 15 verified, 0 errors
```

All 15 proof obligations verified with zero errors. The file correctly imports `vstd::prelude::*` and uses `verus!` macro.

---

### 5. Fuzz stubs call actual functions — PASS

**Evidence:**
```rust
// fuzz/fuzz_targets.rs:4-6
pub fn yaml_events(data: &[u8]) {
    fuzz_lib::fuzz_yaml_events(data);
}

// fuzz/fuzz_targets.rs:30-33
pub extern "C" fn LLVMFuzzerTestOneInputYamlEvents(data: *const u8, len: usize) -> i32 {
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    yaml_events(slice);
    0
}
```

All 14 fuzz target stubs delegate to actual `fuzz_lib::fuzz_*` implementations. The C ABI entrypoints call the safe Rust wrappers.

---

### 6. kani-list.json shows 170 harnesses — PASS

**Evidence:**
```json
{
  "kani-version": "0.67.0",
  "file-version": "0.2",
  "standard-harnesses": {
    "total-discovered": 170,
    ...
  },
  "totals": {
    "standard-harnesses": 170,
    "contract-harnesses": 0,
    "functions-under-contract": 0
  }
}
```

`total-discovered: 170` is present and consistent across both `standard-harnesses.total-discovered` and `totals.standard-harnesses`.

---

### 7. Kani files have GOD RULE 1 FIX header — FAIL

**Severity:** HIGH

**Expected paths (as specified in review request):**
- `crates/vb_compile/src/kani_expr_bound.rs` — **DOES NOT EXIST**
- `crates/vb_compile/src/kani_capability_harnesses.rs` — **DOES NOT EXIST**

**Actual paths where files DO exist:**
- `crates/vb_core/src/kani_expr_bound.rs` — EXISTS, has GOD RULE 1 FIX header
- `crates/vb_core/src/kani_capability_harnesses.rs` — EXISTS, has GOD RULE 1 FIX header

**Evidence:**
```
crates/vb_core/src/kani_expr_bound.rs:3: //! GOD RULE 1 FIX: Replaced all hardcoded literal expression arrays with
crates/vb_core/src/kani_capability_harnesses.rs:4: // GOD RULE 1 FIX: Replaced all hardcoded capability names and action IDs
```

**Finding:** The GOD RULE 1 FIX headers are present and correct, but the files reside in `crates/vb_core/src/` instead of the expected `crates/vb_compile/src/`. The `vb_compile` crate has a `kani/` subdirectory (`vb_compile_accessor.rs`, `vb_compile_bytecode.rs`, etc.) but lacks the two files named in the review request.

**Required fix:** Either:
- (a) Move/copy `kani_expr_bound.rs` and `kani_capability_harnesses.rs` into `crates/vb_compile/src/` and update module declarations, OR
- (b) Update the review request to reference the correct paths in `crates/vb_core/src/`

---

### 8. TLA+ files now have TypeOK — CRITICAL FAIL

**Severity:** CRITICAL

**Evidence:**
- Total `.tla` files in repository: **56**
- Total non-bead `.tla` files (excluding `.beads/`): **48**
- Non-bead `.tla` files **with** TypeOK: **13** (27%)
- Non-bead `.tla` files **without** TypeOK: **35** (73%)

**Files in `verification/tla/` specifically:**
- Total: 27 files
- With TypeOK: 8 files
- Without TypeOK: 19 files (70% failure rate)

**Complete list of 35 non-bead .tla files lacking TypeOK:**

```
verification/tla/AtomicAcceptedRunAdmission.tla
verification/tla/CapabilityLifecycle.tla
verification/tla/ConcurrencyControl.tla
verification/tla/EngineYamlAdmission.tla
verification/tla/EngineYamlIngress.tla
verification/tla/EngineYamlRunLifecycle.tla
verification/tla/IdempotencySafety.tla
verification/tla/LifecycleJournal.tla
verification/tla/RecoveryCrashRestart.tla
verification/tla/RecoveryHydration.tla
verification/tla/RetryFSM.tla
verification/tla/V1PrimitiveLowering.tla
verification/tla/VbKyyfReplayDeterminism.tla
verification/tla/WorkflowBoundedAdmission.tla
verification/tla/YamlE2eChain.tla
verification/tla/specs/ActionRouting.tla
verification/tla/specs/MultiShardRuntime.tla
verification/tla/specs/RunLifecycle.tla
verification/tla/specs/ShardProcessing.tla
verification/tla/specs/TimerWheel.tla
specs/tla/AttemptTracking.tla
specs/tla/BoundedAdmission.tla
specs/tla/JournalBeforeDispatch.tla
specs/tla/RecoveryReplay.tla
specs/tla/ShardOwnership.tla
specs/tla/ShardScheduler.tla
specs/tla/StepState.tla
specs/tla/TaintLattice.tla
specs/LifecycleJournal.tla
specs/RetryFSM.tla
specs/RetryJournal.tla
specs/idempotency_gate/IdempotencyGate.tla
specs/vb_qi37_2_5/BoundednessSlice.tla
specs/vb_qi37_2_5/NestedBoundednessAdmission.tla
contracts/tla/ContractsAsData.tla
```

**Note:** 5 additional files in `.beads/` subdirectories also lack TypeOK:
- `.beads/vb-e4mt/specs/AggregateResourceSpec.tla`
- `.beads/vb-e4mt/specs/StepBudgetSpec.tla`
- `.beads/vb-e4mt/specs/WorkflowBudgetSpec.tla`
- `.beads/vb-core-lower-control-primitives/specs/ControlLowering.tla`
- `.beads/vb-rpch/evidence/specs/RecoveryReplayFull.tla`

**Finding:** TypeOK is missing from the vast majority of TLA+ files. Per the proof-reviewer skill's lethal findings: "Weak TLA+ model: missing TypeOK, meaningful invariant, deadlock stance, fairness/liveness decision, or non-toy bounds" is grounds for rejection. This is a critical gap that must be addressed before approval.

**Required fix:** Add TypeOK invariants to all 35 non-bead `.tla` files. Each TypeOK must constrain variable domains to finite, meaningful bounds matching the Rust implementation's limits.

---

### 9. Orphaned loom/proptest files deleted — PASS

**Evidence:**
```
$ find /home/lewis/src/velvet-ballistics -name "budget_concurrency.rs" -o -name "resource_bounds_properties.rs"
(no output)
```

Both orphaned files are confirmed absent.

---

### 10. vb_boundary_inventory and vb_benchmark have `forbid unsafe` — PASS

**Evidence:**
```
crates/vb_boundary_inventory/src/lib.rs:1: #![forbid(unsafe_code)]
crates/vb_benchmark/src/lib.rs:1: #![forbid(unsafe_code)]
```

Both crates correctly declare `#![forbid(unsafe_code)]` at the top of their lib.rs files.

---

## Lethal Findings Summary

1. **TLA+ TypeOK Gap (CRITICAL):** 35 of 48 non-bead TLA+ files lack TypeOK invariants. This violates the proof-reviewer skill's lethal finding for "missing TypeOK" and represents a fundamental weakness in the temporal specifications.

2. **Kani File Path Mismatch (HIGH):** GOD RULE 1 FIX headers exist in the wrong crate (`vb_core` instead of `vb_compile`). While the fix content is correct, the files are not at the expected paths, indicating either a migration was incomplete or the review request tracked incorrect paths.

---

## Recommendations

### Immediate (blocks approval)
1. Add TypeOK invariants to all 35 non-bead `.tla` files listed above.
2. Resolve Kani file path discrepancy — either move files to `vb_compile` or update documentation to reference correct `vb_core` paths.

### Secondary
3. Consider consolidating or removing the 5 `.tla` files inside `.beads/` directories if they are stale bead artifacts.
4. Add a CI gate that greps for `TypeOK` in all `.tla` files to prevent regression.

---

STATUS: REJECTED
