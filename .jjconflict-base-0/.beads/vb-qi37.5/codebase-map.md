# Codebase map: vb-qi37.5 State 2 exploration

bead_id: vb-qi37.5
title: verifier: Idempotency verification gate
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5
state: 2 explore and scope

## Bead acceptance summary

The bead asks to replace the master-doc stub idempotency gate with real static/runtime verification for retry-safe actions and replay-safe workflow behavior. Acceptance requires:

- Verification rejects non-idempotent actions in retryable positions unless an explicit safe policy exists.
- Action contracts expose idempotency metadata.
- Generated certificates include idempotency results.
- Tests cover retry, duplicate completion, stale completion, and non-idempotent action rejection.

Command evidence: `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.5 --json` succeeded from the isolated workspace and reports dependencies vb-qi37.5.1/vb-qi37.5.2 closed, vb-qi37.5.3/vb-qi37.5.4 in progress.

## Relevant production crates and files

### vb_core: action contract and runtime idempotency primitives

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_core/src/action.rs`
  - Symbols: `Idempotency`, `SideEffect`, `RetrySafety`, `RetryPolicy`, `IdempotencyViolation`, `ActionContract`, `ActionTicket`, `ActionError::NonIdempotentReplayBlocked`, `ActionError::CompletionAlreadyRecorded`, `verify_idempotency`, `validate_idempotency_key_ingredients`, `issue_action_ticket`, `propagate_action_taint`.
  - Evidence: lines 12-102 define action contract metadata; lines 317-373 implement runtime key/taint idempotency validation; lines 131-148 define `ActionTicket.idempotency_key`.
  - Scope note: random/time key rejection variants exist but the current validation only enforces secret/derived-secret taint.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_core/src/kani_idempotency_gates.rs`
  - Symbols/proofs: KANI-RUNTIME-001 through KANI-RUNTIME-006 for `verify_idempotency`.
  - Scope note: KANI-RUNTIME-004 and KANI-RUNTIME-005 are placeholder-current-behavior proofs that assert RandomInKey/TimeInKey are not enforced yet.

### vb_validate: static idempotency verifier model

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_validate/src/idempotency_contract.rs`
  - Symbols: `validate_workflow_idempotency_contracts`, `validate_action_idempotency_contract`, `is_statically_idempotent_contract`, `IdempotencyContractError`, `IdempotencyContractViolation`.
  - Evidence: lines 123-160 define the stricter static decision table: side-effecting `RetrySafety::Unsafe`, `Idempotency::AtLeastOnceExternal`, and side-effecting `Idempotency::DeterministicPure` are rejected.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_validate/src/shared.rs`
  - Symbols: `ValidationPipeline::validate_with_contracts`, free `validate_with_contracts`.
  - Evidence: lines 139-151 run the structural validation pipeline and gate 12 contract completeness only. It does not call `idempotency_contract::validate_workflow_idempotency_contracts`.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_validate/src/gates.rs`
  - Symbols: `validate_gate_12_action_contract_completeness`, capability schema helpers.
  - Evidence: lines 1403-1454 enforce action-contract bijection/capability schema, not idempotency legality.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_validate/src/kani_idempotency_contract.rs`
  - Symbols/proofs: KANI-DECISION-001 through KANI-DECISION-005 for the static decision table.
  - Scope note: comments document a match-order mismatch in obligation wording for `RetrySafety::Unsafe` with deterministic/at-least-once variants.

### vb_compile: compile-time entry point and parity gap

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_compile/src/lib.rs`
  - Symbols: `compile_workflow_with_contracts`, `check_idempotency_gates`, `CompileError::IdempotencyViolation`.
  - Evidence: lines 475-484 call `validate_with_contracts` then `check_idempotency_gates`; lines 1014-1060 reject side-effecting `RetrySafety::Unsafe` and `Idempotency::AtLeastOnceExternal`, but do not reject side-effecting `Idempotency::DeterministicPure` unless retry safety is unsafe.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_compile/src/kani_idempotency_parity.rs`
  - Symbols/proofs: cross-crate parity harness between `check_idempotency_gates` and `vb_validate::idempotency_contract::is_statically_idempotent_contract`.
  - Scope note: requires proof review because read tests show known parity disagreements.

