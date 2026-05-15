# Proof Strategy: vb-qi37.4.2

## Bead
**vb-qi37.4.2** — runtime: Enforce admission gate before run creation
**State**: 4 (Proof Planning)
**Goal**: Prove admission gate (line 86 `build_admission`) runs BEFORE frame allocation (line 87 `take_frame_for`), journal events (lines 91–111), and `runs.insert` (line 125).

---

## Verification Scope

### Scope Files
| File | Risk Tags | Verifier Modes |
|------|-----------|----------------|
| `crates/vb_runtime/src/admission.rs` | `persistence`, `public_api`, `user_visible_behavior` | `miri`, `integration_test` |
| `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` | `persistence`, `concurrency` | `miri`, `integration_test` |
| `crates/vb_runtime/src/shard/lifecycle_tests/chunk_003.rs` | `persistence` | `miri`, `integration_test` |
| `crates/vb_runtime/src/error/mod.rs` | `user_visible_behavior` | `integration_test` |

### Critical Gap
The existing test `admission_rejection_does_not_insert_run_state` (chunk_003.rs:53) uses `Relaxed` policy and asserts run IS inserted (`active_run_count() == 1`). Does NOT test rejection.

The contract requires `NeverPresentArtifactStore` (implements `AcceptedArtifactStore`) that always returns `ArtifactNotFound`. This type does NOT yet exist at module level — it must be created in `admission.rs`.

---

## Admission Gate Sequencing

**INV-002** sequencing in `handle_submit_with_inputs_contracts_and_header_mode` (lines 86–125):
```
Line 86:  build_admission(run, digest, caps)?  ← GATE (evaluated first)
Line 87:  take_frame_for(run, &workflow)?       ← Frame allocated AFTER admission
Line 89:  trace_ring.push(RunSubmitted)
Line 91–100: journal RunSubmitted               ← Journaled AFTER admission
Line 102–111: journal RunAdmission             ← Journaled AFTER admission
Line 125: self.runs.insert(run, state)         ← Run created AFTER admission
```
**Critical**: If `build_admission` fails, `?` propagates — all subsequent steps (frame allocation, journal, run insertion) are skipped.

---

## Risk Classification

| Risk | Trigger | Verifier Lane |
|------|---------|---------------|
| UB in rejection path | unsafe code forbidden; `?` propagation of Result | Miri |
| Sequencing correctness | INV-001 (deterministic linear control flow) | Integration test |
| Capability denial | INV-001 variant | Integration test |
| Journal event absence | POST-002 | Integration test |
| Static analysis | crate build + clippy | cargo build + clippy |

---

## Proof Lane Selection

### Lanes NOT Required

| Lane | Reason |
|------|--------|
| **TLA+** | INV-002 is a single linear step function with no branching, concurrency, or temporal behavior. The sequencing is enforced by the `?` operator — deterministic Rust control flow, not a state machine. |
| **Verus** | INV-001 is a Rust-local invariant deterministically enforced by `?` propagation. No ghost state or loop invariants needed. Integration tests provide faster feedback. |
| **Kani** | No bounded state machine or harness needed for this bead. The rejection path is a simple `Result` return. |
| **Loom** | Single-shard execution with no concurrent interleavings at the shard level. |
| **Flux** | No refinement types or numeric predicates involved. |
| **proptest/fuzz** | No broad input space — the rejection condition is a single deterministic path. |

### Lanes Required

| Lane | Obligation | Evidence |
|------|------------|----------|
| **Integration Test** | INT-INV-001, INT-INV-002, INT-ERR-001, INT-POST-001 | `active_run_count() == 0`, `runs_submitted == 0`, correct error variant |
| **Miri** | MRI-001 | No UB reported on rejection path |
| **cargo test** | UNIT-ADMIT-001, UNIT-ADMIT-002 | `Err(AdmissionError::ArtifactNotFound)` returned |
| **cargo clippy** | LINT-001 | exit code 0, no errors |
| **cargo build** | COMPILE-001 | exit code 0 |

---

## Waiver Entries

