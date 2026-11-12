# TLA+ Temporal Model Plan: vb-qi37.5

## Boundary

- Temporal/workflow behavior: validation-to-compile-to-certificate-to-admission lifecycle, retry/replay scheduling, duplicate completion, stale completion, and fail-closed admission.
- Rust/core behavior excluded from TLA+ and handled by Verus/Kani/tests: pure idempotency decision table, certificate array/set construction, key ingredient validation, typed error formatting.
- External systems abstracted: Fjall/Postcard storage is modeled as durable artifact/journal states; wall-clock, filesystem, and concrete serialization bytes are outside TLA+.
- Non-applicability rationale: not applicable; this bead is explicitly temporal because retry, replay, admission, duplicate completion, stale completion, and persistence ordering are state-over-time behaviors.

## TLA+-Owned Clauses

- TLA-RETRY-001 -> `specs/idempotency_gate/IdempotencyGate.tla::NoRejectedEffectScheduled`
- TLA-REPLAY-002 -> `specs/idempotency_gate/IdempotencyGate.tla::ResolvedActionMonotonic`
- TLA-ADMIT-003 -> `specs/idempotency_gate/IdempotencyGate.tla::AdmissionRequiresPassedIdempotencyEvidence`

## Model Shape

- Module/model path: `specs/idempotency_gate/IdempotencyGate.tla`
- Config path: `specs/idempotency_gate/IdempotencyGate.cfg`
- Variables: `actions`, `contracts`, `decision`, `certificate`, `artifact`, `admission`, `journal`, `resolved`, `tickets`, `completions`.
- Init action: `Init` creates finite action IDs, contract states, unresolved journal, no runnable admission, and no completions.
- Next/actions: `Validate`, `Compile`, `EmitCertificate`, `AdmitArtifact`, `ScheduleAction`, `CompleteAction`, `RetryAction`, `ReplayJournal`, `Reject`.
- State constraints: finite bounded action set, finite ticket set, finite completion digest set, finite retry count for TLC exploration.
- Symmetry sets: action IDs and run IDs are symmetric; digest values may be symmetric except equality/distinctness.
- Bounded model limits: at least 3 actions, 2 runs, 2 tickets per action, 3 retry/replay steps, and 2 digest values to expose stale/conflicting completion cases.

## Properties

- Safety invariants:
  - `NoRejectedEffectScheduled`: an action with a rejected decision is never externally scheduled in retry/replay.
  - `CertificateSound`: certificate idempotency evidence contains only actions accepted by the decision table.
  - `AdmissionRequiresEvidence`: runnable admission implies passed idempotency evidence and compatible proof schema.
  - `ResolvedActionMonotonic`: once resolved, an action/step cannot move back to scheduled for a non-idempotent effect.
  - `DuplicateCompletionSameDigestOnly`: duplicate completion is accepted only when ticket/key and digest match the recorded completion.
- Liveness/eventuality:
  - `EventuallyAdmittedOrRejected`: after validation and certificate emission, every artifact eventually reaches admitted or rejected.
  - `EventuallyReplaySettles`: replay of finite durable journal eventually settles without scheduling rejected effects.
- Fairness assumptions: weak fairness on `Validate`, `Compile`, `EmitCertificate`, `AdmitArtifact`, `ReplayJournal`, and `Reject` when enabled; no fairness on unsafe external completion arrivals.
- Deadlock freedom: TLC must report no deadlock under bounded finite retry/replay constraints; terminal admitted/rejected states are modeled as stuttering states.
- Refinement to Rust/runtime behavior: Rust validation/compile/certificate/admission events refine the named actions by action ID, run ID, contract decision, idempotency evidence, ticket key, and completion digest.

## Evidence Command

- Preferred exact command after model files exist: `tlc -config specs/idempotency_gate/IdempotencyGate.cfg specs/idempotency_gate/IdempotencyGate.tla`
- If repo proof lane owns TLA+ execution: `moon run :verify-proof`
- State 3 status: model files do not yet exist; proof-writer must create them before formal-verifier can execute the TLC command.

## Waivers

- None for temporal behavior. Retry/replay/admission lifecycle requires TLA+ coverage.
