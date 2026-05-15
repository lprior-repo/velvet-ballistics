# Contract Specification

Bead: `vb-qi37.4.2` - runtime: Enforce admission gate before run creation.

## Context

- Source artifacts read: `baseline-report.md`, `codebase-map.md`, `delivery-scope.jsonl`, and `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.4.2 --json`.
- Feature: strict runtime admission must accept only durable accepted-artifact envelopes and must reject raw, failed, stale, malformed, digest-mismatched, or under-verified artifacts before runtime state allocation.
- Domain terms:
  - Accepted artifact: postcard-encoded `AcceptedArtifact` with digest, gate evidence, durable flag, capability evidence, and schema/version evidence.
  - Raw artifact: `WorkflowParts`, YAML, JSON, malformed bytes, or any artifact lacking the accepted envelope.
  - Strict/journaled runtime: production admission path that must use storage-backed artifact loading, not dummy existence-only stores.
  - Admission boundary: point before `take_frame_for`, `self.runs.insert`, `drive_run`, `RunAccepted`, or any API/CLI/IPC success acknowledgement.
- Assumptions:
  - Canonical accepted-artifact gate count is `15` until the upstream contract explicitly changes it.
  - `vb-qi37.4.1` owns the envelope definition and is closed; this bead owns runtime enforcement of that envelope before run creation.
  - Dependent beads own atomic Fjall batch durability and production `StorageArtifactStore` wiring, but this bead must not permit a bypass that would make those dependents meaningless.
- Open questions:
  - Whether runtime diagnostics should introduce finer public variants or preserve existing variants with structured detail.
  - Whether inner IR digest validation and envelope digest validation must both be enforced at the same runtime boundary.

## Preconditions

- PRE-001: Strict or journaled run creation input MUST identify a persisted accepted-artifact envelope by digest; raw `WorkflowParts`, YAML, JSON, or opaque bytes are not admissible runtime inputs.
- PRE-002: The loaded envelope MUST decode as accepted-artifact v1 and MUST carry canonical `gate_count == 15` with all required gate proof flags accepted.
- PRE-003: The envelope digest MUST match the requested artifact digest and the persisted compiled-IR record digest; mismatch is a hard admission failure. State3 does not claim an executable Kani harness because `verification/kani/digest_admission_harness.rs` is absent; the contract requires integration/domain tests as compensating evidence until a later proof-writing state creates a bounded harness.
- PRE-004: The envelope MUST be durable and non-stale according to its admission metadata; relaxed artifacts with `gate_count == 0`, `durable == false`, or stale certificate data are rejected.
- PRE-005: Capability grants in the envelope MUST exactly cover the workflow-required capability profile: no missing, excess, prefix, action-mismatched, or duplicate grants.
- PRE-006: Strict/journaled production constructors MUST use a storage-backed accepted-artifact loader or an equivalent verified source; `AlwaysPresentArtifactStore` is permitted only for relaxed/test-only contexts.

## Postconditions

- POST-001: Valid accepted artifacts proceed to run creation without runtime YAML or JSON parsing.
- POST-002: Raw, malformed, failed-gate, stale, non-durable, digest-mismatched, or capability-mismatched artifacts return typed admission diagnostics.
- POST-003: Any admission failure occurs before runtime state allocation: no frame is taken, no run is inserted, no runnable state exists, no `drive_run` occurs, and no `RunAccepted` is emitted.
- POST-004: Rejected diagnostics preserve the rejected digest and semantic cause, including malformed envelope, missing artifact, failed gate, stale certificate, digest mismatch, and capability denial.
- POST-005: Successful admission records the artifact digest, admission certificate/profile, and initial metadata needed by downstream header-persistence work.

## Invariants

- INV-001: There is exactly one canonical accepted-artifact gate-count contract shared by runtime and storage for strict admission.
- INV-002: Existence-only artifact checks cannot satisfy strict admission.
- INV-003: Admission is fail-closed: unknown schema version, missing field, decode failure, stale evidence, or unsupported proof status denies.
- INV-004: Strict/journaled admission never depends on runtime YAML/JSON parsing.
- INV-005: Denied admission cannot allocate or expose runnable state.
- INV-006: Capability checking is exact-cardinality and exact-name/action.
- INV-007: Diagnostics are typed and non-lossy enough for API/CLI/IPC callers to distinguish accepted-envelope failures from storage-not-found and capability denial.

## Error Taxonomy

