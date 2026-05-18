# Contract Specification: vb-qi37.1

## Context
- Feature: runtime/storage full live-frame recovery hydration for `velvet-ballastics`.
- Bead: `vb-qi37.1` - `runtime/storage: Complete full live-frame recovery hydration`.
- Source constraints read from State 2 artifacts and `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.1 --json`.
- Scope: recovery from durable Fjall journal/run headers plus optional snapshots into `RecoveryRuntimeSummary`, `RecoveryFrameSeed`, and runtime `RunFrame` hydration boundaries.

## Domain terms
- Run: durable execution identified by `RunId`.
- Journal event: ordered persisted `JournalEvent` with `EventSeq` and a run identity.
- Snapshot: compact `RunSnapshot` at a persisted event sequence.
- Frame seed: storage-level `RecoveryFrameSeed` containing pc, dimensions, slots, taint, step states, pending actions, summary, and unsupported-state flags.
- Live frame: runtime `RunFrame` that can resume or inspect execution state.
- Fail closed: recovery returns a typed error instead of fabricating or silently discarding state.

## Assumptions
- State 3 does not edit production code, tests, or proof/model files.
- Existing verification file `verification/verus/recovery_verification.rs` is a known Verus target for unsupported-state and digest-gap proofs.
- No existing TLA+ recovery-hydration model was found in State 2 artifacts; TLA+ model creation is a later proof-writer task.
- Pending action resumability may still be unsupported by current runtime projection; unsupported state must reject live-frame hydration until implemented.

## Open questions
- OQ-001: Is slot taint represented durably in `SlotWrittenEvent::extra`, recovered from workflow replay, or still absent for event-only recovery?
- OQ-002: Are waits/asks/retries/collect pagination intended to be resumable in `RunFrame` now, or explicitly fail-closed until child work lands?
- OQ-003: Resolved for this bead after State 6 rejection: current isolated production code has no action ABI or policy digest lookup/check path in `DigestCheck::Full`; this bead requires workflow-source and compiled-IR digest checks only. Action ABI and policy digest checks are explicit optional/deferred contract clauses until a production design surface exists.

## Preconditions
- PRE-001: The caller supplies a concrete `RunId` and recovery input set containing journal events, or a snapshot plus strictly later tail events.
- PRE-002: Every event used for one recovery attempt belongs to the requested `RunId`; mixed-run streams are invalid.
- PRE-003: Snapshot plus tail recovery requires `snapshot.run == run_id` and each tail `EventSeq` must be greater than `snapshot.seq`.
- PRE-004: Digest verification inputs include expected workflow digest and, when `DigestCheck::WorkflowAndIr` or `DigestCheck::Full` is requested, expected/found compiled IR digests.
- PRE-005: Full live-frame hydration may proceed only when `UnsupportedRecoveryState` has all flags false.

## Postconditions
- POST-001: Successful summary recovery contains non-empty event bounds: `first_seq <= last_seq`, `run == requested RunId`, and counts derived only from durable events.
- POST-002: Successful frame-seed recovery reconstructs pc, step dimensions, slot dimensions, step states, slot values, slot taints, pending action facts, terminal state, and journal sequence bounds from durable data.
- POST-003: `hydrate_run_frame` and `hydrate_run_frame_from_events` never return an empty successful frame for a non-empty run when required live-frame state is missing or unsupported.
- POST-004: Snapshot plus tail hydration applies snapshot slots/taints first, then only ordered tail events after the snapshot sequence.
- POST-005: Summary-only recovery cannot be converted by the runtime boundary into a successful empty `RunFrame`.
- POST-006: Digest mismatch detection returns typed `RecoveryError` variants for workflow source and compiled IR mismatch. Action ABI and policy digest mismatch detection are not required for this bead because production `verify_digests` has no action ABI or policy digest input/lookup/check surface; ERR-004 and ERR-005 are waived optional downstream obligations, not State 5 blockers.
- POST-007: Crash-before-ack and crash-after-ack recovery evidence demonstrates persisted headers/events/snapshots survive restart without lost slots, taint, tickets, waits, asks, retries, or collect state; unsupported pieces return typed diagnostics.

