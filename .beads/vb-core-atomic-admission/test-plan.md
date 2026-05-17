# Test Plan: vb-core-atomic-admission

## Summary

- Behaviors identified: 18 contract behaviors plus 8 typed error behaviors.
- Trophy allocation: 10 unit / 14 integration / 2 E2E / 7 static-formal gates. Integration is deliberately widest because the bead's risk is cross-crate durable Fjall admission, restart/readback, CLI/runtime/storage acknowledgement ordering, and failure injection.
- Proptest invariants: 9.
- Fuzz targets: 4.
- Kani harnesses: 3.
- Mutation threshold: >=90% killed mutants overall, and 100% kill or reviewed-equivalent outcome for critical atomicity/error-propagation mutants listed below.

## 0. Inputs and Scope Guard

- Approved inputs consumed:
  - `.beads/vb-core-atomic-admission/proof-review.md` with `STATUS: APPROVED`.
  - `.beads/vb-core-atomic-admission/contract-verification-review.md` with `STATUS: APPROVED`.
  - `.beads/vb-core-atomic-admission/contract.md`.
  - `.beads/vb-core-atomic-admission/traceability-matrix.jsonl`.
  - `.beads/vb-core-atomic-admission/proof-obligations.jsonl`.
  - `.beads/vb-core-atomic-admission/proof-obligations.planned.jsonl`.
  - `.beads/vb-core-atomic-admission/delivery-scope.jsonl`.
- Public surfaces under test from `delivery-scope.jsonl`: `vb_storage::FjallJournal::batch`, `vb_storage::JournalWriteBatch`, `vb_storage::submit_artifact`, `vb_storage::submit_artifact_with_contracts`, `vb_storage::AcceptedArtifact`, `vb_storage::VerificationProof`, `vb_storage::WorkflowSourceRecord`, `vb_storage::CompiledIrRecord`, `vb_storage::RunHeaderRecord`, `vb_storage::JournalEvent::RunAccepted`, `vb_runtime::admission::AcceptedArtifactStore`, `vb_runtime::admission::StorageArtifactStore`, `vb_runtime::admission::admit_artifact_run`, and `velvet_ballastics` CLI submit/run storage paths.
- No production code, test code, proof/model code, dependency files, or CI files are to be edited in State 7.
- Test-writer must assert exact returned values, exact persisted records, exact durable absence, and exact error variants with operation/run/record-kind/boundary/causal-class context. Bare `is_ok()` / `is_err()` assertions are rejected.

## 1. Behavior Inventory

| ID | Behavior | Contract clauses | Proof/trace IDs | Primary layer |
|---|---|---|---|---|
| B01 | Strict admission stages a complete coherent input model when caller supplies source, accepted artifact, header, run id, workflow id/digest, policy, and capabilities for the same run | PRE-001 | VERUS-PRE-001 | unit + integration |
| B02 | Strict admission rejects mismatched source/artifact/header/policy/capability/run/workflow/digest before any acknowledgement-visible effect | PRE-002 | VERUS-PRE-002, ERR-INCONSISTENT-016 | unit + integration |
| B03 | Strict admission fails closed when strict storage or journal batch commit capability is unavailable | PRE-003, POST-002 | TLA-ATOM-001, ERR-COMMIT-018, INTEG-FAIL-012 | integration + E2E |
| B04 | Index derivation deterministically binds status, workflow, and action indexes to the accepted run identity | PRE-004, INV-005 | VERUS-IDX-005, ERR-INDEX-022 | unit + integration |
| B05 | Strict admission rejects raw `WorkflowParts`, legacy payloads, malformed payloads, stale proof gates, missing gates, and digest mismatches | PRE-005, POST-006, INV-004 | VERUS-ART-004, FUZZ-ART-008, ERR-INVALID-015, ERR-STRICT-RAW-021 | unit + fuzz + integration |
| B06 | Successful strict commit makes source, `CompiledIrRecord(AcceptedArtifact)`, run header, `RunAccepted`, and required indexes visible together | POST-001, INV-001 | TLA-ATOM-001, INTEG-FAIL-012, API-COMPAT-013 | integration |
| B07 | Successful acknowledgement is emitted only after strict batch commit and persistence complete | POST-002, INV-002 | TLA-ATOM-001, ERR-COMMIT-018 | integration + E2E |
| B08 | Successful artifact `accepted_at_seq` equals the real committed `RunAccepted.seq` for the same run and is not sentinel | POST-003, INV-003 | VERUS-SEQ-003, KANI-PROP-007, ERR-SEQUENCE-020 | unit + integration + Kani |
| B09 | Restart/readback resolves the accepted run by run id, workflow id/digest, status index, accepted artifact digest, and acceptance event | POST-004, INV-007 | TLA-ATOM-001, MIRI-CODEC-009 | integration |
| B10 | Failure before or during commit leaves no externally visible accepted subset and no runtime runnable state | POST-005, INV-001 | TLA-ATOM-001, INTEG-FAIL-012, MUT-ERR-010 | integration + mutation |
| B11 | `CompiledIrRecord` on strict paths stores only an `AcceptedArtifact` envelope, never raw `WorkflowParts` | POST-006, INV-004 | VERUS-ART-004, FUZZ-ART-008, ERR-STRICT-RAW-021 | unit + fuzz + integration |
| B12 | Visible indexes point only to committed accepted runs with all required record families present | INV-005 | TLA-ATOM-001, VERUS-IDX-005, ERR-INDEX-022 | integration |
| B13 | Every storage, journal, codec, digest, schema, or proof mismatch returns a typed fail-closed error carrying context | INV-006 | VERUS-ERR-006, ERR-* | unit + integration + mutation |
| B14 | Restart/readback reconstructs acceptance only from durable records, never from in-memory state or loose artifacts | INV-007 | TLA-ATOM-001, ERR-PARTIAL-019, INTEG-FAIL-012 | integration |
| B15 | Public storage/runtime/CLI API compatibility is preserved or explicitly reviewed | POST-001 | API-COMPAT-013 | static |
| B16 | Touched production source remains free of forbidden constructs and ignored persistence `Result`s | INV-006 | STATIC-SCAN-011 | static |
| B17 | Strict artifact codec/readback raw-byte paths have no Miri-detected UB when touched | POST-004 | MIRI-CODEC-009 | static-formal |
| B18 | No performance claim is introduced without measured evidence | Non-goals | PERF-NONGOAL-014 | static-review |

