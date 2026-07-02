bead_id: vb-ssei
phase: 2
updated_at: 2026-05-18T21:50:13Z
attempt: 1-of-7

# Codebase map

Touched files:
- `crates/workspace_tests/tests/vb_ssei_verification_admission_acceptance.rs` — new executable Given/When/Then acceptance scenarios for strict verification/admission.
- `crates/workspace_tests/src/acceptance_catalog.rs` — maps catalog scenario `VB-BDD-CATALOG-008` to executable evidence target instead of deferred follow-up.
- `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs` — updates catalog target/deferred counts and expected target list.

Public APIs exercised:
- `vb_runtime::admission::admit_artifact_run`
- `vb_runtime::admission::AcceptedArtifactStore`
- `vb_runtime::admission::AdmissionError`
- `vb_storage::admission::{AcceptedArtifact, VerificationProof}`
- `vb_core::{Capability, CapabilitySet, ActionId, RunId, WorkflowDigest, RuntimePolicy}`

Risk tags: BDD acceptance, strict admission, capability fail-closed, digest mismatch, idempotency certificate, catalog traceability.
