# Contract Specification: vb-qi37.5 Idempotency Verification Gate

## Context

- Bead: `vb-qi37.5` - verifier idempotency verification gate.
- Feature: replace stub idempotency gate with static/runtime verification for retry-safe actions and replay-safe workflow behavior.
- State 2 inputs: `codebase-map.md`, `delivery-scope.jsonl`, `baseline-report.md`, and `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.5 --json`.
- In-scope crates: `vb_core`, `vb_validate`, `vb_compile`, `vb_storage`, `vb_runtime`.

## Domain Terms

- Action contract: `vb_core::action::ActionContract` metadata describing action idempotency, retry safety, side effects, capabilities, and taint.
- Idempotency metadata: `Idempotency`, `RetrySafety`, `SideEffect`, idempotency key ingredients, and certificate fields `idempotency_keyed`/`idempotency_attested`.
- Retryable position: workflow position where retry, resume, replay, or duplicate completion can re-attempt an action.
- Replay-safe side effect: an external effect whose repeat execution is blocked, collapsed, or proven equivalent by a deterministic key/policy.
- Accepted artifact: storage/runtime artifact with proof flags and admission metadata.

## Assumptions

- The canonical static decision table is owned by `vb_validate::idempotency_contract::is_statically_idempotent_contract` unless contract-verification review explicitly rejects that choice.
- `vb_compile::check_idempotency_gates` must either call/refine the same decision table or prove parity against it.
- Existing Kani harness files named in State 2 are proof surfaces, but later proof agents must confirm exact Kani runner wiring.
- Storage/runtime 2-gate versus 15-gate mismatch is in scope because bead acceptance requires generated certificates and runtime admission to carry idempotency results.

## Open Questions

- Whether CLI certificate rendering already consumes `VerificationProof.idempotency_keyed` and `idempotency_attested`.
- Exact runtime duplicate completion digest semantics beyond `ActionError::CompletionAlreadyRecorded`.
- Whether random/time idempotency key ingredients must be rejected in this bead or filed as a follow-up; current State 2 notes show placeholders.

## Preconditions

- PRE-001: Every workflow action referenced by verification or compilation has exactly one complete `ActionContract` reachable from the validation/compile path.
- PRE-002: Every retryable, resumable, or replayable action position has an explicit `RetrySafety` and `Idempotency` value.
- PRE-003: Every side-effecting action that claims safe retry/replay has deterministic idempotency metadata sufficient to form or attest a stable key.
- PRE-004: Accepted artifact admission receives certificate/proof data derived from the same action contracts accepted by validation/compilation.

## Postconditions

- POST-001: Static verification rejects any side-effecting action in a retryable/replayable position when the canonical decision table classifies the contract as non-idempotent.
- POST-002: Validation and compilation make identical pass/fail decisions for every `Idempotency` x `SideEffect` x `RetrySafety` combination.
- POST-003: Generated certificates expose idempotency results: keyed action identifiers and attested action identifiers are populated from verified contracts, not default-empty stubs.
- POST-004: Runtime admission rejects missing, failed, mismatched, or stale idempotency proof evidence before a runnable state is acknowledged.
- POST-005: Replay/hydration rejects duplicate non-idempotent scheduling after an action/step has reached a resolved terminal observation.
- POST-006: Duplicate completion is idempotent only when the completion refers to the same action ticket/key and same effect digest; stale or conflicting completions are rejected with typed errors.

## Invariants

- INV-001: There is one canonical idempotency decision table for verifier, compiler, certificate generation, and admission semantics.
- INV-002: Side-effecting `RetrySafety::Unsafe` actions are never accepted in retryable/replayable positions.
- INV-003: Side-effecting `Idempotency::AtLeastOnceExternal` actions are never accepted without an explicit safe policy contract.
- INV-004: Side-effecting `Idempotency::DeterministicPure` is rejected unless the contract proves there is no external side effect or the decision table is amended with an explicit safe policy.
- INV-005: Certificate idempotency fields are a sound summary of accepted action contracts: no action may be attested/keyed in the certificate unless its contract passed the canonical gate.
- INV-006: Runtime and storage agree on the accepted artifact proof schema and gate count before runtime admission accepts the artifact.
- INV-007: Replay state is monotonic for action completion: once an action/step is resolved, later replay cannot schedule another non-idempotent external effect for the same action/step.
- INV-008: All idempotency diagnostics are deterministic and include the rejected action identity, reason, and repair hint.

## Error Taxonomy

- `IdempotencyContractError::MissingContract` - violates PRE-001.
- `IdempotencyContractError::UnsafeRetrySideEffect` - violates INV-002.
- `IdempotencyContractError::AtLeastOnceExternalReplay` - violates INV-003.
- `IdempotencyContractError::DeterministicPureHasSideEffect` - violates INV-004.
- `CompileError::IdempotencyViolation` - compile path rejects a static contract that fails POST-001/POST-002.
- `AdmissionError::MissingIdempotencyEvidence` - accepted artifact lacks required idempotency proof data.
- `AdmissionError::FailedIdempotencyEvidence` - certificate reports an idempotency gate failure.
- `AdmissionError::ProofSchemaMismatch` - storage/runtime proof gate count or flags violate INV-006.
- `RecoveryError::NonIdempotentActionBlocked` - replay/hydration would re-schedule a resolved non-idempotent action.
- `ActionError::CompletionAlreadyRecorded` - duplicate completion exactly repeats an already recorded action result.
- `ActionError::NonIdempotentReplayBlocked` - duplicate/stale/conflicting completion or replay would duplicate an external effect.

## Contract Signatures

- `fn validate_workflow_idempotency_contracts(workflow, contracts) -> Result<(), IdempotencyContractError>`
- `fn validate_action_idempotency_contract(action_id, contract) -> Result<(), IdempotencyContractError>`
- `fn is_statically_idempotent_contract(contract) -> Result<IdempotencyDecision, IdempotencyContractViolation>`
- `fn check_idempotency_gates(compiled, contracts) -> Result<(), CompileError>`
- `fn submit_artifact_with_contracts(artifact, contracts) -> Result<AcceptedArtifact, AdmissionError>`
- `fn admit_artifact_run(artifact, run_request) -> Result<RunAdmission, AdmissionError>`
- `fn replay_events(events, tracker) -> Result<ReplayOutcome, RecoveryError>`
- `fn verify_idempotency(ticket, completion_or_replay) -> Result<(), ActionError>`

## Verus-Owned Clauses

- VERUS-INV-001: Pure decision-table determinism and totality for action contract classification.
- VERUS-INV-002: Certificate summary soundness: every reported keyed/attested action maps to an accepted contract.
- VERUS-INV-003: Replay tracker monotonicity over resolved action/step pairs.

## TLA+-Owned Clauses

- TLA-RETRY-001: Retry/replay lifecycle never schedules a rejected non-idempotent side effect.
- TLA-REPLAY-002: Duplicate and stale completions either collapse to the same recorded outcome or are rejected before external effect duplication.
- TLA-ADMIT-003: Accepted artifact admission cannot reach runnable state unless idempotency evidence is present, passed, and schema-compatible.

## Theorem-Owned Clauses

- None required at State 3. The idempotency lattice and replay monotonicity properties are small enough for Verus/TLA+ first. Lean/Aeneas/Hax is deferred unless Verus cannot express the certificate-summary refinement.

## Non-goals

- No production code, tests, or proof code in State 3.
- No performance speedup claim. Admission/replay checks must remain bounded, but no p99 target is contracted here.
- No UI behavior.
- No generated Rust/maxperf codegen parity.
