# Verification Layers: vb-core-atomic-admission

## Boundary

- Verus-owned kernel: pure accepted-run input coherence, artifact envelope discriminator, sequence binding, index derivation, typed error classification.
- TLA+ temporal model: all-or-none admission, before-ack ordering, failure injection, restart/readback.
- Theorem projection: waived unless proof review discovers a Verus gap.
- Runtime shell: Fjall batch commit, CLI/run/submit plumbing, storage-backed runtime admission, restart behavior.
- External systems excluded from formal proof: OS, Fjall internal correctness beyond documented atomic batch semantics, wall-clock time.

## Layer Assignment

- PRE-001 -> Verus + proptest + integration tests.
- PRE-002 -> Verus + Kani/proptest digest/proof mismatch exploration.
- PRE-003 -> integration/failure-injection with strict Fjall batch; no Verus because it is I/O capability.
- PRE-004 -> Verus + proptest for deterministic index derivation.
- PRE-005 -> Verus + fuzz/proptest for raw/legacy/malformed strict rejection.
- POST-001 -> TLA+ + integration restart/readback.
- POST-002 -> TLA+ + BDD/CLI before-ack failure injection.
- POST-003 -> Verus + integration readback asserting real `RunAccepted.seq` equality.
- POST-004 -> integration restart/readback + coverage.
- POST-005 -> TLA+ + failure-injection + mutation.
- POST-006 -> Verus + fuzz/proptest + strict negative fixtures.
- INV-001 -> TLA+ + failure-injection.
- INV-002 -> TLA+ + strict acknowledgement tests.
- INV-003 -> Verus + Kani/proptest.
- INV-004 -> Verus + fuzz/proptest.
- INV-005 -> TLA+ + Verus + integration readback.
- INV-006 -> Verus + mutation + static scan for dropped Results/log-and-continue.
- INV-007 -> TLA+ + restart/readback integration.
- `AdmissionError::InvalidAcceptedArtifact` -> `ERR-INVALID-015` via `moon ci` scenario `given_invalid_accepted_artifact_when_strict_admission_runs_then_invalid_accepted_artifact_error` + Verus taxonomy `VERUS-ERR-006`.
- `AdmissionError::InconsistentAdmissionInput` -> `ERR-INCONSISTENT-016` via `moon ci` scenario `given_inconsistent_admission_input_when_strict_admission_runs_then_inconsistent_admission_input_error` + Verus taxonomy `VERUS-ERR-006`.
- `AdmissionError::BatchStageFailed` -> `ERR-STAGE-017` via `moon ci` scenario `given_batch_stage_failure_when_strict_admission_runs_then_batch_stage_failed_error_without_partial_visibility` + Verus taxonomy `VERUS-ERR-006`.
- `AdmissionError::BatchCommitFailed` -> `ERR-COMMIT-018` via `moon ci` scenario `given_batch_commit_failure_when_strict_admission_runs_then_batch_commit_failed_error_and_no_ack` + Verus taxonomy `VERUS-ERR-006`.
- `AdmissionError::PartialVisibilityDetected` -> `ERR-PARTIAL-019` via `moon ci` scenario `given_partial_visibility_when_readback_runs_then_partial_visibility_detected_error` + Verus taxonomy `VERUS-ERR-006`.
- `AdmissionError::SequenceBindingFailed` -> `ERR-SEQUENCE-020` via `moon ci` scenario `given_sequence_binding_failure_when_strict_admission_runs_then_sequence_binding_failed_error` + Verus taxonomy `VERUS-ERR-006`.
- `AdmissionError::StrictRawWorkflowPartsRejected` -> `ERR-STRICT-RAW-021` via `moon ci` scenario `given_raw_workflow_parts_when_strict_admission_runs_then_strict_raw_workflow_parts_rejected_error` + Verus taxonomy `VERUS-ERR-006`.
- `AdmissionError::IndexDerivationFailed` -> `ERR-INDEX-022` via `moon ci` scenario `given_index_derivation_failure_when_strict_admission_runs_then_index_derivation_failed_error` + Verus taxonomy `VERUS-ERR-006`.

## Verus Scope

- Planned target: `verification/verus/accepted_run_atomic_admission.rs` or the downstream State 4 equivalent.
- Runtime targets to abstract: `vb_storage::{AcceptedArtifact, VerificationProof, WorkflowSourceRecord, CompiledIrRecord, RunHeaderRecord, JournalEvent::RunAccepted}` and accepted-run index keys.
- Spec/proof functions:
  - `spec_valid_commit_input`
  - `spec_artifact_matches_header_and_source`
  - `spec_bind_accepted_at_seq`
  - `proof_sequence_binding_preserves_truth`
  - `spec_strict_payload_is_accepted_artifact`
  - `proof_index_derivation_points_to_committed_run`
  - `proof_error_taxonomy_exhaustive`