Typed error behaviors required by contract lines 69-78:

| Error ID | Required behavior/scenario | Contract clause | Proof ID |
|---|---|---|---|
| E01 | `given_invalid_accepted_artifact_when_strict_admission_runs_then_invalid_accepted_artifact_error` | `AdmissionError::InvalidAcceptedArtifact` | ERR-INVALID-015 |
| E02 | `given_inconsistent_admission_input_when_strict_admission_runs_then_inconsistent_admission_input_error` | `AdmissionError::InconsistentAdmissionInput` | ERR-INCONSISTENT-016 |
| E03 | `given_batch_stage_failure_when_strict_admission_runs_then_batch_stage_failed_error_without_partial_visibility` | `AdmissionError::BatchStageFailed` | ERR-STAGE-017 |
| E04 | `given_batch_commit_failure_when_strict_admission_runs_then_batch_commit_failed_error_and_no_ack` | `AdmissionError::BatchCommitFailed` | ERR-COMMIT-018 |
| E05 | `given_partial_visibility_when_readback_runs_then_partial_visibility_detected_error` | `AdmissionError::PartialVisibilityDetected` | ERR-PARTIAL-019 |
| E06 | `given_sequence_binding_failure_when_strict_admission_runs_then_sequence_binding_failed_error` | `AdmissionError::SequenceBindingFailed` | ERR-SEQUENCE-020 |
| E07 | `given_raw_workflow_parts_when_strict_admission_runs_then_strict_raw_workflow_parts_rejected_error` | `AdmissionError::StrictRawWorkflowPartsRejected` | ERR-STRICT-RAW-021 |
| E08 | `given_index_derivation_failure_when_strict_admission_runs_then_index_derivation_failed_error` | `AdmissionError::IndexDerivationFailed` | ERR-INDEX-022 |

## 2. Trophy Allocation

| Layer | Count | Planned coverage | Rationale |
|---|---:|---|---|
| Static/formal base | 7 | `moon ci`, source-governance scan, `cargo semver-checks --workspace`, targeted Miri, approved TLC/Verus reruns, no-performance-claim review | Base gate catches forbidden source constructs, API breakage, UB in touched codec/readback paths, and proof-regression drift. |
| Unit/calc | 10 | input coherence, strict payload discriminator, sequence binding, index key derivation, typed error classification, batch builder model, readback partial-subset classifier | Pure/domain logic must be exhaustively checked without Fjall I/O where possible. |
| Integration | 14 | real `FjallJournal` temp stores, real `JournalWriteBatch`, real restart/reopen, runtime `StorageArtifactStore`, CLI/storage path with local filesystem | Widest layer because atomicity and restart/readback are boundary properties; use real storage, not mocks, except controlled failpoints/fakes for deterministic injected errors. |
| E2E/acceptance | 2 | CLI submit/run success after durable commit; CLI submit/run commit failure no acknowledgement/no runnable state | Few but essential black-box acknowledgement ordering checks. |

Deviation from nominal ratio: static/formal is higher than 5% because the bead has approved proof obligations and zero-tolerance Rust governance. E2E remains narrow; integration is still the widest executable layer.

## 3. BDD Scenarios

### B01: complete coherent input is staged as one admission unit

`fn given_complete_valid_accepted_run_input_when_committing_then_all_required_families_are_staged()`

- Given: a valid `AcceptedRunCommitInput` with mutually consistent workflow source digest, accepted artifact digest/proof/capabilities, run header, run id, workflow id/digest, runtime policy, and capability set.
- When: `build_accepted_run_batch` is requested.
- Then: the staged batch contains exactly workflow source, `CompiledIrRecord(AcceptedArtifact)`, run header, one `JournalEvent::RunAccepted`, status index, workflow index, and required action indexes for the same run.
- And: no acknowledgement, runtime allocation, or committed durable state is produced by batch construction alone.
- Maps: PRE-001, VERUS-PRE-001.

### B02/E02: incoherent input fails before visible effects

`fn given_inconsistent_admission_input_when_strict_admission_runs_then_inconsistent_admission_input_error()`

- Given: each mismatch class in turn: source digest mismatch, artifact digest mismatch, proof digest mismatch, header compiled digest mismatch, run id mismatch, workflow id mismatch, policy mismatch, capability mismatch.
- When: strict admission runs.
- Then: it returns `AdmissionError::InconsistentAdmissionInput` with operation `strict_admission`, the exact run id when derivable, boundary `precommit_validation`, and causal class naming the mismatch.
- And: readback by run id, workflow id/digest, status index, artifact digest, and event stream returns no accepted run.
- Maps: PRE-002, INV-006, VERUS-PRE-002, ERR-INCONSISTENT-016.