## Invariants
- INV-001: Journal sequence order is monotonic within a recovery stream; out-of-order or corrupt ordering cannot be accepted as faithful recovery.
- INV-002: A recovered live frame must not contain fabricated default slot values or fabricated clean taint.
- INV-003: Unsupported durable state flags are reject gates, not warnings.
- INV-004: Recovery never reparses runtime YAML/JSON/HTTP artifacts; it consumes accepted durable artifacts and typed persisted events.
- INV-005: Failures propagate as typed recovery/runtime diagnostics; no fallible storage/recovery result is silently discarded.
- INV-006: Terminal state recovered from journal events must match the terminal result exposed by recovery output.

## Error taxonomy
- ERR-001: `RecoveryError::Journal` - journal storage read/write failure during recovery; expected scenario `given_journal_io_failure_when_recovering_then_recovery_error_journal_is_returned`.
- ERR-002: `RecoveryError::WorkflowSourceDigestMismatch` - accepted workflow digest differs from requested digest; expected scenario `given_workflow_source_digest_mismatch_when_verify_digests_runs_then_workflow_source_digest_mismatch_is_returned`.
- ERR-003: `RecoveryError::CompiledIrDigestMismatch` - compiled IR digest differs from expected digest; expected scenario `given_compiled_ir_digest_mismatch_when_verify_digests_runs_then_compiled_ir_digest_mismatch_is_returned`.
- ERR-004: `RecoveryError::ActionAbiMismatch` - optional/downstream only. Waived for this bead because the production digest API has no action ABI digest input or lookup path; expected downstream scenario after design exists: `given_action_abi_digest_mismatch_in_full_mode_when_verify_digests_runs_then_action_abi_mismatch_is_returned`.
- ERR-005: `RecoveryError::PolicyDigestMismatch` - optional/downstream only. Waived for this bead because the production digest API has no policy digest input or lookup path; expected downstream scenario after design exists: `given_policy_digest_mismatch_in_full_mode_when_verify_digests_runs_then_policy_digest_mismatch_is_returned`.
- ERR-006: `RecoveryError::ReplayDivergence` - mixed run, invalid sequence, impossible transition, terminal mismatch, corrupt ordering, or replay state drift; expected scenario `given_mixed_run_or_corrupt_sequence_when_recovery_runs_then_replay_divergence_is_returned`.
- ERR-007: `RecoveryError::NoRecoveryData` - requested recovery has no usable snapshot or journal evidence; expected scenario `given_empty_journal_and_no_snapshot_when_recovery_runs_then_no_recovery_data_is_returned`.
- ERR-008: `RecoveryError::CorruptSnapshot` - snapshot slot/taint bytes are corrupt or undecodable; expected scenario `given_corrupt_snapshot_bytes_when_hydrating_then_corrupt_snapshot_is_returned`.
- ERR-009: `RecoveryError::TerminalStateMismatch` - recovered terminal state contradicts expected terminal result; expected scenario `given_terminal_event_contradicts_recovered_terminal_when_hydrating_then_terminal_state_mismatch_is_returned`.
- ERR-010: `RecoveryError::FrameDimensionOverflow` - recovered dimensions exceed representable runtime frame bounds; expected scenario `given_recovered_dimensions_exceed_runtime_bounds_when_hydrating_then_frame_dimension_overflow_is_returned`.
- ERR-011: `RuntimeError::InvalidRecoveryHydration` - runtime boundary rejects unsupported or inconsistent frame seed; expected scenario `given_unsupported_frame_seed_when_runtime_boundary_hydrates_then_invalid_recovery_hydration_is_returned`.
- ERR-012: `RuntimeError::UnsupportedFullRecoveryHydration` - summary-only recovery attempts live-frame hydration; expected scenario `given_summary_only_recovery_when_runtime_boundary_hydrates_then_unsupported_full_recovery_hydration_is_returned`.

