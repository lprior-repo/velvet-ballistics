# State 13 Assurance Bundle - vb-0253.5

STATUS: APPROVED

## Requirement Trace

- INV-001: eight state variants. Evidence: Rust source, Verus model, TLA state set, scoped tests.
- INV-002: valid transition matrix. Evidence: Kani parity, Verus all-pairs clauses, TLA `IsValidTransition`, Rust tests.
- INV-003: terminal states block outward transitions. Evidence: Kani cover/assertion, Verus `proof_terminal_blocks_outward`, TLA `TerminalStateBlocksOutwardTransitions`, runtime tests.

## Raw Gate Evidence

- `proof-evidence.md` contains Kani, Verus, TLA, and Rust test raw summaries.
- `machine-gate-report.md` contains machine gate outcomes and the formatting drift classification.
- `verification-ledger.jsonl` maps obligations to command evidence and status.

## Residual Risks

- Direct Verus verification of `crates/vb_proof_kernels/src/step_state.rs` is not claimed.
- Full workspace formatting remains red for unrelated files. This is global drift, not a vb-0253.5 local blocker.

## Decision

The StepState contract is bookmark-ready.
