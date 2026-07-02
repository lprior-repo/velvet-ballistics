# Contract Specification: vb-core-atomic-admission

## Context

- Bead: `vb-core-atomic-admission`
- Feature: persist strict accepted-run creation as a single durable Fjall admission boundary before acknowledgement.
- Source inputs: State 2 `codebase-map.md`, State 2 `delivery-scope.jsonl`, and `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-core-atomic-admission --json`.
- Touched crates: `vb_storage`, `vb_runtime`, `velvet_ballistics`.
- Critical acceptance: source, accepted artifact, run header, `RunAccepted`, and required indexes are all durable together; injected persistence failure leaves no partially accepted run; `accepted_at_seq` is the real committed journal sequence.

## Domain Terms

- Accepted run: a run whose workflow source, accepted artifact envelope, run header, acceptance event, and lookup indexes have been committed as one admission unit.
- Accepted artifact: stable `AcceptedArtifact` envelope stored in `CompiledIrRecord`, not raw `WorkflowParts`, for strict paths.
- Admission batch: one strict `JournalWriteBatch`/Fjall commit containing all required records.
- Acknowledgement: CLI/runtime externally visible success, returned run id, in-memory runnable state, or observable accepted status.
- Required indexes: status, workflow, and any action/run indexes required for restart/readback of the accepted run.

## Assumptions

- Fjall `OwnedWriteBatch` commit is the durable atomic primitive for storage records staged into one batch.
- State 4 proof authors may add TLA+/Verus artifacts under the isolated workspace before production implementation.
- `vb-core-accepted-artifact-format` and `vb-core-proof-15-gate` define final envelope and proof schema; this contract requires compatibility and fail-closed behavior while those blockers are open.
- No Cargo dependency change is required by this bead.

## Open Questions

- Final exact 15-gate proof schema is owned by `vb-core-proof-15-gate`; this bead must consume it without downgrading gate count.
- Final accepted artifact byte layout is owned by `vb-core-accepted-artifact-format`; this bead must reject legacy/raw payloads on strict admission paths.

## Preconditions

- PRE-001: Caller provides a validated workflow source record, accepted artifact envelope, run header, run id, workflow id/digest, runtime policy, and capability set for the same logical run.
- PRE-002: Accepted artifact digest, workflow digest, proof envelope, and run header references are mutually consistent before staging any acknowledgement-visible effect.
- PRE-003: Strict admission path has access to a storage journal capable of one strict durable batch commit.
- PRE-004: Required index keys can be derived deterministically from the source, artifact, header, and acceptance event without reading partially committed state.
- PRE-005: Raw `WorkflowParts`, unverified compiled data, stale proof gates, missing proof gates, digest mismatch, or legacy artifact formats are not valid strict accepted artifacts.

## Postconditions

- POST-001: On success, exactly one accepted-run durability boundary has committed workflow source, `CompiledIrRecord` containing `AcceptedArtifact`, run header, `JournalEvent::RunAccepted`, and required indexes.
- POST-002: On success, acknowledgement occurs only after strict batch commit and durable persistence complete successfully.
- POST-003: On success, `AcceptedArtifact.accepted_at_seq` equals the real committed sequence of the `RunAccepted` journal event for the same run.
- POST-004: On success, storage-backed readback after restart can resolve the run by run id, workflow id/digest, status index, accepted artifact digest, and acceptance event.
- POST-005: On failure before/during commit, no subset of source/artifact/header/event/index records is externally visible as an accepted run, and no acknowledgement or runnable runtime state is produced.
- POST-006: On strict paths, `CompiledIrRecord` never stores raw `WorkflowParts` as a substitute for `AcceptedArtifact`.

## Invariants

- INV-001: Atomic all-or-none visibility: for any strict accepted run, either all required record families are visible together or none form an accepted run.
- INV-002: Before-ack ordering: durable accepted-run commit precedes every external success acknowledgement and runtime allocation that can be observed as accepted.
- INV-003: Sequence truth: `accepted_at_seq` is non-sentinel and equals the committed `RunAccepted.seq` for the same run.
- INV-004: Artifact purity: strict admission and readback accept only `AcceptedArtifact` envelopes with matching digest/proof/capability metadata, never raw `WorkflowParts`.
- INV-005: Index consistency: status/workflow/action indexes point only to committed accepted runs and never to missing source/artifact/header/event records.
- INV-006: Fail-closed errors: every storage, journal, codec, digest, schema, or proof mismatch returns a typed error carrying operation, run id, record kind or boundary, and causal class.
- INV-007: Recovery determinism: restart/readback reconstructs the same accepted-run decision from durable records and does not infer acceptance from in-memory state or loose artifacts.

## Error Taxonomy

