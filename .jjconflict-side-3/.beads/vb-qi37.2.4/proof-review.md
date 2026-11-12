# Proof Review: vb-qi37.2.4

STATUS: APPROVED

## Decision
- The State 5 proof artifacts are approved for the proof-loop gate: TLA+ and Verus artifacts are executable and passed direct reviewer reruns.
- PR-004 is repaired: `VERUS-AGG-001` and `VERUS-DIAG-001` now have executable State 5 rows in `proof-obligations.jsonl` and traceability entries.
- Prior rejection treated State 7 and State 12 obligations as State 6 blockers. That was over-strict for the go-skill state boundary: `KANI-BUD-001`, `PROP-BUD-001`, and `PROP-DIAG-001` are valid downstream State 7 test/proof-realization obligations, and `GATE-BUD-001`/`GATE-BUD-002` are State 12 formal execution rollup obligations. They are not waived and must remain blocking at their owner states if unsatisfied.

## Files Reviewed
- `.beads/vb-qi37.2.4/contract.md`
- `.beads/vb-qi37.2.4/tla-spec.md`
- `.beads/vb-qi37.2.4/lean-contract.md`
- `.beads/vb-qi37.2.4/verification-layers.md`
- `.beads/vb-qi37.2.4/proof-obligations.jsonl`
- `.beads/vb-qi37.2.4/traceability-matrix.jsonl`
- `.beads/vb-qi37.2.4/proof-strategy.md`
- `.beads/vb-qi37.2.4/proof-writer-report.md`
- `.beads/vb-qi37.2.4/proof-evidence.md`
- `specs/tla/BoundedAdmission.tla`
- `specs/tla/BoundedAdmission.cfg`
- `verification/verus/budget_bounded.rs`
- `crates/vb_core/src/budget.rs`

## Command Evidence
- `pwd -P` -> `/home/lewis/src/vb-femdation/vb-qi37-2-4`.
- `test -s` required review artifacts -> pass.
- `jq -c . .beads/vb-qi37.2.4/proof-obligations.jsonl >/dev/null` -> pass.
- `jq -c . .beads/vb-qi37.2.4/traceability-matrix.jsonl >/dev/null` -> pass.
- `verus verification/verus/budget_bounded.rs` -> `verification results:: 15 verified, 0 errors`.
- `tlc -config specs/tla/BoundedAdmission.cfg specs/tla/BoundedAdmission.tla` -> no errors, `108977 states generated`, `9762 distinct states found`, `0 states left on queue`, complete depth `9`.
- `moon run :verify-proof` -> downstream State 12 gate evidence: exit code 2 before proof execution because `scripts/rust-verification-gauntlet.sh` has `//!` doc-comment syntax parsed by bash.
- State 5 repair check: `jq -c . .beads/vb-qi37.2.4/proof-obligations.jsonl >/dev/null` and `jq -c . .beads/vb-qi37.2.4/traceability-matrix.jsonl >/dev/null` -> pass after adding `VERUS-AGG-001` and `VERUS-DIAG-001`.
- State 5 repair rerun: `verus verification/verus/budget_bounded.rs` -> `verification results:: 15 verified, 0 errors`.
- State 5 repair rerun: `tlc -config specs/tla/BoundedAdmission.cfg specs/tla/BoundedAdmission.tla` -> no errors, `108977 states generated`, `9762 distinct states found`, `0 states left on queue`, complete depth `9`.
- State 6 rerun: `moon run :verify-proof` -> exit code 2, `scripts/rust-verification-gauntlet.sh: line 3: //!: No such file or directory`, syntax error at line 7; recorded as State 12 `BLOCKED_TOOLING`, not a State 5/6 proof-artifact rejection.
- Current orchestrator rerun: `verus verification/verus/budget_bounded.rs` -> `verification results:: 15 verified, 0 errors`.
- Current orchestrator rerun: `tlc -config specs/tla/BoundedAdmission.cfg specs/tla/BoundedAdmission.tla` -> no errors, `108977 states generated`, `9762 distinct states found`, `0 states left on queue`, complete depth `9`.
- Current orchestrator rerun: `moon run :verify-proof` -> exit code 2, same bash parsing failure in `scripts/rust-verification-gauntlet.sh`; carried to State 12.

