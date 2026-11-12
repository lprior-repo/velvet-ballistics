# TLA+ Temporal Model Plan - vb-qi37.12.2

STATUS: PLANNED

## Boundary

- Temporal/workflow behavior: resume state transitions across `Resumable`, `Resuming`, `Resumed`, restore-on-failed-append, and returned error classification.
- Rust/core behavior excluded from TLA+: concrete Rust error enum layout, storage implementation internals, and public semver checking.
- External systems abstracted: durable journal append as either success or failure; resume drive as either success or failure.

## TLA+-Owned Clauses

- R2/INV-001: failed resume drive or append never returns successful `Resumed`.
- R3/INV-002: failed `Resumed` append restores visible state to `Resumable`.
- R5e: append failure fallback is deterministic when no source carrier exists.

## Model Shape

- Module/model path: `specs/vb_qi37_12_2_resume.tla` (to be created by proof-writer if this bead adds formal model code).
- Config path: `specs/vb_qi37_12_2_resume.cfg`.
- Variables: `state`, `appendResult`, `driveResult`, `returned`, `sourceCarrier`, `sourceBound`.
- Init action: `InitResumable`.
- Next/actions: `BeginResume`, `DriveSucceeds`, `DriveFails`, `AppendResumedSucceeds`, `AppendResumedFailsRestore`, `ReturnError`, `ReturnResumed`.
- State constraints: finite enum values for state, append/drive result, return value, source-carrier mode, and source-bound flag.
- Symmetry sets: none.
- Bounded model limits: single run id; all nondeterministic drive/append outcomes explored.

## Properties

- Safety invariants: `NoFalseResumedSuccess`, `FailedAppendRestoresResumable`, `DeterministicJournalAppendFallback`, `NoSourceClaimWithoutCarrier`.
- Liveness/eventuality: every started resume attempt eventually returns `Resumed` or a typed error in the bounded model.
- Fairness assumptions: weak fairness on return actions when enabled.
- Deadlock freedom: TLC must report no deadlock under the config bounds.
- Refinement to Rust/runtime behavior: a Rust call to `handle_resume` refines one resume attempt; durable append failure maps to `AppendResumedFailsRestore`; returned unit `ResumeError::JournalAppendFailed` maps to typed fallback with `sourceCarrier = none` and must not assert `sourceBound = TRUE`.

## Evidence Command

- If model files are introduced: `tlc -config specs/vb_qi37_12_2_resume.cfg specs/vb_qi37_12_2_resume.tla`
- If repo verification lane owns TLA+ discovery: `moon run :verify-proof`

## Waivers

- No TLA+ code is authored by State 3. If State 4 declines to create this small workflow model, it must record an explicit waiver and rely on focused state-machine tests for R2/R3/R5e.
