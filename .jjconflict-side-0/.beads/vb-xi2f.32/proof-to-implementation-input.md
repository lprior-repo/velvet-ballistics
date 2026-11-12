# Proof-to-Implementation Bridge Input: Wait Digest Coverage

**Bead:** vb-xi2f.32
**Date:** 2026-05-24
**State:** proof-planner (State 4)
**Schema:** proof-to-implementation-input/v1

This document provides the bridge (`proof-to-implementation` agent) with the necessary mappings to translate approved proof claims into Rust implementation obligations, test obligations, and harness requirements.

## 1. Implementation Source Refs (MUST FIX)

These are the exact locations that require code changes:

### Primary fix locations

| Ref ID | File | Line | Symbol | Change |
|--------|------|------|--------|--------|
| IMPL-001 | `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | 140-162 | `digest_step_primitive` | Add `Wait { event, timeout }` match arm BEFORE the `other =>` catch-all arm. Hash discriminator + field values. |
| IMPL-002 | `crates/vb_compile/src/compile/mod.rs` | 243-261 | `digest_step_primitive` | Same change as IMPL-001. Must be byte-for-byte identical in behavior. |

### No changes needed (but referenced by proofs)

| Ref ID | File | Role |
|--------|------|------|
| IMPL-003 | `crates/vb_compile/src/mod_compile_lowering/part_01.rs:46` | Calls `canonical_digest(source)` — unchanged, but any test calling `compile_source` exercises this path |
| IMPL-004 | `crates/vb_compile/src/compile/mod.rs:220` | `canonical_digest` — unchanged, but calls `digest_step_primitive` |
| IMPL-005 | `crates/vb_core/src/ids/mod.rs:342` | `WorkflowDigest` — unchanged type |
| IMPL-006 | `crates/vb_yaml/src/ast/types.rs:238` | `StepPrimitive::Wait { event: Option<String>, timeout: Option<String> }` — unchanged type |

## 2. Expected Code Pattern for IMPL-001 / IMPL-002

The new match arm should be added **before** the `other =>` catch-all arm:

```rust
vb_yaml::ast::StepPrimitive::Wait { event, timeout } => {
    // Discriminator: "wait_until" vs "wait_event"
    if event.is_some() {
        hasher.update(b"wait_event");
        // Hash event field value
        if let Some(event_val) = event.as_deref() {
            hasher.update(event_val.as_bytes());
        }
    } else {
        hasher.update(b"wait_until");
    }
    // Hash timeout field or sentinel
    match timeout.as_deref() {
        Some(timeout_val) => hasher.update(timeout_val.as_bytes()),
        None => hasher.update(b"none"),
    }
}
```

**Key design decisions:**
1. Discriminator `"wait_until"` / `"wait_event"` is hashed FIRST to distinguish WaitUntil from WaitEvent.
2. For WaitUntil (event=None), no event value is hashed — only discriminator + timeout.
3. For WaitEvent (event=Some), the event value is hashed AFTER the discriminator.
4. For WaitEvent-unbounded (timeout=None), the sentinel `"none"` is hashed.
5. For WaitEvent-bounded (timeout=Some), the timeout value is hashed.
6. The `event` field is never `None` after `is_some()` check, so `unwrap()` is not needed — use `if let Some`.
7. The fix must never introduce `unwrap()`, `expect()`, `panic!`, `todo!`, `unsafe`, or unchecked indexing.

## 3. Test Obligations (MUST WRITE)

### New proptest properties

| Test ID | File | Test Name | Proof Obligation |
|---------|------|-----------|-----------------|
| TEST-001 | `crates/vb_compile/tests/v1_primitive_lowering.rs` | `proptest_wait_field_sensitivity` | PO-002 |
| TEST-002 | `crates/vb_compile/tests/v1_primitive_lowering.rs` | `proptest_wait_until_vs_wait_event` | PO-004 |
| TEST-003 | `crates/vb_compile/tests/v1_primitive_lowering.rs` | `proptest_wait_sentinel_unambiguous` | PO-006 |
| TEST-004 | `crates/vb_compile/tests/v1_primitive_lowering.rs` | `proptest_wait_pairwise_distinct_digests` | PO-011 |
| TEST-005 | `crates/vb_compile/tests/` (new file or existing integration) | `cross_path_wait_digest_equivalence` | PO-009 |
| TEST-006 | `crates/vb_compile/tests/` (new file or existing integration) | `cross_path_digest_output_equivalence` | PO-016 |

### Existing tests (MUST STILL PASS)

| Test ID | File | Test Name | Proof Obligation |
|---------|------|-----------|-----------------|
| TEST-007 | `crates/vb_compile/tests/v1_primitive_lowering.rs:828` | `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` | PO-008 |
| TEST-008 | `crates/vb_compile/src/tests/error_variant_tests.rs:765` | `compiled_digest_is_deterministic` | PO-014 |
| TEST-009 | `crates/vb_compile/src/tests/error_variant_tests.rs:781` | `different_sources_produce_different_digests` | PO-014 |

### Additional integration tests (existing, must still pass)

| Test ID | File | Test Name | Concern |
|---------|------|-----------|---------|
| TEST-010 | `crates/vb_compile/tests/v1_primitive_lowering.rs:113` | Wait compile tests | Wait still compiles correctly |
| TEST-011 | `crates/vb_compile/tests/v1_primitive_lowering.rs:231` | WaitUntil shape test | WaitUntil IR shape unchanged |
| TEST-012 | `crates/vb_core/src/engine/tests/integration_step_behavior.rs:519` | WaitUntil engine test | Engine behavior unchanged |
| TEST-013 | `crates/vb_core/src/engine/tests/integration_step_behavior.rs:555` | WaitEvent engine test | Engine behavior unchanged |

## 4. Kani Harness Obligations (MUST WRITE)

| Harness ID | File | Harness Name | Proof Obligation |
|------------|------|-------------|-----------------|
| HARN-001 | `verification/kani/wait_digest_panic_freedom.rs` | `wait_digest_step_primitive_no_panic` | PO-001 |
| HARN-002 | `verification/kani/wait_digest_panic_freedom.rs` | `wait_digest_both_copies_no_panic` | PO-015 |
| HARN-003 | `verification/kani/wait_digest_discrimination.rs` | `wait_until_vs_wait_event_no_collision` | PO-005 |
| HARN-004 | `verification/kani/wait_digest_cross_path_equivalence.rs` | `cross_path_digest_step_primitive_equivalence` | PO-010 |
| HARN-005 | `verification/kani/wait_digest_exhaustive_collision.rs` | `wait_configurations_pairwise_distinct` | PO-013 |

**GOD RULE 1 compliance:** All Kani harnesses MUST use `kani::Arbitrary` or generator harnesses using `kani::any()` for Wait field values. Hardcoded dummy `WorkflowParts` or `StepPrimitive::Wait` structs with fixed strings are REJECTED.

## 5. Fuzz Target Obligations (MUST WRITE)

| Fuzz ID | File | Fuzz Target Name | Proof Obligation |
|---------|------|-----------------|-----------------|
| FUZZ-001 | `fuzz/fuzz_targets/wait_digest_sensitivity.rs` | `wait_digest_sensitivity` | PO-003 |
| FUZZ-002 | `fuzz/fuzz_targets/wait_sentinel_collision.rs` | `wait_sentinel_collision` | PO-007 |
| FUZZ-003 | `fuzz/fuzz_targets/wait_digest_exhaustive_collision.rs` | `wait_digest_exhaustive_collision` | PO-012 |

## 6. Proof Claim → Rust Source Mapping

Each proof claim must map to exact Rust source locations and behavioral expectations:

### Claim: "Different Wait event values produce different digests" (C1)
- **Proof:** PO-002 (proptest), PO-003 (fuzz), PO-001 (Kani)
- **Rust source:** `digest_step_primitive` Wait arm in part_05.rs:140 and compile/mod.rs:243
- **Behavior:** `hasher.update(event.as_deref().unwrap().as_bytes())` must be called for event=Some values
- **GOD RULE 2 check:** The proof must verify the actual Rust code, not a separate model

### Claim: "WaitUntil ≠ WaitEvent even with identical timeout text" (C2)
- **Proof:** PO-004 (proptest), PO-005 (Kani)
- **Rust source:** Discriminator branch in `digest_step_primitive` Wait arm
- **Behavior:** `b"wait_until"` hashed for event=None; `b"wait_event"` + event value for event=Some

### Claim: "Absent timeout ≠ timeout=Some(\"none\")" (C3)
- **Proof:** PO-006 (proptest), PO-007 (fuzz)
- **Rust source:** Timeout handling branch in Wait arm
- **Behavior:** `b"none"` sentinel hashed for timeout=None; actual value hashed for timeout=Some

### Claim: "canonical_digest remains deterministic" (C4)
- **Proof:** PO-008 (proptest), PO-014 (regression)
- **Rust source:** `canonical_digest` in part_05.rs:116 — no time/random/state introduced
- **Behavior:** Pure function property preserved

### Claim: "Both compiler paths produce identical digests" (C5)
- **Proof:** PO-009, PO-010, PO-016 (proptest + Kani)
- **Rust source:** Both copies of `digest_step_primitive` and `canonical_digest`
- **Behavior:** Cross-path digest equality for same WorkflowSource

### Claim: "New Wait arm is panic-free" (C1 via ps-wait-008)
- **Proof:** PO-001, PO-015 (Kani)
- **Rust source:** Wait match arm in both copies
- **Behavior:** No panic, no overflow, no assertion violation for any legal Wait field values

## 7. Refinement Harness References

After implementation, the following harnesses bridge proof claims to source:

| Proof Ref | Rust Source Ref | Harness |
|-----------|----------------|---------|
| PO-002 (wait field sensitivity) | `digest_step_primitive` Wait arm | proptest: `proptest_wait_field_sensitivity` |
| PO-004 (WaitUntil vs WaitEvent) | Discriminator branch | proptest: `proptest_wait_until_vs_wait_event` |
| PO-001 (panic-freedom) | Full Wait arm | Kani: `wait_digest_step_primitive_no_panic` |
| PO-005 (no collision) | Full Wait arm | Kani: `wait_until_vs_wait_event_no_collision` |
| PO-010 (cross-path) | Both copies | Kani: `cross_path_digest_step_primitive_equivalence` |

## 8. Dependency Map

```
Proof obligation (planned) ──► Implementation fix ──► Test suite pass ──► Proof execution
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
              part_05.rs     compile/mod.rs    Both copies tested
              Wait arm       Wait arm          for equivalence
```

## 9. Blockers and Preconditions

Before implementation can begin:

1. **proof-plan-reviewer** must approve this plan (writes `verifier-lane-review.jsonl` and `proof-plan-review.md`).
2. **proof-writer** must create harnesses, fuzz targets, and test scaffolding before the implementation fix is landed.
3. **Rust implementation** (by functional-rust or holzman-rust agent) must apply identical fix to BOTH copies.
4. **Bridge agent** (`proof-to-implementation`) maps approved claims to exact source obligations.

## 10. Non-Implementation Obligations (Documentation / Artifact)

| Item | Description |
|------|-------------|
| Release notes | Document digest change for Wait workflows. Existing persisted digests for Wait-containing workflows become invalid — recompilation required. |
| Follow-up bead | File bead for deduplication of `canonical_digest` / `digest_step_primitive` / `canonical_primitive_name` |
| Follow-up bead | File bead for broader digest fix (Ask, Do, Save, Choose, etc. primitives) |