### B03/E04: storage unavailable or commit failure is fail-closed

`fn given_batch_commit_failure_when_strict_admission_runs_then_batch_commit_failed_error_and_no_ack()`

- Given: valid admission input and a deterministic journal/failpoint that fails strict batch commit or sync persistence.
- When: CLI/runtime strict admission is attempted.
- Then: it returns `AdmissionError::BatchCommitFailed` with operation, run id, boundary `batch_commit`, and storage causal class.
- And: no success acknowledgement, no returned accepted run id, no runnable runtime state, and no accepted status are visible.
- And: restart/readback finds no accepted run by every required lookup path.
- Maps: PRE-003, POST-002, POST-005, INV-001, INV-002, ERR-COMMIT-018, INTEG-FAIL-012.

### B04/E08: deterministic index derivation and failure

`fn given_valid_input_when_indexes_are_derived_then_index_keys_match_run_and_workflow_identity()`

- Given: valid source, artifact, header, run, workflow, status, and action metadata.
- When: required index keys are derived.
- Then: status index embeds the accepted status/timestamp/run id, workflow index embeds workflow id/run id, each action index embeds action/run/step, and all point to the same committed run.
- Maps: PRE-004, INV-005, VERUS-IDX-005.

`fn given_index_derivation_failure_when_strict_admission_runs_then_index_derivation_failed_error()`

- Given: an input class that makes a required status/workflow/action index key or value impossible to derive deterministically.
- When: strict admission runs.
- Then: it returns `AdmissionError::IndexDerivationFailed` with operation, run id, record kind `IndexUpdate`, and causal class.
- And: no index points to a missing source/artifact/header/event family.
- Maps: PRE-004, INV-005, ERR-INDEX-022.

### B05/E01/E07/B11: strict artifact envelope only

`fn given_invalid_accepted_artifact_when_strict_admission_runs_then_invalid_accepted_artifact_error()`

- Given: raw `WorkflowParts`, legacy compiled payload, malformed postcard bytes, stale gate count, missing gate, false proof flag, digest-mismatched envelope, or proof/capability metadata mismatch.
- When: strict admission runs.
- Then: it returns `AdmissionError::InvalidAcceptedArtifact` with operation, run id if known, record kind `CompiledIr`, and causal class for the exact invalidity.
- And: no source/header/event/index subset is visible as accepted.
- Maps: PRE-005, INV-004, INV-006, ERR-INVALID-015, FUZZ-ART-008.

`fn given_raw_workflow_parts_when_strict_admission_runs_then_strict_raw_workflow_parts_rejected_error()`

- Given: a `CompiledIrRecord` whose bytes decode as raw `WorkflowParts` or the strict path attempts to persist raw `WorkflowParts` directly.
- When: strict admission or strict readback runs.
- Then: it returns `AdmissionError::StrictRawWorkflowPartsRejected` with record kind `CompiledIr` and boundary `strict_payload_discriminator`.
- And: no `AcceptedArtifact` is synthesized from raw parts.
- Maps: POST-006, INV-004, ERR-STRICT-RAW-021.

`fn given_strict_compiled_ir_record_when_decoded_then_payload_must_be_accepted_artifact_envelope()`

- Given: a strict `CompiledIrRecord` from successful admission.
- When: the payload is decoded.
- Then: it decodes to `AcceptedArtifact` with matching digest, proof digest, capability metadata, and non-sentinel `accepted_at_seq`; decoding as raw `WorkflowParts` is not accepted.
- Maps: POST-006, VERUS-ART-004, FUZZ-ART-008.

### B06: success commits all families together

`fn given_successful_strict_commit_when_reading_storage_then_source_artifact_header_event_and_indexes_are_present()`

- Given: valid strict admission input and a real temporary Fjall journal.
- When: `persist_accepted_run_atomic` succeeds.
- Then: after reopening the journal, source by digest, `CompiledIrRecord` by artifact digest, run header by run id, exactly one `RunAccepted` event for the run/sequence, status index, workflow index, and required action indexes are all present.
- And: all retrieved records contain the same run id, workflow id/digest, artifact digest, policy/capability metadata, and sequence binding.
- Maps: POST-001, INV-001, API-COMPAT-013, INTEG-FAIL-012.

### B07: acknowledgement after durable commit

`fn given_runtime_allocation_or_cli_success_when_observed_then_durable_commit_evidence_already_exists()`

- Given: strict CLI/runtime submit path with real local storage.
- When: the external interface reports success, returns a run id, marks accepted status, or allocates runnable runtime state.
- Then: durable readback after process restart already shows all required record families and indexes for that run.
- And: no success path can observe runtime allocation before durable commit evidence exists.
- Maps: POST-002, INV-002, TLA-ATOM-001.

### B08/E06: sequence truth

`fn given_successful_accepted_run_when_reading_artifact_then_accepted_at_seq_equals_run_accepted_seq()`

- Given: a successful strict accepted run.
- When: the accepted artifact and `RunAccepted` event are read back after restart.
- Then: `AcceptedArtifact.accepted_at_seq == RunAccepted.seq`, `accepted_at_seq != EventSeq::ZERO` if zero is the sentinel in the implementation contract, and both records reference the same run/workflow digest.
- Maps: POST-003, INV-003, VERUS-SEQ-003, KANI-PROP-007.

