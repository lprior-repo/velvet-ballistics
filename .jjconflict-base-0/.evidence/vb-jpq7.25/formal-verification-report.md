# Formal Verification Report: vb-jpq7.25

**Date:** 2026-05-22
**Agent:** formal-verifier
**Bead:** vb-jpq7.25 - Repair Kani harness discovery and generators
**Repository:** /home/lewis/src/velvet-ballistics

---

## Executive Summary

This bead repairs Kani proof harnesses in the velvet-ballistics repository. The goal was to replace zero-discovery and hardcoded structural Kani harnesses with real per-crate harness discovery and arbitrary/exhaustive generators.

**Finding:** The root-level `kani/` directory contains OLD-style disconnected harnesses with hardcoded shapes. The proper Arbitrary-based harnesses exist in `crates/vb_core/` and `crates/vb_validate/`. However, several crates have broken Kani harnesses due to missing `kani::Arbitrary` implementations and one crate causes a Kani internal panic.

---

## Evidence: Cargo Kani List Results

### Crates with Working Harnesses

#### vb_core (139 harnesses)
```bash
$ cd /home/lewis/src/velvet-ballistics/crates/vb_core && cargo kani list
...
Total | 139 |
```

**Status:** ✅ PASS - All 139 harnesses compile and are discovered

Key harness files with proper Arbitrary implementations:
- `kani_workflow_arbitrary.rs` - 500 lines implementing Arbitrary for WorkflowParts, CompiledNode, CompiledNodeKind, Taint, SlotValue
- `kani_id_arbitrary.rs` - Arbitrary for RunId, SeqNo, EventSeq, StepIdx, SlotIdx, ActionId, WorkflowDigest
- `kani_workflow_budget_harnesses.rs` - Arbitrary for AggregateResourceUsage, AggregateResourceBudget, AggregateResourceCapacity, StepBudget

#### vb_validate (26 harnesses)
```bash
$ cd /home/lewis/src/velvet-ballistics/crates/vb_validate && cargo kani list
...
Total | 26 |
```

**Status:** ✅ PASS - All 26 harnesses compile and are discovered

---

### Crates with Broken Harnesses

#### vb_storage (0 harnesses - 36 compilation errors)
```
$ cd /home/lewis/src/velvet-ballistics/crates/vb_storage && cargo kani list
error: could not compile `vb_storage` (lib) due to 36 previous errors
```

**Status:** ❌ FAIL_LOCAL - 36 compilation errors

**Errors:**
```
error[E0432]: unresolved import `crate::recovery::replay::summary::recover_runtime_summary_from_events`
error[E0277]: the trait bound `types::EventSeq: kani::Arbitrary` is not satisfied
error[E0277]: the trait bound `vb_core::CapabilitySet: kani::Arbitrary` is not satisfied
error[E0277]: the trait bound `vb_core::RuntimePolicy: kani::Arbitrary` is not satisfied
error[E0277]: the trait bound `chrono::DateTime<chrono::Utc>: kani::Arbitrary` is not satisfied
error[E0277]: the trait bound `journal::core::FjallJournal: kani::Arbitrary` is not satisfied
error[E0063]: missing field `seq` in initializer of `events::JournalEvent`
```

**Missing Arbitrary implementations:**
- `types::EventSeq`
- `vb_core::CapabilitySet`
- `vb_core::RuntimePolicy`
- `chrono::DateTime<chrono::Utc>`
- `journal::core::FjallJournal`

**Broken harness files:**
- `kani_admission.rs` - uses FjallJournal, RuntimePolicy without Arbitrary
- `kani_recovery_hydrate.rs` - missing EventSeq Arbitrary, has JournalEvent construction errors

#### vb_compile (Kani internal panic)
```
$ cd /home/lewis/src/velvet-ballistics/crates/vb_compile && cargo kani list
thread 'rustc' panicked at kani-compiler/src/codegen_cprover_gotoc/overrides/hooks.rs:158:51:
called `Option::unwrap()` on a `None` value
```

**Status:** ❌ FAIL_GLOBAL - Kani internal compiler panic

**Panic location:** `crates/vb_compile/src/kani_foreach_parity.rs:546` - `foreach_arbitrary_done_forward`

This is a Kani bug, not a harness code issue. The harness uses `kani::any::<WorkflowParts>()` which works in other crates.

#### vb_ipc, vb_runtime (Transitively broken)
These crates depend on vb_storage and cannot compile their Kani harnesses due to the vb_storage errors.

---

## Root-Level `kani/` Directory Analysis

### Location
`/home/lewis/src/velvet-ballistics/kani/`

### Files Found (19 harness files)
```
admission_atomic_sequence_k01_k03.rs
decision_table_at_least_once_rejected.rs
decision_table_deterministic_rejected.rs
decision_table_ok_branch.rs
decision_table_unsafe_rejected.rs
gate_07_stack.rs
gate_09_slots.rs
gate_10_node.rs
gate_11_loop.rs
gate_12_14_15.rs
idempotency_gate_parity.rs
is_statically_idempotent_contract.rs
pipeline.rs
verify_idempotency_all_clean.rs
verify_idempotency_missing_key.rs
verify_idempotency_random_in_key.rs
verify_idempotency_secret_in_key.rs
verify_idempotency_single_error.rs
verify_idempotency_time_in_key.rs
```

