# Proof Plan Review Input: vb-qi37.5 State 4 Attempt 3

STATUS: READY_FOR_PROOF_PLAN_REVIEW

## Review Focus

- Verify `proof-obligations.planned.jsonl` is traceable to repaired State 3 clauses and State 6 rejection repairs.
- Reject the plan if parity obligations can still pass by tautology or excluded cases.
- Reject the plan if TLA+ evidence can still be produced with deadlock checking disabled.
- Reject the plan if certificate summary soundness can still be count-only or detached from action identifiers.
- Confirm `FUZZ-ARTIFACT-011` is now executable and no longer a blocked placeholder.
- Confirm `TEST-COMPLETION-015` gives implementation-realization coverage for POST-006 beyond TLA+.

## Repaired Planning Deltas

- `FUZZ-ARTIFACT-011` uses exact command `cargo fuzz run admission_fuzz -- -runs=1000`.
- `TEST-COMPLETION-015` is carried forward as required runtime realization evidence for same-ticket/key same-digest collapse and stale/conflicting rejection.
- `TLA-RETRY-001`, `TLA-REPLAY-002`, and `TLA-ADMIT-003` expected evidence now requires deadlock-enabled TLC evidence and expanded duplicate-completion bounds.
- `VERUS-PARITY-002` and `KANI-PARITY-006` explicitly reject the prior tautology/exclusion approach.
- `VERUS-CERT-003` requires identifier/set-based certificate soundness, not count-only proof.

## Commands Planned For Later States

- `tlc -config specs/idempotency_gate/IdempotencyGate.cfg specs/idempotency_gate/IdempotencyGate.tla`
- `verus verification/verus/idempotency_decision.rs`
- `verus verification/verus/idempotency_certificate_summary.rs`
- `verus verification/verus/idempotency_replay_tracker.rs`
- `cargo kani -p vb_validate`
- `cargo kani -p vb_compile`
- `cargo test -p vb_compile --test idempotency_parity`
- `cargo test -p vb_storage admission`
- `cargo test -p vb_runtime admission`
- `cargo fuzz run admission_fuzz -- -runs=1000`
- `cargo miri test -p vb_storage recovery`
- `moon run :lint-src`
- `moon run :verify-proof`
- `moon run :mutants-smoke`

## Non-Executable Rows

- `WAIVER-THEOREM-016`: waived theorem-kernel lane, row 19.
- `NA-LOOM-017`: concurrency model checking not applicable for current scope, row 20.
- `NA-FLUX-018`: Flux not applicable and unavailable in prior discovery, row 21.
- `NA-SUPPLY-019`: dependency/supply-chain lane not applicable unless dependency/config files change, row 22.

## Blockers

- None at planning time. Later states may record `blocked_tooling` if required tools are missing when executing exact commands.