`fn given_sequence_binding_failure_when_strict_admission_runs_then_sequence_binding_failed_error()`

- Given: missing sequence allocation, sentinel sequence, or mismatched artifact/event sequence input.
- When: strict admission attempts to bind `accepted_at_seq`.
- Then: it returns `AdmissionError::SequenceBindingFailed` with operation, run id, boundary `sequence_binding`, and causal class.
- And: no successful `AcceptedArtifact` is persisted.
- Maps: POST-003, INV-003, ERR-SEQUENCE-020.

### B09/B14/E05: restart/readback durable-only reconstruction

`fn given_restart_after_success_when_readback_runs_then_run_resolves_by_id_workflow_digest_status_and_acceptance_event()`

- Given: a process restart after successful strict admission.
- When: readback runs using run id, workflow id/digest, status index, accepted artifact digest, and event stream.
- Then: every lookup resolves to the same accepted run and same `AcceptedArtifact`/`RunAccepted` sequence relation.
- Maps: POST-004, INV-007, MIRI-CODEC-009.

`fn given_restart_after_success_or_failure_when_readback_runs_then_decision_matches_durable_records_only()`

- Given: prior in-memory admission state exists before restart or a loose artifact exists without full durable families.
- When: readback runs after restart.
- Then: acceptance is reconstructed only from durable source/artifact/header/event/index families; in-memory remnants and loose artifacts do not imply acceptance.
- Maps: INV-007, TLA-ATOM-001, INTEG-FAIL-012.

`fn given_partial_visibility_when_readback_runs_then_partial_visibility_detected_error()`

- Given: a deliberately corrupted store containing any impossible subset: missing source, missing artifact, missing header, missing `RunAccepted`, missing status index, missing workflow index, or missing required action index while some other families exist.
- When: readback runs.
- Then: it returns `AdmissionError::PartialVisibilityDetected` naming the missing/present record families and refuses to acknowledge/recover the run as accepted.
- Maps: POST-005, INV-001, INV-005, INV-007, ERR-PARTIAL-019.

### B10/E03: failure injection at every staging boundary

`fn given_injected_failure_at_each_admission_boundary_when_restarted_then_no_partial_accepted_run_is_visible()`

- Given: failpoints before source stage, before artifact stage, before header stage, before `RunAccepted` stage, before status index stage, before workflow index stage, before each action index stage, and before commit.
- When: strict admission is attempted and the process/journal is reopened.
- Then: each failure point yields the exact typed error required by the boundary and no accepted run is visible through any lookup.
- Maps: POST-005, INV-001, INTEG-FAIL-012, MUT-ERR-010.

`fn given_batch_stage_failure_when_strict_admission_runs_then_batch_stage_failed_error_without_partial_visibility()`

- Given: deterministic encode/key/stage failure for each required record or index before commit.
- When: strict admission runs.
- Then: it returns `AdmissionError::BatchStageFailed` with operation, run id, exact record kind or boundary, and causal class.
- And: batch commit is not attempted; restart/readback finds no accepted run.
- Maps: ERR-STAGE-017.

### B12: visible indexes are never orphaned

`fn given_visible_index_when_target_run_is_read_then_all_required_accepted_run_records_exist()`

- Given: any status/workflow/action index visible in storage for an accepted run.
- When: the indexed run is read.
- Then: source, `AcceptedArtifact`, run header, and `RunAccepted` event also exist and match the indexed identity.
- Maps: INV-005, TLA-ATOM-001, VERUS-IDX-005.

### B13: every failure maps to one typed error with context

`fn given_each_storage_codec_digest_schema_or_proof_failure_when_strict_commit_runs_then_specific_typed_error_is_returned()`

- Given: one failure at a time across storage unavailable, journal stage error, commit error, codec decode error, digest mismatch, schema/gate mismatch, proof mismatch, sequence binding failure, raw payload, index derivation failure, and partial visibility.
- When: strict admission or readback runs.
- Then: exactly one contract `AdmissionError::*` variant is returned with operation, run id where derivable, record kind/boundary, and causal class.
- And: no failure collapses into generic success, generic string error, or an unrelated variant.
- Maps: INV-006, VERUS-ERR-006, ERR-*.

### B15: public API compatibility

`fn public_api_changes_are_semver_reviewed_for_atomic_admission()`

- Given: implementation modifies any public storage/runtime/CLI surface listed in `delivery-scope.jsonl`.
- When: `cargo semver-checks --workspace` or equivalent reviewed API diff runs.
- Then: no unreviewed public API breakage remains; intentional breakage has a reviewed compatibility note and downstream callers are updated.
- Maps: API-COMPAT-013.

### B16: source governance

`fn touched_strict_admission_source_contains_no_forbidden_constructs_or_ignored_results()`

- Given: all touched production files from `delivery-scope.jsonl`.
- When: `moon ci` and targeted forbidden-construct scans run.
- Then: no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing/casts/arithmetic, or ignored persistence `Result` exists in strict admission paths.
- Maps: STATIC-SCAN-011.

### B17: codec/readback Miri gate

`fn strict_accepted_artifact_codec_readback_paths_pass_miri_when_raw_byte_paths_change()`