### vb_storage: accepted artifact/certificate and replay policy

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_storage/src/admission.rs`
  - Symbols: `VerificationProof`, `AcceptedArtifact`, `submit_artifact`, `submit_artifact_with_contracts`, `admit_compiled_artifact`, `VerificationWarning`, `ProofFlag`.
  - Evidence: lines 58-81 define proof fields including `retry_safe`, `replayable`, `idempotency_keyed`, `idempotency_attested`; lines 117-119 set `ADMISSION_GATE_COUNT` to 2; lines 188-199 create a default-true proof with no idempotency evidence populated.
  - Scope note: generated certificate fields exist, but current submit path does not derive idempotency_keyed/idempotency_attested from contracts.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_storage/src/recovery/replay/core.rs`
  - Symbols: `replay_events`, `recover_full_journal`, `recover_snapshot_plus_tail`.
  - Evidence: lines 80-87 reject an `ActionScheduled` event when the tracker already resolved that action/step.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_storage/src/recovery/hydrate_support.rs`
  - Symbols: tail hydration action tracker checks.
  - Evidence: lines 152-158 and 249-255 reject scheduling already-resolved action/step pairs as `NonIdempotentActionBlocked`.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_storage/src/recovery/types.rs`
  - Symbols: `RecoveryError::NonIdempotentActionBlocked`, `ActionReplayTracker`.
  - Evidence found by grep: fields carry action and step.

### vb_runtime: strict admission consumes accepted artifact proof

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_runtime/src/admission.rs`
  - Symbols: `REQUIRED_GATE_COUNT`, `ArtifactEnvelopeError`, `AdmissionError`, `AcceptedArtifactStore`, `StorageArtifactStore`, `admit_artifact_run`, `admit_run`.
  - Evidence: line 16 requires 15 gates; lines 324-347 validate `gate_count == 15` and proof flags `bounded`, `taint_safe`, `retry_safe`, `durable`, `replayable`; lines 377-448 enforce full artifact validation and capability coverage before run admission.
  - Scope note: runtime requires 15 gates while storage submit path emits gate_count 2; this is an integration risk for accepted artifact admission.

## Relevant tests and verification artifacts

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_compile/tests/idempotency_parity.rs`
  - Coverage: side-effect none acceptance, unsafe side effects rejected, idempotent external accepted, at-least-once external rejected.
  - Risk evidence: lines 142-196 explicitly count 16 disagreement cases between static and compile gate, including side-effecting deterministic-pure with Safe/KeyRequired.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/tests/gate_12_14_15_tests.rs`
  - Coverage: gate 12 action-contract bijection and capability schema. Does not cover idempotency legality.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/tests/integration_validation_tests.rs`
  - Coverage: validates public adapter routes through `validate_with_contracts`; current helpers use pure/no-side-effect contracts.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_storage/src/recovery/tests.rs` and `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_storage/src/recovery/vb_h6ix_tests.rs`
  - Coverage found by grep: `action_tracker_blocks_non_idempotent_replay`, `RecoveryError::NonIdempotentActionBlocked` tests.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/crates/vb_storage/src/vb_2bok_durability_gate_tests.rs`
  - Coverage found by grep: accepted artifact proof/gate_count and duplicate admission idempotence tests.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/verification/verus/step_state_machine.rs`
  - Related proof: `lemma_terminal_idempotency`, but this is terminal state idempotency, not action side-effect idempotency.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5/verification/verus/taint_lattice.rs`
  - Related proof: taint join idempotence, not action retry idempotency.

## Required downstream contract/proof/test focus

1. Decide the canonical static decision table: `vb_validate::idempotency_contract` currently rejects side-effecting deterministic-pure; `vb_compile::check_idempotency_gates` currently allows it when retry_safety is Safe/KeyRequired. This must be reconciled before implementation.
2. Thread idempotency contract checking into the same public verification/compile path that emits accepted artifacts and certificates. `validate_with_contracts` alone does not enforce idempotency legality.
3. Make `VerificationProof.idempotency_keyed` and `VerificationProof.idempotency_attested` derived from actual action contracts, not default empty arrays.
4. Reconcile storage submit gate count 2 with runtime required gate count 15 before accepted artifact admission can honestly pass.
5. Runtime/recovery duplicate completion, stale completion, and retry tests must bind to actual `ActionTicket.idempotency_key`, digest equality/difference, and non-idempotent policy. Current replay tracker is action/step based, not obviously key/digest aware from explored files.

## Risk tags

- temporal: retry/replay and stale completion acceptance depend on journal event ordering.
- persistence: accepted artifacts and journal replay/hydration are storage-backed.
- public API: `compile_workflow_with_contracts`, `validate_with_contracts`, storage `submit_artifact_with_contracts`, runtime `admit_artifact_run`.
- parser/codec: accepted artifacts are postcard-encoded and digest-checked.
- verification: Kani harnesses exist but include placeholder/current-behavior obligations for random/time key enforcement.
- user-visible behavior: certificate/admission diagnostics and CLI verify output must expose idempotency results.
- performance: no new perf-critical scope identified for State 2, but admission/replay checks are hot-adjacent.

## Open questions / unknowns

- UNKNOWN: whether CLI certificate rendering consumes `idempotency_keyed`/`idempotency_attested`; not mapped in this focused pass.
- UNKNOWN: exact duplicate completion digest semantics in runtime action completion path; grep found `CompletionAlreadyRecorded`, but focused reads did not locate digest comparison logic.
- UNKNOWN: whether existing proof runner executes `#[cfg(kani)]` idempotency harnesses in CI; formal-verifier must confirm in later states.