### Status: NOT DISCOVERED
These files are NOT discovered by `cargo kani list` because they are not part of any crate's compilation unit. They exist as orphaned files in the repository root.

### Issues with Root-Level Harnesses

1. **Hardcoded WorkflowParts shapes** - e.g., `gate_07_stack.rs` lines 26-46:
```rust
let parts = WorkflowParts {
    name: Box::from("kani_g7"),
    digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
    nodes: Box::new([vb_core::workflow::CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: vb_core::workflow::CompiledNodeKind::Finish { ... },
    }]),
    // ...
};
```

2. **Hardcoded RunFrame shapes** - e.g., `verify_idempotency_all_clean.rs` lines 37-54:
```rust
let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 4);
kani::assume(frame.is_ok());
let mut frame = frame.ok().unwrap();
// Populate slots manually
```

3. **No use of kani::Arbitrary** - These harnesses manually construct specific shapes instead of using the Arbitrary generators already available in `crates/vb_core/src/kani_workflow_arbitrary.rs`

---

## Proper Arbitrary Implementations (Already Exist)

### Location: `crates/vb_core/src/kani_workflow_arbitrary.rs`

The proper Arbitrary implementations exist and are used by vb_core's 139 harnesses:

```rust
impl kani::Arbitrary for WorkflowParts {
    fn any() -> Self {
        let node_count: u8 = kani::any();
        kani::assume(node_count <= 8);
        // ... generates arbitrary nodes, expressions, constants, etc.
    }
}

impl kani::Arbitrary for CompiledNode {
    fn any() -> Self {
        let id = StepIdx::new(kani::any());
        // ... all fields generated with kani::any()
    }
}

impl kani::Arbitrary for CompiledNodeKind {
    fn any() -> Self {
        match kani::any::<u8>() {
            // ... all 33+ node kinds covered
        }
    }
}
```

---

## Summary of Changes Required

### High Priority (Blocking)

1. **vb_storage Arbitrary implementations** - Add `kani::Arbitrary` for:
   - `types::EventSeq` (in vb_storage or vb_core)
   - `vb_core::CapabilitySet`
   - `vb_core::RuntimePolicy`
   - `chrono::DateTime<chrono::Utc>` (stub is acceptable)
   - `journal::core::FjallJournal` (stub with kani::any() returning default)

2. **vb_compile Kani panic** - This is a Kani internal bug. Workaround needed:
   - Either disable the `foreach_arbitrary_done_forward` harness
   - Or simplify the harness to not trigger the Kani panic

### Medium Priority

3. **Root-level `kani/` directory** - These are orphaned files:
   - Either migrate them into vb_validate (if they're validating gate logic)
   - Or remove them if they're superseded by the Arbitrary-based harnesses

4. **vb_storage harness construction errors** - The `kani_recovery_hydrate.rs` has `JournalEvent` construction errors (missing `seq` field)

---

## Remaining Blockers

1. **Missing Arbitrary implementations** - Cannot run vb_storage, vb_ipc, vb_runtime Kani harnesses until `EventSeq`, `CapabilitySet`, `RuntimePolicy`, `DateTime<Utc>`, and `FjallJournal` have Arbitrary implementations

2. **Kani internal panic in vb_compile** - The `foreach_arbitrary_done_forward` harness causes Kani to panic. This is a Kani bug, not a code issue.

3. **Root-level orphaned harnesses** - 19 harness files in `kani/` are not discoverable and use hardcoded shapes instead of Arbitrary

---

## Evidence Files

- `/home/lewis/src/velvet-ballistics/verification/kani/` - Placeholder files pointing to canonical harness locations
- `/home/lewis/src/velvet-ballistics/crates/vb_core/src/kani_workflow_arbitrary.rs` - Proper Arbitrary implementations
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/kani_recovery_hydrate.rs` - Broken harness with missing Arbitrary
- `/home/lewis/src/velvet-ballistics/crates/vb_compile/src/kani_foreach_parity.rs` - Causes Kani panic

---

## Verification Ledger

| Crate | Harnesses | Status | Notes |
|-------|-----------|--------|-------|
| vb_core | 139 | ✅ PASS | Proper Arbitrary usage |
| vb_validate | 26 | ✅ PASS | Proper Arbitrary usage |
| vb_storage | 0 | ❌ FAIL_LOCAL | 36 compilation errors |
| vb_compile | ? | ❌ FAIL_GLOBAL | Kani internal panic |
| vb_ipc | N/A | ❌ FAIL_LOCAL | Transitive vb_storage error |
| vb_runtime | N/A | ❌ FAIL_LOCAL | Transitive vb_storage error |
| workspace_tests | 0 | ✅ PASS | No Kani harnesses defined |
| root kani/ | 19 | ❌ FAIL_LOCAL | Orphaned, not discovered |
