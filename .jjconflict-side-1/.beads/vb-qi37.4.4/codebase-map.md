bead_id: vb-qi37.4.4
bead_title: runtime: Add admission durability errors
phase: State 2 - codebase map
updated_at: 2026-05-11T00:00:00Z

# Codebase Map

- `crates/vb_runtime/src/lib.rs`: `RuntimeError` variants, `Display`, `Error::source`, equality, diagnostic codes, runtime-code mapping.
- `crates/vb_runtime/src/shard/lifecycle.rs`: maps `AdmissionError` to `RuntimeError` and propagates journal append errors from header/admission persistence.
- `crates/vb_runtime/src/journal.rs`: converts runtime journal events into storage journal append operations.
- `crates/velvet_ballastics/tests/admission_evidence_integration.rs`: direct API/integration scenarios should assert typed errors are not lossy.

State 2 command evidence: grep/read found current admission errors `AdmissionArtifactNotFound`, `AdmissionArtifactInvalid`, `AdmissionCapabilityDenied`, and generic `StorageJournalAppend` diagnostic handling.