- Given: implementation touches `AcceptedArtifact` serialization/readback or raw-byte codec paths.
- When: targeted Miri command runs.
- Then: Miri exits 0, or the formal verifier records not-applicable only with evidence that no raw-byte/codec path changed.
- Maps: MIRI-CODEC-009.

### B18: no unsupported performance claim

`fn no_atomic_admission_performance_claim_exists_without_benchmark_evidence()`

- Given: release notes, bead artifacts, implementation comments, or PR text for this bead.
- When: landing review inspects functional/performance claims.
- Then: no speed/vectorization/latency/throughput claim is accepted unless a baseline/result benchmark artifact exists; otherwise `PERF-NONGOAL-014` remains not applicable.
- Maps: PERF-NONGOAL-014.

## 4. Unit Test Groups

### U01: `build_accepted_run_batch(input)` pure/domain validation

- Valid input returns a staged model containing exactly the required families; assert family set, count, identities, and no acknowledgement side effect.
- Each mismatch class returns `AdmissionError::InconsistentAdmissionInput` with exact causal class.
- Missing/invalid source/artifact/header/policy/capability/run/workflow fields return the specified contract error, not a generic storage error.
- Empty required action list yields no action index; non-empty list yields one deterministic action index per action/step.

### U02: `bind_accepted_at_seq(artifact, seq)`

- Non-sentinel sequence for same run/digest returns artifact with exact `accepted_at_seq`.
- Sentinel sequence returns `AdmissionError::SequenceBindingFailed`.
- Mismatched run/digest event context returns `AdmissionError::SequenceBindingFailed`.
- Rebinding an already-bound artifact with different sequence returns `AdmissionError::SequenceBindingFailed` or a contract-specified idempotent same-value result only if implementation explicitly documents that behavior.

### U03: strict payload discriminator / `reject_strict_raw_workflow_parts(record)`

- `AcceptedArtifact` envelope with matching digest/proof/capability metadata returns the exact artifact.
- Raw `WorkflowParts` bytes return `AdmissionError::StrictRawWorkflowPartsRejected`.
- Malformed bytes, legacy schema bytes, stale gate count, missing proof gates, false proof flags, and digest mismatch return `AdmissionError::InvalidAcceptedArtifact` with exact cause.

### U04: index derivation

- Status/workflow/action index keys are deterministic for identical input.
- Different run id, workflow id, status, timestamp, action id, or step id changes only the relevant key component.
- Invalid/unrepresentable index input returns `AdmissionError::IndexDerivationFailed`.

### U05: readback classifier

- Full family set returns `AcceptedRunReadback` with exact source/artifact/header/event/index identities.
- Every single missing family from a partial set returns `AdmissionError::PartialVisibilityDetected` naming the missing family.
- Loose artifact with no `RunAccepted` is not accepted.
- Event/header exists without indexes is not accepted.

### U06: error taxonomy projection

- Storage failure, journal failure, codec failure, digest failure, schema/proof failure, sequence failure, raw payload, index derivation failure, and partial visibility each map to the exact `AdmissionError::*` variant required by the contract.
- Assert context fields: operation, run id where derivable, record kind or boundary, causal class.

## 5. Integration Test Groups

Use real temporary `FjallJournal` stores and reopen them for restart/readback checks. Use deterministic failpoints/fakes only for impossible-to-trigger I/O failures; assert state, never interactions.

### I01: successful strict atomic commit with real storage

- Exercise `persist_accepted_run_atomic` through `vb_storage` and runtime caller.
- Assert all required records are present after reopen.
- Assert all records agree on run id, workflow id/digest, artifact digest, policy/capabilities, status, and `accepted_at_seq`.

### I02: failure injection matrix

- Inject failure before each stage: source, artifact, header, `RunAccepted`, status index, workflow index, each action index, strict commit/sync.
- For each point, assert exact error variant and no accepted run visible after reopen.
- Include a second run in the same store to prove unrelated committed runs remain readable.

### I03: partial visibility corruption/readback

- Build impossible subsets directly in a temp store: each single missing family and representative multi-missing combinations.
- Assert `PartialVisibilityDetected`; assert no acknowledgement/recovery path treats subset as accepted.

### I04: strict artifact compatibility with runtime store

- Store valid `AcceptedArtifact` in `CompiledIrRecord`; `StorageArtifactStore::load_accepted_artifact` and `admit_artifact_run` accept it under Strict/Journaled only with sufficient capabilities.
- Store raw `WorkflowParts`/legacy bytes/malformed bytes; strict path rejects with contract error.
- Relaxed policy behavior remains explicitly separate and must not be used to satisfy strict acceptance.

### I05: CLI/runtime before-ack order

- Through `velvet_ballastics` submit/run path, assert success output occurs only after restart-readable durable evidence exists.
- With commit failpoint, assert CLI returns failure, no success run id/status, and no runtime frame/runnable state.

### I06: API compatibility downstream callers

- Compile and run downstream callers in `vb_runtime` and `velvet_ballastics` against any changed public API.
- Assert no caller bypasses strict accepted-run admission by writing source/artifact/event/index records independently.

## 6. Proptest Invariants

### P01: coherent input roundtrip

- Invariant: generated valid accepted-run input always stages exactly the required record families for one logical run.
- Strategy: generate bounded run ids, workflow ids, workflow digests, artifact digests, capabilities, action indexes, and non-empty source/artifact bytes with coherent references.
- Anti-invariant: any one reference mismatch must return `InconsistentAdmissionInput` and stage nothing acknowledgement-visible.