- Trusted boundary: validated conversion from runtime records into the pure Verus model; Postcard/Fjall bytes and filesystem effects are shell exclusions.
- Shell exclusions: Fjall I/O, serialization implementation internals, CLI output formatting, runtime scheduler/allocation, wall-clock time.
- Evidence command after State 4 proof file exists: `verus verification/verus/accepted_run_atomic_admission.rs`.

## TLA+ Scope

- Module/model path: `verification/tla/AtomicAcceptedRunAdmission.tla`.
- Config path: `verification/tla/AtomicAcceptedRunAdmission.cfg`.
- Variables: `runs`, `source`, `artifact`, `header`, `acceptedEvent`, `indexes`, `staged`, `committed`, `acked`, `runtimeAllocated`, `failures`, `restarted`, `readback`.
- Actions: `Init`, `ValidateInput`, `StageSource`, `StageArtifact`, `StageHeader`, `StageAcceptedEvent`, `StageIndexes`, `CommitBatch`, `FailBeforeCommit`, `FailCommit`, `Acknowledge`, `AllocateRuntime`, `Restart`, `Readback`, `RejectInvalidArtifact`.
- Safety invariants: `NoAckBeforeCommit`, `AllRecordsOrNoAcceptedRun`, `NoRuntimeAllocationBeforeCommit`, `IndexesOnlyCommitted`, `NoPartialAfterFailure`, `ReadbackOnlyCommitted`.
- Temporal properties: `EventuallyAckOrFail`, `EventuallyReadableAfterCommit`.
- Fairness/deadlock stance: weak fairness for commit/ack/readback when enabled; deadlock freedom required.
- Refinement boundary: public storage commit and CLI/runtime acknowledgement events refine model actions.
- Evidence command: `tlc -config verification/tla/AtomicAcceptedRunAdmission.cfg verification/tla/AtomicAcceptedRunAdmission.tla`.

## Second-Ring Evidence

- Kani/proptest: bounded exploration for digest mismatch, sequence binding, index derivation, and error classification after implementation targets exist.
- Fuzz/Bolero: malformed/legacy/raw compiled IR payload rejection for strict path if artifact envelope codec changes.
- Miri/cargo-careful: targeted codec/readback checks if unsafe, raw bytes, or new serialization paths are touched.
- Mutation: kill mutants that drop commit errors, acknowledge before commit, accept raw `WorkflowParts`, or ignore missing indexes.
- Coverage: admission failure-injection and restart/readback branches covered by bead-local tests.
- Static scan: no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/casts/arithmetic, or ignored `Result` in touched source.
- Performance: no speed claim; run only non-regression evidence if strict batch adds measurable overhead to existing submit/run benchmark.
- API compatibility: required if public signatures in `vb_storage`, `vb_runtime`, or CLI contract change.
- Release provenance: inherited by release-critical `moon ci`/release gates; no new dependency expected.

## Waivers

- Lean/Aeneas/Hax waived per `lean-contract.md` because Verus/TLA+ split is sufficient at State 3.
- Loom/shuttle waived for this bead unless downstream implementation introduces concurrent in-memory admission shared state; current mapped risk is durable atomic batch and before-ack ordering, covered by TLA+ plus failure injection.
- Kani sequence-binding obligation `KANI-PROP-007` is an approved-planning waiver unless/until State 8 creates an exact harness: owner `State 8 implementation/proof repair`; reason no exact harness exists in current artifacts and p3-contract-repair2 cannot write proof/test/source files; limitation no bounded production-code model checking yet; expiry before State 12 formal verification or release/landing; compensating evidence `VERUS-SEQ-003`, `INTEG-FAIL-012`, and `STATIC-SCAN-011`.
- Fuzz strict-artifact obligation `FUZZ-ART-008` is an approved-planning waiver unless/until State 8 creates an exact target: owner `State 8 implementation/proof repair`; reason no exact fuzz target exists in current artifacts and p3-contract-repair2 cannot write proof/test/source files; limitation no byte-level malformed-payload fuzz evidence yet; expiry before State 12 formal verification or release/landing; compensating evidence `VERUS-ART-004`, `ERR-STRICT-RAW-021`, and `INTEG-FAIL-012`.