| ID | Clause | Reason | Compensating Evidence |
|----|--------|--------|----------------------|
| WAIVER-TLA-001 | INV-002 | Single atomic step function; no temporal behavior or concurrent interleavings | Integration tests confirm linear sequencing |
| WAIVER-VERUS-001 | INV-001 | Deterministic `?` propagation; Rust control flow verifiable by inspection + integration test | Integration tests for rejection paths |

---

## Artifact Changes Required

### New Type: `NeverPresentArtifactStore`

**Location**: `crates/vb_runtime/src/admission.rs` (module-level, near `AlwaysPresentArtifactStore`)

```rust
/// Artifact store that always reports artifacts as absent.
/// Used to trigger admission rejection under Strict/Journaled policy.
#[derive(Debug, Default)]
pub struct NeverPresentArtifactStore;

impl NeverPresentArtifactStore {
    #[must_use]
    pub fn shared() -> SharedAcceptedArtifactStore {
        Arc::new(Self)
    }
}

impl AcceptedArtifactStore for NeverPresentArtifactStore {
    fn load_accepted_artifact(
        &self,
        digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
        Err(ArtifactEnvelopeError::ArtifactNotFound { digest })
    }
}
```

### New Integration Tests in `chunk_003.rs`

| Test Name | Policy | Artifact Store | Assertions |
|-----------|--------|----------------|------------|
| `admission_rejection_does_not_insert_run_state_strict` | Strict | NeverPresentArtifactStore | `active_run_count() == 0`, `runs_submitted == 0` |
| `admission_rejection_does_not_insert_run_state_journaled` | Journaled | NeverPresentArtifactStore | `active_run_count() == 0`, `runs_submitted == 0` |
| `admission_capability_mismatch_does_not_insert` | Strict | AlwaysPresentArtifactStore | `active_run_count() == 0`, error is `AdmissionCapabilityDenied` |
| `admission_rejection_no_counter_increment` | Strict | NeverPresentArtifactStore | `runs_submitted == 0` |

### Shard Constructor for Strict/Journaled Tests

New helper config:
```rust
fn strict_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: RuntimePolicy::Strict,
    }
}

fn make_strict_shard() -> Shard {
    Shard::new_with_journal_and_artifact_store(
        strict_config(),
        NoopRuntimeJournal::shared(),
        NeverPresentArtifactStore::shared(),
    )
}
```

---

## Admission Gate Sequencing (Critical Ordering Property)

INV-002 is a **sequencing invariant** — the order of operations is enforced by the `?` operator at line 86:

1. `build_admission` returns `Err` → all subsequent lines 87–125 are skipped
2. `build_admission` returns `Ok` → lines 87–125 execute sequentially

This is **NOT** a temporal property requiring TLA+. It is a **control-flow ordering** property enforced by Rust's deterministic sequential evaluation and the `?` operator's early-return semantics.

**Evidence**: Source code lines 86 (`?`) → 87 → 125 in `lifecycle/chunk_001.rs`.

---

## Execution Order

1. **First**: Create `NeverPresentArtifactStore` type in `admission.rs`
2. **Then**: Write integration tests in `chunk_003.rs` using `NeverPresentArtifactStore`
3. **Then**: Run `cargo build -p vb_runtime` (COMPILE-001)
4. **Then**: Run `cargo clippy -p vb_runtime --lib --bins` (LINT-001)
5. **Then**: Run `cargo test -p vb_runtime admission_rejection_does_not_insert_run_state_strict` (INT-INV-001)
6. **Then**: Run `MIRIENV='-Zmiri-strict-provenance=y' cargo miri test` (MRI-001)

---

## Summary

- **INV-002 sequencing** (admission before all other steps): Verified by inspection of `?` propagation; confirmed by integration tests asserting zero side effects on rejection
- **INV-001 run-not-inserted**: Verified by integration tests using `NeverPresentArtifactStore` + Strict/Journaled policy
- **No TLA+ needed**: Single linear step function with no branching or concurrency
- **No Verus needed**: Deterministic Rust control flow; integration tests faster and more maintainable
- **No Kani/Loom/Flux/proptest/fuzz**: Risk scope does not trigger these lanes