### P02: sequence binding truth

- Invariant: any non-sentinel event sequence bound to an artifact produces `accepted_at_seq == RunAccepted.seq` for the same run.
- Strategy: generate non-zero `EventSeq`, run id, workflow digest, artifact digest.
- Anti-invariant: sentinel or mismatched sequence/run/digest always returns `SequenceBindingFailed`.

### P03: all-or-none family visibility classifier

- Invariant: only the full required-family set classifies as accepted; empty set classifies absent; every non-empty proper subset classifies `PartialVisibilityDetected`.
- Strategy: generate bitsets over required family enum `{source, artifact, header, event, status_index, workflow_index, action_indexes}`.
- Anti-invariant: no proper subset may return accepted.

### P04: index determinism

- Invariant: same accepted-run model produces identical status/workflow/action keys; changing one identity dimension changes the corresponding key or returns typed derivation failure.
- Strategy: generated bounded ids/timestamps/actions/steps/status bytes.
- Anti-invariant: an index for run A may never resolve to run B.

### P05: strict payload discriminator totality

- Invariant: every decoded strict payload tag is classified as accepted envelope, raw workflow parts, legacy, malformed, stale, or digest/proof mismatch.
- Strategy: arbitrary enum over payload kind plus bounded byte vectors.
- Anti-invariant: raw/legacy/malformed bytes never produce `AcceptedArtifact`.

### P06: error taxonomy totality

- Invariant: every modeled failure cause maps to exactly one `AdmissionError` class.
- Strategy: arbitrary failure-cause enum covering all contract taxonomy classes.
- Anti-invariant: no cause maps to success or generic unclassified error.

### P07: capability/proof metadata coherence

- Invariant: accepted artifact required capabilities are covered by granted capabilities before strict admission succeeds.
- Strategy: generated capability sets and required capability arrays.
- Anti-invariant: missing required capability returns typed invalid/inconsistent admission error per final implementation taxonomy.

### P08: idempotent readback after restart

- Invariant: repeated readback of durable records after restart yields the same accepted/absent/partial decision.
- Strategy: generated durable family sets with coherent identities.
- Anti-invariant: in-memory-only state never changes the durable readback decision.

### P09: batch staging count and abort behavior

- Invariant: successful staging count equals required family count; any staged validation failure aborts the batch or prevents commit per implementation contract.
- Strategy: generated stage plans with one optional failing stage.
- Anti-invariant: failed stage plan cannot produce committed partial accepted run.

## 7. Fuzz Targets

### F01: strict `AcceptedArtifact` compiled IR decoder

- Target: strict decoder/readback path for `CompiledIrRecord.ir` bytes.
- Input type: bytes.
- Risk: raw `WorkflowParts` or malformed/legacy bytes accepted as `AcceptedArtifact`, panic/OOM, wrong error variant.
- Corpus seeds: valid `AcceptedArtifact`, raw `WorkflowParts`, empty bytes, single byte, truncated postcard envelope, overlong vector lengths, stale gate count, missing gate fields, false proof flags, digest mismatch, capability metadata mismatch.
- Maps: FUZZ-ART-008, PRE-005, POST-006, INV-004.

### F02: workflow source/artifact digest coherence parser

- Target: admission input construction from source bytes + artifact bytes + digests.
- Input type: structured arbitrary bytes for source/artifact/header digest fields.
- Risk: digest mismatch bypass, panic, inconsistent input accepted.
- Corpus seeds: all-zero digest, one-bit digest mismatch, swapped source/artifact digest, empty source, maximal allowed source, malformed source bytes.
- Maps: PRE-002, ERR-INCONSISTENT-016.

### F03: readback family-set reconstruction

- Target: readback reconstruction over encoded durable records and indexes.
- Input type: arbitrary set of record blobs keyed by family.
- Risk: partial visibility accepted, orphan indexes accepted, panic on corrupt record.
- Corpus seeds: full family set, each single missing family, duplicate events, mismatched run ids, mismatched workflow ids, orphan status/workflow/action indexes.
- Maps: POST-004, POST-005, INV-005, INV-007, ERR-PARTIAL-019.

### F04: CLI/runtime strict admission input surface

- Target: CLI submit/run argument/file boundary if implementation accepts user-supplied strict admission artifacts.
- Input type: strings and bytes for paths/payloads/options.
- Risk: strict path falls back to relaxed/raw payload, panic, wrong acknowledgement.
- Corpus seeds: missing file, malformed artifact file, raw workflow file, legacy payload, path to valid accepted artifact, unicode path, very long path.
- Maps: POST-002, INV-002, ERR-INVALID-015.

## 8. Kani Harnesses

### K01: accepted sequence binding

- Property: for all bounded run ids, digests, and non-sentinel sequences, successful binding implies `artifact.accepted_at_seq == event.seq` and same run/digest relation; sentinel or mismatch cannot return success.
- Bound: small fixed id/digest arrays and `EventSeq` values including sentinel, 1, max-small, and mismatch cases.
- Rationale: sequence truth is release-critical and proptest can miss boundary combinations.
- Maps: KANI-PROP-007, POST-003, INV-003.

### K02: all-or-none visibility classifier

- Property: over a bounded bitset of required record families, accepted classification is true iff all required bits are present; any non-empty proper subset is partial.
- Bound: 7 family bits plus up to two action-index bits.
- Rationale: exhaustive over the atomicity state lattice; mutation/proptest alone is insufficient for fail-closed proof of every subset.
- Maps: TLA-ATOM-001 complement, POST-005, INV-001, ERR-PARTIAL-019.

