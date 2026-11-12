# Proof Repair Guide: vb-core-atomic-admission

State 6 attempt 3 rejected repaired State 5 proof artifacts.

## Required Repair

1. Repair `TLA-ATOM-001` restart/readback coverage.

Required options:

- Preferred: extend `verification/tla/AtomicAcceptedRunAdmission.tla` with an explicit restart state/action and a post-restart readback decision that depends only on durable records.
- Add an invariant or temporal property that proves restart/readback determinism, for example that after restart a committed durable full record set reads as accepted and failed/non-durable or partial durable states do not read as accepted.
- Add the new invariant/property to `verification/tla/AtomicAcceptedRunAdmission.cfg`.
- Keep deadlock checking enabled; do not add `CHECK_DEADLOCK FALSE`.
- Rerun `tlc -config verification/tla/AtomicAcceptedRunAdmission.cfg verification/tla/AtomicAcceptedRunAdmission.tla` and record raw output in `.beads/vb-core-atomic-admission/proof-evidence.md`.

Alternative only if contract owners agree:

- Narrow `TLA-ATOM-001`, `verification-layers.md`, and traceability so State 5 does not claim restart determinism in TLA+.
- Move restart/readback determinism entirely to later integration/formal-verifier obligations with explicit owner, expiry, and compensating evidence.

## Required Mapping Repair

2. Explain or strengthen the record-family abstraction.

Required options:

- Add a clear refinement note mapping `RecordKinds` members to source, artifact, header, `RunAccepted`, status index, workflow index, and action index.
- Show how `AllRecordsOrNoAcceptedRun`, `IndexesOnlyCommitted`, `NoPartialAfterFailure`, and `ReadbackOnlyCommitted` cover the planned per-family contract clauses.
- If the abstraction is insufficient, model the per-family variables/actions directly.

## Rerun Targets

- `pwd -P`
- Artifact existence and JSONL validation checks for State 5/6 inputs.
- `tlc -config verification/tla/AtomicAcceptedRunAdmission.cfg verification/tla/AtomicAcceptedRunAdmission.tla`
- `verus verification/verus/accepted_run_atomic_admission.rs`
- Discovery scan for `Restart|restart|restarted|readback|CHECK_DEADLOCK|PROPERTY|WF_vars` across the TLA+ files and proof evidence.

After repair, rerun State 6 proof review attempt 4.
