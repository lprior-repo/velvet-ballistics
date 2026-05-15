# Proof Repair Guide: vb-qi37.4.2

**Bead**: vb-qi37.4.2 — runtime: Enforce admission gate before run creation
**Blocker**: `NeverPresentArtifactStore` MISSING from `admission.rs`
**Workspace**: /tmp/vb-ws/vb-qi37.4.2
**Date**: 2026-05-15

---

## Critical Blocker

**Type**: `NeverPresentArtifactStore`
**Location Required**: `crates/vb_runtime/src/admission.rs` (near `AlwaysPresentArtifactStore` at line 206)
**Status**: MISSING — must be implemented before any proof obligation can execute

---

## NeverPresentArtifactStore Specification

### Definition (from contract.md line 19)

> `NeverPresentArtifactStore` — Artifact store (implementing `AcceptedArtifactStore`) that always returns `ArtifactNotFound` — used to trigger rejection under Strict/Journaled

### Contract Interface

The type must implement:
```rust
pub trait AcceptedArtifactStore {
    fn load_accepted_artifact(
        &self,
        digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>;
}
```

### Required Implementation

```rust
/// Artifact store that always reports artifacts as absent.
/// Used to trigger admission rejection under Strict/Journaled policy.
#[derive(Debug, Default)]
pub struct NeverPresentArtifactStore;

impl NeverPresentArtifactStore {
    /// Returns a shared reference wrapped in Arc.
    #[must_use]
    pub fn shared() -> Arc<Self> {
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

### Placement

Add immediately after `AlwaysPresentArtifactStore` in `admission.rs`. The existing `AlwaysPresentArtifactStore` is at lines 206–248. Insert the new type before the test section (around line 830).

### Dependencies

- `vb_storage::admission::ArtifactEnvelopeError::ArtifactNotFound` — already imported in admission.rs
- `vb_storage::admission::AcceptedArtifact` — already in scope
- `WorkflowDigest` — already in scope
- `Arc` from `std::sync` — check imports

---

## Downstream Impact

After implementing `NeverPresentArtifactStore`, these gates can execute:

| Obligation | Command | Expected |
|------------|---------|----------|
| COMPILE-001 | `cargo build -p vb_runtime` | exit code 0 |
| LINT-001 | `cargo clippy -p vb_runtime --lib --bins -- -D warnings` | exit code 0 |
| UNIT-ADMIT-001 | `cargo test -p vb_runtime admit_run_strict_without_artifact_rejected` | test passes |
| UNIT-ADMIT-002 | `cargo test -p vb_runtime admit_run_journaled_without_artifact_rejected` | test passes |
| INT-INV-001 | `cargo test -p vb_runtime admission_rejection_does_not_insert_run_state_strict` | active_run_count == 0 |
| INT-INV-002 | `cargo test -p vb_runtime admission_rejection_does_not_insert_run_state_journaled` | active_run_count == 0 |
| INT-ERR-001 | `cargo test -p vb_runtime admission_capability_mismatch_does_not_insert` | active_run_count == 0, error = AdmissionCapabilityDenied |
| INT-POST-001 | `cargo test -p vb_runtime admission_rejection_no_counter_increment` | runs_submitted == 0 |
| MRI-001 | `MIRIENV='-Zmiri-strict-provenance=y' cargo miri test -p vb_runtime admission_rejection_does_not_insert_run_state_strict` | no UB |

---

## Verification Steps After Repair

1. `cargo build -p vb_runtime` — must compile without errors
2. `cargo clippy -p vb_runtime --lib --bins -- -D warnings` — no errors
3. `cargo test -p vb_runtime admit_run_strict_without_artifact_rejected` — passes
4. `cargo test -p vb_runtime admit_run_journaled_without_artifact_rejected` — passes
5. `cargo test -p vb_runtime admission_rejection_does_not_insert_run_state_strict` — active_run_count == 0
6. `cargo test -p vb_runtime admission_rejection_does_not_insert_run_state_journaled` — active_run_count == 0
7. `cargo test -p vb_runtime admission_capability_mismatch_does_not_insert` — active_run_count == 0
8. `cargo test -p vb_runtime admission_rejection_no_counter_increment` — runs_submitted == 0
9. (Optional Miri) `MIRIENV='-Zmiri-strict-provenance=y' cargo miri test -p vb_runtime admission_rejection_does_not_insert_run_state_strict`

---

## Test Code to Add (provided by proof-writer-report.md)

### Unit tests (admission.rs test section)

```rust
#[cfg(test)]
mod admission_rejection_tests {
    use super::*;

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
        let store = NeverPresentStore;
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let run_id = RunId::new(1);
        let caps = CapabilitySet::empty();
        let policy = RuntimePolicy::Strict;
        let result = admit_artifact_run(&store, policy, run_id, digest, caps);
        assert!(matches!(result, Err(AdmissionError::ArtifactNotFound { .. })));
    }

    #[test]
    fn admit_run_journaled_without_artifact_rejected() {
        let store = NeverPresentStore;
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let run_id = RunId::new(2);
        let caps = CapabilitySet::empty();
        let policy = RuntimePolicy::Journaled;
        let result = admit_artifact_run(&store, policy, run_id, digest, caps);
        assert!(matches!(result, Err(AdmissionError::ArtifactNotFound { .. })));
    }

    #[test]
    fn admit_run_relaxed_skips_validation() {
        let store = NeverPresentStore;
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let run_id = RunId::new(3);
        let caps = CapabilitySet::empty();
        let policy = RuntimePolicy::Relaxed;
        let result = admit_artifact_run(&store, policy, run_id, digest, caps);
        assert!(result.is_ok());
    }
}
```

---

*Generated by proof-reviewer for vb-qi37.4.2*