### K03: error taxonomy totality

- Property: every bounded failure-cause enum maps to exactly one non-success `AdmissionError` class; no catch-all success/default branch exists.
- Bound: one enum variant per contract failure class plus unknown/unrepresentable index class if implemented.
- Rationale: typed error exhaustiveness is contractual and mutation-critical.
- Maps: VERUS-ERR-006 complement, INV-006, ERR-*.

## 9. Mutation Checkpoints

Minimum threshold: >=90% mutation kill rate overall. Critical mutants below require 100% killed or reviewed-equivalent classification with written rationale.

| Mutant | Must be killed by |
|---|---|
| Drop/ignore `JournalWriteBatch::commit` error and still acknowledge | `given_batch_commit_failure_when_strict_admission_runs_then_batch_commit_failed_error_and_no_ack`; CLI before-ack E2E |
| Move acknowledgement before commit | `given_runtime_allocation_or_cli_success_when_observed_then_durable_commit_evidence_already_exists` |
| Omit workflow source from batch | `given_successful_strict_commit_when_reading_storage_then_source_artifact_header_event_and_indexes_are_present`; partial readback tests |
| Omit accepted artifact from batch | success readback and partial visibility tests |
| Omit run header from batch | success readback and partial visibility tests |
| Omit `RunAccepted` event from batch | sequence truth and partial visibility tests |
| Omit status/workflow/action index | index consistency and partial visibility tests |
| Store raw `WorkflowParts` instead of `AcceptedArtifact` | strict raw rejection and fuzz target F01 |
| Accept malformed/legacy/stale/missing-gate artifact | invalid artifact scenarios and fuzz target F01 |
| Bind `accepted_at_seq` to sentinel or wrong sequence | sequence binding unit/proptest/Kani |
| Collapse typed errors into a generic error | per-error BDD scenarios E01-E08 |
| Swallow stage failure and commit remaining records | stage failure scenario and all-or-none integration matrix |
| Readback infers acceptance from loose artifact or memory state | durable-only restart/readback scenario |
| Orphan index accepted as committed run | visible-index consistency scenario |

Command gate: `cargo mutants --package vb_storage --package vb_runtime --timeout 120` or documented workspace equivalent. Evidence artifact: `mutation-report.md` with mutant list, killed/equivalent/survived summary, and reviewer disposition for any survivor.

## 10. Static, Formal, and CI Gates

| Gate ID | Command/evidence | Required assertion |
|---|---|---|
| G01 | `moon ci` | Full canonical CI exits 0; no source lint/static gate failures. |
| G02 | Forbidden construct scan over `delivery-scope.jsonl` production files | No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing/casts/arithmetic, ignored persistence `Result`. |
| G03 | `cargo semver-checks --workspace` | No unreviewed public API breakage for storage/runtime/CLI admission surfaces. |
| G04 | `cargo +nightly miri test -p vb_storage codec_miri_tests` or exact accepted-artifact codec/readback target | Miri exits 0 if raw-byte/codec paths changed; otherwise formal verifier records not-applicable with file-diff evidence. |
| G05 | TLC rerun for `AtomicAcceptedRunAdmission` | Existing approved TLA+ model still passes; no regression in deadlock, before-ack, restart/readback properties. |
| G06 | Verus rerun for `accepted_run_atomic_admission.rs` | Existing approved Verus pure model still verifies 6/0 or strengthened equivalent. |
| G07 | Kani/fuzz waiver expiry check | `KANI-PROP-007` and `FUZZ-ART-008` must be replaced by concrete harness/target evidence or renewed with valid owner/reason/limitation/expiry before State 12/landing. |
| G08 | No performance claim review | Any new performance claim requires real baseline/result benchmark evidence; otherwise PERF-NONGOAL-014 stays not-applicable. |

## 11. Combinatorial Coverage Matrix

### Accepted-run commit input

| Scenario | Input class | Expected output | Layer | Trace |
|---|---|---|---|---|
| Complete input | coherent source/artifact/header/run/workflow/policy/caps | staged/committed full family set with exact identities | unit + integration | PRE-001, POST-001 |
| Source digest mismatch | source bytes not matching digest | `AdmissionError::InconsistentAdmissionInput` | unit + integration | PRE-002, ERR-INCONSISTENT-016 |
| Artifact digest mismatch | artifact/proof/header mismatch | `AdmissionError::InconsistentAdmissionInput` or `InvalidAcceptedArtifact` per boundary | unit | PRE-002, PRE-005 |
| Missing strict storage | journal unavailable | `AdmissionError::BatchCommitFailed`, no ack | integration + E2E | PRE-003, ERR-COMMIT-018 |
| Index derivation valid | all ids bounded/valid | exact status/workflow/action keys | unit | PRE-004 |
| Index derivation invalid | unrepresentable/malformed key component | `AdmissionError::IndexDerivationFailed` | unit + integration | ERR-INDEX-022 |

### Strict artifact payload

