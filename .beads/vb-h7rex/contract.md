# Bead Contract: vb-h7rex - Bind admitted policy identity to run admission

## Problem

`RunAdmission` currently carries `artifact_digest` and `RuntimePolicy` (enum) but not the actual policy digest that governed admission. On crash/recovery, the run has no durable binding to the specific policy identity from the accepted artifact.

## Design

Add `RunPolicyPin` enum (parallel to `RunInputBinding` pattern):
- `Unverified`: legacy/relaxed/no-accepted-artifact admission records
- `Pinned(WorkflowDigest)`: strict/journaled admission with actual `AcceptedArtifact.policy_digest`

Add `policy_pin: RunPolicyPin` field to:
- Runtime `RunAdmission` struct
- Storage `JournalEvent::RunAdmission` variant

New record tag: `RunAdmissionPinned = 39` (after input-bound 36, retry 37, policy decision 38 reserved).

## API Changes

### vb_storage
```rust
pub enum RunPolicyPin {
    Unverified,
    Pinned(WorkflowDigest),
}
```

### vb_runtime admission
```rust
impl RunAdmission {
    pub fn policy_pin(&self) -> RunPolicyPin;
}
```

## Production Seams

1. `admit_artifact_run_with_certificate_floor` → passes `artifact.policy_digest` as `RunPolicyPin::Pinned(digest)` for Strict/Journaled; `RunPolicyPin::Unverified` for Relaxed
2. `RuntimeJournalEvent::RunAdmission` → serializes with policy_pin
3. `recovery/full/admission.rs::validate_recovered_admission_evidence` → validates `admission.policy_pin == artifact.policy_digest` (or explicitly `Unverified` for legacy)
4. Wire serialization/deserialization → new tag 39 for policy-pinned records

## Migration Behavior

- Old records (tags 24, 36) decode with `policy_pin: RunPolicyPin::Unverified`
- New strict/journaled admission writes tag 39 with actual policy digest
- Relaxed mode writes tag 24/36 with `Unverified`
- Legacy decode path unchanged (explicit version rejection)

## Acceptance

1. Strict admission stores actual `policy_digest` from accepted artifact (≠ artifact digest)
2. Recovery reads `policy_pin` and validates it matches artifact
3. Conflicting policy pin rejected on recovery
4. Legacy/relaxed records decode as `Unverified`
5. Focused tests with public Runtime/Fjall admission + recovery path
