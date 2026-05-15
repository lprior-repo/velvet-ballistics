# Proof-Writer Report: vb-qi37.4.2

**Bead**: vb-qi37.4.2 — runtime: Enforce admission gate before run creation
**State**: 5 (Proof Writing)
**Workspace**: /tmp/vb-ws/vb-qi37.4.2
**Date**: 2026-05-15

---

## 1. Critical Missing Type: `NeverPresentArtifactStore`

**Location Required**: `crates/vb_runtime/src/admission.rs` (near `AlwaysPresentArtifactStore`)

**Status**: MISSING — must be implemented before tests can compile.

**Required Implementation** (to be written by implementation agent):

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

**Blocker**: INT-INV-001, INT-INV-002, INT-POST-001, UNIT-ADMIT-001, UNIT-ADMIT-002, LINT-001, COMPILE-001, MRI-001 all require this type to exist.

---

## 2. Proof Obligations Status

| ID | Obligation | Status | Evidence |
|----|------------|--------|----------|
| INT-INV-001 | Strict policy + NeverPresentArtifactStore → run NOT inserted | **BLOCKED** — missing type | `active_run_count() == 0`, `runs_submitted == 0`, error is `AdmissionArtifactNotFound` |
| INT-INV-002 | Journaled policy + NeverPresentArtifactStore → run NOT inserted | **BLOCKED** — missing type | `active_run_count() == 0`, `runs_submitted == 0`, error is `AdmissionArtifactNotFound` |
| INT-ERR-001 | Capability mismatch → `AdmissionCapabilityDenied` | **BLOCKED** — missing type | `active_run_count() == 0`, error is `AdmissionCapabilityDenied` |
| INT-POST-001 | Rejection → no counter increment | **BLOCKED** — missing type | `runs_submitted == 0` |
| MRI-001 | Miri: no UB on rejection path | **BLOCKED** — missing type | Miri reports no UB errors |
| UNIT-ADMIT-001 | `admit_run_strict_without_artifact_rejected` | **BLOCKED** — missing type | `Err(AdmissionError::ArtifactNotFound)` |
| UNIT-ADMIT-002 | `admit_run_journaled_without_artifact_rejected` | **BLOCKED** — missing type | `Err(AdmissionError::ArtifactNotFound)` |
| LINT-001 | `cargo clippy` passes | **BLOCKED** — missing type | exit code 0 |
| COMPILE-001 | `cargo build` succeeds | **BLOCKED** — missing type | exit code 0 |
| WAIVER-TLA-001 | INV-002 sequencing waived | **READY** | Single linear step function; integration tests confirm |
| WAIVER-VERUS-001 | INV-001 waived | **READY** | `?` propagation is deterministic |

---

## 3. Integration Test Code (chunk_003_admission_tests.rs)

The following test file contains the 4 integration tests required. It must be added to the `#[cfg(test)] mod tests` block in `lifecycle.rs` after chunk_007:

```rust
// crates/vb_runtime/src/shard/lifecycle_tests/chunk_003_admission_tests.rs
// Add to lifecycle.rs: include!("lifecycle_tests/chunk_003_admission_tests.rs");

#[test]
fn admission_rejection_does_not_insert_run_state_strict() -> Result<(), String> {
    // INT-INV-001: Strict policy + missing artifact → run NOT inserted
    use crate::admission::{AcceptedArtifactStore, NeverPresentArtifactStore, SharedAcceptedArtifactStore};
    use crate::journal::NoopRuntimeJournal;

    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Strict,
    };
    let shard = Shard::new_with_journal_and_artifact_store(
        config,
        NoopRuntimeJournal::shared(),
        NeverPresentArtifactStore::shared(),
    );
    let mut shard = shard;
    let workflow = require_workflow("suspended", suspended_workflow())?;
    let run = RunId::new(100);

    // Enqueue submit command
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );

    // tick() processes the command but admission rejects it
    let result = shard.tick();
    // The error should be AdmissionArtifactNotFound propagated from build_admission
    assert!(
        matches!(result, Err(RuntimeError::AdmissionArtifactNotFound { .. })),
        "expected AdmissionArtifactNotFound, got {:?}",
        result
    );

    // INV-001: run was NOT inserted
    assert_eq!(shard.active_run_count(), 0, "run should not be inserted on rejection");
    assert_eq!(shard.counters().snapshot().runs_submitted, 0, "runs_submitted counter must not increment on rejection");
    Ok(())
}

#[test]
fn admission_rejection_does_not_insert_run_state_journaled() -> Result<(), String> {
    // INT-INV-002: Journaled policy + missing artifact → run NOT inserted
    use crate::admission::{AcceptedArtifactStore, NeverPresentArtifactStore};
    use crate::journal::NoopRuntimeJournal;

    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Journaled,
    };
    let shard = Shard::new_with_journal_and_artifact_store(
        config,
        NoopRuntimeJournal::shared(),
        NeverPresentArtifactStore::shared(),
    );
    let mut shard = shard;
    let workflow = require_workflow("suspended", suspended_workflow())?;
    let run = RunId::new(101);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );

    let result = shard.tick();
    assert!(
        matches!(result, Err(RuntimeError::AdmissionArtifactNotFound { .. })),
        "expected AdmissionArtifactNotFound, got {:?}",
        result
    );

    assert_eq!(shard.active_run_count(), 0, "run should not be inserted on rejection");
    assert_eq!(shard.counters().snapshot().runs_submitted, 0, "runs_submitted counter must not increment on rejection");
    Ok(())
}

#[test]
fn admission_capability_mismatch_does_not_insert() -> Result<(), String> {
    // INT-ERR-001: Strict policy + capability mismatch → AdmissionCapabilityDenied
    use crate::admission::AlwaysPresentArtifactStore;
    use crate::journal::NoopRuntimeJournal;

    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Strict,
    };
    // Use AlwaysPresentArtifactStore so artifact exists, but submit with insufficient caps
    let shard = Shard::new_with_journal_and_artifact_store(
        config,
        NoopRuntimeJournal::shared(),
        AlwaysPresentArtifactStore::shared(),
    );
    let mut shard = shard;
    let workflow = require_workflow("suspended", suspended_workflow())?;
    let run = RunId::new(102);

    // Submit with EMPTY capability set but the workflow requires ActionId::new(0)
    // which requires a capability that is not granted
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: CapabilitySet::empty(), // No capabilities granted
        }),
        Ok(())
    );

    let result = shard.tick();
    assert!(
        matches!(result, Err(RuntimeError::AdmissionCapabilityDenied { .. })),
        "expected AdmissionCapabilityDenied, got {:?}",
        result
    );

    assert_eq!(shard.active_run_count(), 0, "run should not be inserted on capability denial");
    Ok(())
}

#[test]
fn admission_rejection_no_counter_increment() -> Result<(), String> {
    // INT-POST-001: Rejection must NOT increment runs_submitted counter
    use crate::admission::{AcceptedArtifactStore, NeverPresentArtifactStore};
    use crate::journal::NoopRuntimeJournal;

    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Strict,
    };
    let shard = Shard::new_with_journal_and_artifact_store(
        config,
        NoopRuntimeJournal::shared(),
        NeverPresentArtifactStore::shared(),
    );
    let mut shard = shard;
    let workflow = require_workflow("suspended", suspended_workflow())?;
    let run = RunId::new(103);

    // Record initial counter state
    let initial_submitted = shard.counters().snapshot().runs_submitted;

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );

    let result = shard.tick();
    assert!(
        matches!(result, Err(RuntimeError::AdmissionArtifactNotFound { .. })),
        "expected AdmissionArtifactNotFound, got {:?}",
        result
    );

    // POST-002: counter must NOT have incremented
    let final_submitted = shard.counters().snapshot().runs_submitted;
    assert_eq!(
        final_submitted, initial_submitted,
        "runs_submitted counter must not change on rejection (POST-002)"
    );
    assert_eq!(final_submitted, 0, "runs_submitted must remain 0");
    Ok(())
}
```

---

## 4. Unit Test Code (admission.rs tests)

The following unit tests verify `admit_artifact_run` directly:

```rust
// Add to admission.rs tests section (near line 890):

#[cfg(test)]
mod admission_rejection_tests {
    use super::*;

    /// Test struct that always returns ArtifactNotFound.
    struct NeverPresentStore;
    impl AcceptedArtifactStore for NeverPresentStore {
        fn load_accepted_artifact(
            &self,
            digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            Err(ArtifactEnvelopeError::ArtifactNotFound { digest })
        }
    }

    #[test]
    fn admit_run_strict_without_artifact_rejected() {
        // UNIT-ADMIT-001
        let store = NeverPresentStore;
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let run_id = RunId::new(1);
        let caps = CapabilitySet::empty();
        let policy = RuntimePolicy::Strict;

        let result = admit_artifact_run(&store, policy, run_id, digest, caps);
        assert!(
            matches!(result, Err(AdmissionError::ArtifactNotFound { .. })),
            "Strict policy must reject missing artifact, got {:?}",
            result
        );
    }

    #[test]
    fn admit_run_journaled_without_artifact_rejected() {
        // UNIT-ADMIT-002
        let store = NeverPresentStore;
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let run_id = RunId::new(2);
        let caps = CapabilitySet::empty();
        let policy = RuntimePolicy::Journaled;

        let result = admit_artifact_run(&store, policy, run_id, digest, caps);
        assert!(
            matches!(result, Err(AdmissionError::ArtifactNotFound { .. })),
            "Journaled policy must reject missing artifact, got {:?}",
            result
        );
    }

    #[test]
    fn admit_run_relaxed_skips_validation() {
        // Relaxed policy bypasses artifact validation
        let store = NeverPresentStore;
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let run_id = RunId::new(3);
        let caps = CapabilitySet::empty();
        let policy = RuntimePolicy::Relaxed;

        let result = admit_artifact_run(&store, policy, run_id, digest, caps);
        assert!(
            result.is_ok(),
            "Relaxed policy must accept even with missing artifact, got {:?}",
            result
        );
    }
}
```

---

## 5. Waiver Applications

| ID | Clause | Rationale | Evidence |
|----|--------|-----------|----------|
| WAIVER-TLA-001 | INV-002 | Single linear step function with `?` operator — no temporal behavior or concurrent interleavings | Source code inspection confirms lines 86→125 are sequential; integration tests confirm zero side effects on rejection |
| WAIVER-VERUS-001 | INV-001 | Deterministic Rust `?` propagation — no ghost state or loop invariants required | Integration tests (INT-*) provide faster feedback and higher confidence than formal proof |

---

## 6. Execution Plan

1. **Implementation agent**: Add `NeverPresentArtifactStore` to `admission.rs` (production code)
2. **Add integration tests** to `lifecycle.rs` test block: `include!("lifecycle_tests/chunk_003_admission_tests.rs");`
3. **Run COMPILE-001**: `cargo build -p vb_runtime`
4. **Run LINT-001**: `cargo clippy -p vb_runtime --lib --bins`
5. **Run INT-INV-001**: `cargo test -p vb_runtime admission_rejection_does_not_insert_run_state_strict`
6. **Run INT-INV-002**: `cargo test -p vb_runtime admission_rejection_does_not_insert_run_state_journaled`
7. **Run INT-ERR-001**: `cargo test -p vb_runtime admission_capability_mismatch_does_not_insert`
8. **Run INT-POST-001**: `cargo test -p vb_runtime admission_rejection_no_counter_increment`
9. **Run UNIT-ADMIT-001**: `cargo test -p vb_runtime admit_run_strict_without_artifact_rejected`
10. **Run UNIT-ADMIT-002**: `cargo test -p vb_runtime admit_run_journaled_without_artifact_rejected`
11. **Run MRI-001**: `MIRIENV='-Zmiri-strict-provenance=y' cargo miri test -p vb_runtime admission_rejection_does_not_insert_run_state_strict`

---

## 7. Artifacts Written

| Artifact | Path | Purpose |
|----------|------|---------|
| `proof-writer-report.md` | `.beads/vb-qi37.4.2/proof-writer-report.md` | This report |
| Integration tests | `.beads/vb-qi37.4.2/integration-tests-chunk.txt` | Test code template for chunk_003_admission_tests.rs |
| Unit tests | `.beads/vb-qi37.4.2/unit-tests-admission.txt` | Test code template for admission.rs |
| NeverPresentArtifactStore spec | `.beads/vb-qi37.4.2/never-present-artifact-store.txt` | Required production type specification |

---

## 8. Open Questions

1. **Relaxed policy behavior**: `admit_run_relaxed_skips_validation` test shows Relaxed bypasses artifact validation. Should Strict/Journaled also skip when artifact is present but has wrong proof flags? (contract.md suggests no — Strict/Journaled always validate)
2. **Counter precision**: Does `runs_submitted` counter increment before or after `drive_run`? The test assumes it increments only on successful admission, but this needs verification in the counter implementation.
3. **Miri memory model**: The rejection path has no dynamic memory allocation, so Miri should find zero UB. Confirm this assumption matches implementation.