| Scenario | Input class | Expected output | Layer | Trace |
|---|---|---|---|---|
| Valid envelope | postcard `AcceptedArtifact` matching digest/proof/caps | exact artifact | unit + integration | POST-006 |
| Raw workflow parts | postcard `WorkflowParts` bytes | `AdmissionError::StrictRawWorkflowPartsRejected` | unit + fuzz | ERR-STRICT-RAW-021 |
| Malformed bytes | arbitrary invalid bytes | `AdmissionError::InvalidAcceptedArtifact` | unit + fuzz | ERR-INVALID-015 |
| Legacy schema | older payload shape | `AdmissionError::InvalidAcceptedArtifact` | unit + fuzz | PRE-005 |
| Stale/missing gates | wrong gate count or missing required proof gate | `AdmissionError::InvalidAcceptedArtifact` | unit + integration | PRE-005 |
| Digest mismatch | envelope digest != record key/header/proof | `AdmissionError::InvalidAcceptedArtifact` or `InconsistentAdmissionInput` at exact boundary | unit + integration | PRE-002, PRE-005 |

### Sequence binding

| Scenario | Input class | Expected output | Layer | Trace |
|---|---|---|---|---|
| Non-sentinel matching seq | artifact + `RunAccepted` same run | artifact `accepted_at_seq == event.seq` | unit + integration + Kani | POST-003 |
| Sentinel seq | zero/sentinel | `AdmissionError::SequenceBindingFailed` | unit + Kani | ERR-SEQUENCE-020 |
| Mismatched seq/run | event not same run or seq differs | `AdmissionError::SequenceBindingFailed` | unit + Kani | INV-003 |
| Reopen after success | durable readback | same sequence relation after restart | integration | POST-004 |

### Failure/partial visibility

| Scenario | Input class | Expected output | Layer | Trace |
|---|---|---|---|---|
| Fail before source | failpoint | exact typed error; absent accepted run | integration | POST-005 |
| Fail before artifact | failpoint | exact typed error; absent accepted run | integration | POST-005 |
| Fail before header | failpoint | exact typed error; absent accepted run | integration | POST-005 |
| Fail before event | failpoint | exact typed error; absent accepted run | integration | POST-005 |
| Fail before status index | failpoint | exact typed error; absent accepted run | integration | INV-005 |
| Fail before workflow index | failpoint | exact typed error; absent accepted run | integration | INV-005 |
| Fail before action index | failpoint | exact typed error; absent accepted run | integration | INV-005 |
| Fail at commit/sync | failpoint | `AdmissionError::BatchCommitFailed`; no ack | integration + E2E | POST-002, ERR-COMMIT-018 |
| Every non-empty proper subset | corrupted durable records | `AdmissionError::PartialVisibilityDetected` | unit + integration + Kani | ERR-PARTIAL-019 |

### Error variants

| Variant | Input class | Expected output | Layer | Trace |
|---|---|---|---|---|
| InvalidAcceptedArtifact | malformed/raw/legacy/stale/missing gate/digest mismatch artifact | exact `InvalidAcceptedArtifact` with context | unit + integration + fuzz | ERR-INVALID-015 |
| InconsistentAdmissionInput | coherent-field mismatch | exact `InconsistentAdmissionInput` with context | unit + integration | ERR-INCONSISTENT-016 |
| BatchStageFailed | encode/key/stage failure | exact `BatchStageFailed`, no partial visibility | integration | ERR-STAGE-017 |
| BatchCommitFailed | commit/sync failure | exact `BatchCommitFailed`, no ack | integration + E2E | ERR-COMMIT-018 |
| PartialVisibilityDetected | impossible durable subset | exact `PartialVisibilityDetected` | unit + integration | ERR-PARTIAL-019 |
| SequenceBindingFailed | sentinel/missing/mismatched sequence | exact `SequenceBindingFailed` | unit + Kani | ERR-SEQUENCE-020 |
| StrictRawWorkflowPartsRejected | raw `WorkflowParts` payload | exact `StrictRawWorkflowPartsRejected` | unit + fuzz | ERR-STRICT-RAW-021 |
| IndexDerivationFailed | invalid required index key/value | exact `IndexDerivationFailed` | unit + integration | ERR-INDEX-022 |

## 12. Open Questions for Test Writer

1. Final production names for `AcceptedRunCommitInput`, `AcceptedRunCommitReceipt`, `persist_accepted_run_atomic`, `build_accepted_run_batch`, `bind_accepted_at_seq`, and `readback_accepted_run` may differ from contract signatures. Test names must preserve behavior names even if APIs are renamed.
2. The exact sentinel rule for `EventSeq` must be confirmed in implementation. The plan treats a sentinel/zero sequence as invalid for strict successful admission because the contract requires non-sentinel sequence truth.
3. Deterministic failpoint/fake design for Fjall commit failure must avoid mocking ordinary queries; use real storage for normal integration and only fake/failpoint nondeterministic I/O errors.
4. `KANI-PROP-007` and `FUZZ-ART-008` are approved waivers in State 6, but this State 7 plan requires concrete harness/target replacement or renewed valid waiver before State 12/landing.

## 13. Completion Evidence

- State 7 isolated workspace guard passed: `pwd -P` returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission`; shell guard required that exact path.
- Required input artifacts were present and non-empty; `jq -c .` parsed `traceability-matrix.jsonl`, `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, and `delivery-scope.jsonl` successfully.
- Approved State 6 inputs were read: `proof-review.md` and `contract-verification-review.md` both state `STATUS: APPROVED`.
- This plan writes only `.beads/vb-core-atomic-admission/test-plan.md` and the State 7 transition/evidence in `.beads/vb-core-atomic-admission/STATE.md`.
- No production code, test code, proof/model code, dependency files, CI files, or source-checkout files were edited.