## Contract signatures
- `fn verify_digests(journal: &FjallJournal, run: RunId, workflow_digest: WorkflowDigest, ir_digest: WorkflowDigest, found_ir_digest: WorkflowDigest, level: DigestCheck) -> RecoveryResult<()>`
- `fn recover_runtime_summary(journal: &FjallJournal, run: RunId) -> RecoveryResult<RecoveryHydration>`
- `fn recover_runtime_frame_seed(journal: &FjallJournal, run: RunId) -> RecoveryResult<RecoveryFrameSeed>`
- `fn recover_run_admission(journal: &FjallJournal, run: RunId) -> RecoveryResult<Option<RecoveredRunAdmission>>`
- `fn recover_all_incomplete_runs(journal: &FjallJournal) -> RecoveryResult<Vec<RecoveryHydration>>`
- `fn hydrate_run_frame(snapshot: &RunSnapshot, tail_events: &[JournalEvent], run_id: RunId) -> RecoveryResult<RunFrame>`
- `fn hydrate_run_frame_from_events(events: &[JournalEvent], run_id: RunId) -> RecoveryResult<RunFrame>`
- `fn recovery_boundary_from_hydration(hydration: RecoveryHydration) -> Box<dyn RuntimeRecoveryBoundary>`
- `fn RuntimeRecoveryBoundary::hydrate_run_frame(&self) -> RuntimeResult<RunFrame>`

## Verus-owned clauses
- PRE-005, POST-003, POST-005, INV-002, INV-003, INV-005. INV-005 must use a non-vacuous typed-error model: `Err(e)` input or fallible decision result remains `Err(typed(e))` or refines to an explicitly named runtime diagnostic; an implication whose consequent repeats its antecedent is not acceptable.

## TLA+-owned clauses
- PRE-001, PRE-002, PRE-003, POST-001, POST-002, POST-004, POST-007, INV-001, INV-004, INV-006.

## Typed-error traceability clauses
- ERR-001 through ERR-012 are contract clauses and must each appear in `traceability-matrix.jsonl` with an exact expected scenario and at least one proof, static, property, integration, or manual evidence obligation.

## Theorem-owned clauses
- None for State 3. Verus plus TLA+ own the core and temporal properties.

## Non-goals
- Implementing recovery code, tests, proof code, or TLA+/Verus models in State 3.
- UI recovery behavior.
- Generated Rust/maxperf execution evidence.
- Performance speedup claims; no performance requirement is introduced by this contract.

## State 3 repair transition after State 6 rejection
- Date: 2026-05-15.
- Transition: State 6 rejected required action ABI/policy digest obligations as production-detached and rejected the typed-error Verus proof as tautological.
- Repair decision: POST-006, `VERUS-DIGEST-001`, and traceability now require only workflow-source and compiled-IR digest mismatch detection for this bead. ERR-004 and ERR-005 are explicit optional downstream obligations with waiver metadata and follow-up triggers.
- Typed-error repair: INV-005/`VERUS-INV-005` now requires a non-vacuous typed-error propagation/refinement proof or, if Verus cannot own it, a later waiver plus static/property/integration evidence. The previous tautological shape is forbidden by this contract.

## State 3 schema repair transition after contract-verification rejection
- Date: 2026-05-15.
- Transition: State 6 contract verification rejected the contract-time schema because `PRE-004` had no direct `proof-obligations.jsonl` row, optional waiver rows used non-`planned` statuses, and `PO-036` lacked an explicit `limitation`.
- Repair decision: `VERUS-PRE-004` is the direct proof obligation for the digest-input precondition. It verifies the production-visible workflow-source and compiled-IR digest input requirements for `DigestCheck::WorkflowAndIr` and `DigestCheck::Full`; action ABI and policy digest inputs remain optional downstream waivers until production exposes those surfaces.
- Schema decision: contract-time proof-obligation rows, including waiver rows with `required:false`, must keep `status: "planned"`; execution outcomes such as waived/pass/fail remain reserved for later verifier states.
- Waiver repair: `PO-036` now states the explicit limitation for omitted fuzz, theorem-kernel, and dependency-specific lanes.