- ERR-001 `AdmissionError::ArtifactNotFound` - requested digest has no persisted accepted artifact. Expected scenario: `given_missing_artifact_when_strict_run_created_then_artifact_not_found_before_allocation`; diagnostic preserves requested digest and performs no allocation.
- ERR-002 `AdmissionError::ArtifactEnvelopeDecodeFailed` - artifact bytes are raw, malformed, truncated postcard, YAML, JSON, or not accepted-envelope v1. Expected scenario: `given_raw_or_malformed_bytes_when_strict_run_created_then_decode_failed_with_rejected_digest`; diagnostic preserves rejected digest and decode/malformed cause.
- ERR-003 `AdmissionError::ArtifactEnvelopeInvalid` - envelope decodes but lacks required fields, durable marker, schema support, or accepted proof flags. Expected scenario: `given_decoded_envelope_missing_required_acceptance_fields_then_invalid_envelope_denies`; diagnostic names invalid envelope cause and performs no allocation.
- ERR-004 `AdmissionError::ArtifactGateMismatch` - gate count or gate status does not satisfy canonical strict admission. Expected scenario: `given_gate_count_zero_two_or_failed_status_when_strict_run_created_then_gate_mismatch_denies`; diagnostic records observed gate evidence and required canonical gate.
- ERR-005 `AdmissionError::ArtifactDigestMismatch` - requested digest, persisted record digest, or envelope digest disagree. Expected scenario: `given_digest_mismatch_when_strict_run_created_then_digest_mismatch_denies`; diagnostic records requested and observed digest identities without collapsing to invalid envelope.
- ERR-006 `AdmissionError::ArtifactStale` - certificate/evidence is stale for the required runtime profile. Expected scenario: `given_stale_artifact_when_strict_run_created_then_stale_certificate_denies`; diagnostic preserves staleness cause and rejected digest.
- ERR-007 `AdmissionError::CapabilityDenied` - required capability profile is not exactly granted. Expected scenario: `given_missing_excess_prefix_or_action_mismatched_capability_then_capability_denied`; diagnostic preserves required/granted mismatch class.
- ERR-008 `RuntimeError::AdmissionArtifactNotFound`, `RuntimeError::AdmissionArtifactInvalid`, and `RuntimeError::AdmissionCapabilityDenied` mappings MUST preserve the underlying `AdmissionError` category, rejected digest when present, and semantic cause. Expected scenario: `given_cli_ipc_runtime_error_mapping_when_serialized_then_error_category_digest_and_cause_are_preserved`.

## Contract Signatures

- `AcceptedArtifactStore::load_accepted_artifact(digest: ArtifactDigest) -> Result<AcceptedArtifact, AdmissionError>`
- `admit_artifact_run(store: &dyn AcceptedArtifactStore, digest: ArtifactDigest, required: CapabilityProfile, policy: AdmissionPolicy) -> Result<AdmissionRecord, AdmissionError>`
- `build_admission(run_id: RunId, digest: ArtifactDigest, required: CapabilityProfile) -> Result<AdmissionRecord, RuntimeError>`
- `Runtime::new_with_journal_and_artifact_store(journal: Journal, store: StorageArtifactStore) -> Result<Runtime, RuntimeError>`

## Verus-Owned Clauses

- PRE-005, INV-006: exact capability name/action and exact cardinality, using existing `verification/verus/capability_artifact_model.rs`.
- PRE-002, PRE-004, INV-001, INV-003: accepted-envelope gate/status/durable pure predicate uses `verification/verus/accepted_envelope_model.rs`, verified by `verus verification/verus/accepted_envelope_model.rs` in State5 evidence.

## TLA+-Owned Clauses

- POST-003, INV-005: denied admission leaves no run allocation or journaled accepted state.
- PRE-002, INV-001: gate mismatch denies.
- PRE-006, INV-002: legacy/dummy bypass cannot admit protected strict submissions.
- PRE-005, INV-006: capability cardinality mismatch denies before allocation.

## Theorem-Owned Clauses

- None at State 3. Verus is sufficient for Rust-local predicates. Lean/Aeneas/Hax is a non-goal unless proof-review identifies a tiny algebraic kernel that Verus cannot express.

## Non-goals

- Implementing production code, proof code, or tests in State 3.
- Claiming performance improvement; this bead is correctness/admission only.
- Owning atomic Fjall batch persistence after successful admission; that is dependent bead `vb-core-atomic-admission`.
- Claiming State3 executable Kani/fuzz/cargo-mutants/CI passes where no harness, target, diagnostic tests, or CI run exists. The contract ledger keeps these as `status: planned`; any WAIVED/DEFERRED result belongs only in downstream execution evidence artifacts with owner, reason, expiry, limitation, and compensating evidence.