- AdmissionError::InvalidAcceptedArtifact: strict input is raw, legacy, malformed, stale, digest-mismatched, or lacks required proof gates.
- AdmissionError::InconsistentAdmissionInput: source, artifact, header, policy, capabilities, run id, workflow id, or digest do not agree.
- AdmissionError::BatchStageFailed: required record or index cannot be encoded or staged before commit.
- AdmissionError::BatchCommitFailed: strict Fjall batch commit or sync persistence fails.
- AdmissionError::PartialVisibilityDetected: readback detects an impossible subset of records and refuses acknowledgement/recovery.
- AdmissionError::SequenceBindingFailed: real `RunAccepted` sequence cannot be allocated/bound to the artifact before commit.
- AdmissionError::StrictRawWorkflowPartsRejected: strict path attempts to persist or admit raw `WorkflowParts` in `CompiledIrRecord`.
- AdmissionError::IndexDerivationFailed: required index key/value cannot be deterministically derived.

### Error Variant Traceability Requirements

- `AdmissionError::InvalidAcceptedArtifact` MUST have a downstream executable scenario named `given_invalid_accepted_artifact_when_strict_admission_runs_then_invalid_accepted_artifact_error` and proof obligation `ERR-INVALID-015`.
- `AdmissionError::InconsistentAdmissionInput` MUST have a downstream executable scenario named `given_inconsistent_admission_input_when_strict_admission_runs_then_inconsistent_admission_input_error` and proof obligation `ERR-INCONSISTENT-016`.
- `AdmissionError::BatchStageFailed` MUST have a downstream executable scenario named `given_batch_stage_failure_when_strict_admission_runs_then_batch_stage_failed_error_without_partial_visibility` and proof obligation `ERR-STAGE-017`.
- `AdmissionError::BatchCommitFailed` MUST have a downstream executable scenario named `given_batch_commit_failure_when_strict_admission_runs_then_batch_commit_failed_error_and_no_ack` and proof obligation `ERR-COMMIT-018`.
- `AdmissionError::PartialVisibilityDetected` MUST have a downstream executable scenario named `given_partial_visibility_when_readback_runs_then_partial_visibility_detected_error` and proof obligation `ERR-PARTIAL-019`.
- `AdmissionError::SequenceBindingFailed` MUST have a downstream executable scenario named `given_sequence_binding_failure_when_strict_admission_runs_then_sequence_binding_failed_error` and proof obligation `ERR-SEQUENCE-020`.
- `AdmissionError::StrictRawWorkflowPartsRejected` MUST have a downstream executable scenario named `given_raw_workflow_parts_when_strict_admission_runs_then_strict_raw_workflow_parts_rejected_error` and proof obligation `ERR-STRICT-RAW-021`.
- `AdmissionError::IndexDerivationFailed` MUST have a downstream executable scenario named `given_index_derivation_failure_when_strict_admission_runs_then_index_derivation_failed_error` and proof obligation `ERR-INDEX-022`.

## Contract Signatures

- `fn persist_accepted_run_atomic(input: AcceptedRunCommitInput) -> Result<AcceptedRunCommitReceipt, AdmissionError>`
- `fn build_accepted_run_batch(input: AcceptedRunCommitInput) -> Result<JournalWriteBatch, AdmissionError>`
- `fn bind_accepted_at_seq(artifact: AcceptedArtifact, seq: EventSeq) -> Result<AcceptedArtifact, AdmissionError>`
- `fn readback_accepted_run(run_id: RunId) -> Result<AcceptedRunReadback, AdmissionError>`
- `fn reject_strict_raw_workflow_parts(record: CompiledIrRecord) -> Result<AcceptedArtifact, AdmissionError>`

## Verus-Owned Clauses

- PRE-001, PRE-002, PRE-004, PRE-005: validate coherent pure input model before staging.
- POST-003, INV-003: prove sequence binding relation between artifact and `RunAccepted` event in the pure model.
- INV-004: prove strict artifact envelope discriminator rejects raw/legacy variants.
- INV-005: prove index derivation consistency from accepted-run model.
- INV-006: prove pure error classification is exhaustive for model-level failure causes.

## TLA+-Owned Clauses

- POST-001, POST-002, POST-005, INV-001, INV-002, INV-005, INV-007: model state-over-time admission, failure injection, commit, ack, and restart/readback behavior.

## Theorem-Owned Clauses

- None required at State 3. Verus owns the Rust-local pure model; TLA+ owns temporal atomicity and before-ack behavior.

## Non-goals

- No production code, tests, proof code, or source checkout writes in State 3.
- No UI, generated Rust/codegen, HTTP, JSON, or YAML runtime-core behavior changes are specified by this contract.
- No speed/vectorization claim is made; performance evidence is limited to non-regression of the strict admission path.
