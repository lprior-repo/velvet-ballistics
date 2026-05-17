bead_id: vb-qi37.15.2
bead_title: cli: Add submit command and job ledger
phase: State 3
updated_at: 2026-05-11T00:00:00Z

# TLA+ Temporal Model Plan

## Boundary
- Temporal behavior: submit request moves through ReadInputs -> Compile -> PersistWorkflow -> PersistRunHeader -> AppendRunSubmitted -> ReportSuccess.
- External systems abstracted: filesystem and Fjall journal writes are success/failure actions.

## Model Shape
- Module/model path: `specs/SubmitLedger.tla` (planned; not present in repo during State 3).
- Variables: `phase`, `workflowPersisted`, `runHeaderPersisted`, `submittedEventPersisted`, `reportedSuccess`.
- Init: all persisted/reported flags false, phase `Start`.
- Actions: `ReadInputs`, `Compile`, `PersistWorkflow`, `PersistRunHeader`, `AppendSubmitted`, `ReportSuccess`, `Fail`.

## Properties
- Safety: `reportedSuccess => workflowPersisted /\\ runHeaderPersisted` and durable modes imply `submittedEventPersisted`.
- Liveness: if all writes succeed, eventually `reportedSuccess`.
- Deadlock freedom: bounded model has terminal Success or Fail.
- Refinement: Rust `cmd_submit` write calls refine persist actions in source order.

## Evidence Command
- Planned: `moon run :verify-proof` once SubmitLedger spec exists; until then formal-verifier records waiver/deferred proof setup if not available.