## Findings
- `PR-001` DOWNSTREAM: `GATE-BUD-001` is required but `BLOCKED_TOOLING`; owner_state `12`, rerun_from `12`. This blocks State 12/formal execution if unrepaired, not State 6 approval of direct proof artifacts.
- `PR-002` DOWNSTREAM: `KANI-BUD-001` is required but `BLOCKED_SCOPE`; owner_state `7`, rerun_from `7`. This must be planned/written as a downstream test/proof-realization obligation.
- `PR-003` DOWNSTREAM: `PROP-BUD-001` and `PROP-DIAG-001` are required but `BLOCKED_SCOPE`; owner_state `7`, rerun_from `7`. These must be planned/written as downstream property obligations.
- `PR-004` RESOLVED: `VERUS-AGG-001` and `VERUS-DIAG-001` are now executable rows in `proof-obligations.jsonl` and mapped in `traceability-matrix.jsonl`; State 5 repair accepted.

## Obligation Decision
- `TLA-ADM-001`: PASS, direct TLC rerun accepted within bounded model scope.
- `TLA-ADM-002`: PASS, direct TLC rerun accepted within bounded model scope.
- `VERUS-BUD-001`: PASS, direct Verus rerun accepted for abstract checked sequential composition.
- `VERUS-BUD-002`: PASS, direct Verus rerun accepted for abstract nested finite multiplication, unknown factor rejection, and overflow rejection.
- `VERUS-BUD-003`: PASS, direct Verus rerun accepted for abstract branch max and together fanout bounds.
- `VERUS-AGG-001`: PASS, direct Verus rerun accepted for abstract aggregate-from-verified-whole refinement; runtime realization remains covered by `PROP-BUD-001`/`GATE-BUD-001`.
- `VERUS-DIAG-001`: PASS, direct Verus rerun accepted for abstract diagnostic projection totality; runtime diagnostic parity remains covered by `PROP-DIAG-001`.
- `KANI-BUD-001`: DOWNSTREAM_REQUIRED, owner_state `7`, rerun_from `7`; required concrete Rust bounded overflow/rejection harness remains mandatory before implementation/formal acceptance.
- `PROP-BUD-001`: DOWNSTREAM_REQUIRED, owner_state `7`, rerun_from `7`; required runtime/property realization remains mandatory before implementation/formal acceptance.
- `PROP-DIAG-001`: DOWNSTREAM_REQUIRED, owner_state `7`, rerun_from `7`; required diagnostic parity remains mandatory before implementation/formal acceptance.
- `GATE-BUD-001`: DOWNSTREAM_REQUIRED_TOOLING_REPAIR, owner_state `12`, rerun_from `12`; required proof rollup must execute in State 12 after tooling repair.
- `GATE-BUD-002`: DOWNSTREAM_REQUIRED, owner_state `12`, rerun_from `12`; cannot be counted as evidence until State 12.

## Residual Risk / Downstream Handoff
- The Verus file is an abstract proof surface and does not import production functions from `crates/vb_core/src/budget.rs`; it supports the arithmetic shape but does not prove concrete Rust realization by itself.
- The TLA+ model checks admission ordering, not the concrete budget computation or diagnostic renderer.
- Runtime diagnostic fields for primitive/node/structural path/actual/limit remain unproven until State 7 property coverage exists.
- State 7 must encode `KANI-BUD-001`, `PROP-BUD-001`, and `PROP-DIAG-001` in `test-plan.md` and downstream test artifacts; missing coverage remains `REQUIRED_OBLIGATION_FAIL` there.
- State 12 must repair `scripts/rust-verification-gauntlet.sh` and rerun `moon run :verify-proof`; the current bash parse failure remains `BLOCKED_TOOLING` there.
