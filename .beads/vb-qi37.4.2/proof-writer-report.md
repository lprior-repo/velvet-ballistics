# Proof Writer Report: vb-qi37.4.2

## Scope

- Role: State 5 proof-writer specialist.
- Workspace: `/home/lewis/src/vb-femdation/vb-qi37-4-2`.
- Inputs read: `/home/lewis/.agents/skills/proof-writer/SKILL.md`, `contract.md`, `proof-strategy.md`, `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, `proof-plan-review-input.md`, `traceability-matrix.jsonl`.
- Production behavior edits: none.
- Forbidden agents invoked: none.

## Changed Verification Artifacts

| Artifact | Obligation IDs | Change |
|---|---|---|
| `verification/verus/taint_lattice.rs` | VB-CORE-TAINT-001 through VB-CORE-TAINT-006 | Corrected obligation header to include VB-CORE-TAINT-006. Existing proof body already discharged six taint lattice laws. |
| `verification/verus/step_state_machine.rs` | VB-CORE-STATE-001-VERUS | Corrected obligation header to planned ID. |
| `verification/verus/step_budget.rs` | VB-CORE-BUDGET-003-VERUS | Corrected obligation header to planned ID. |
| `verification/verus/run_frame_invariant.rs` | VB-CORE-RUNFRAME-001, VB-CORE-RUNFRAME-002, VB-CORE-RUNFRAME-003 | Added standalone Verus model for RunFrame constructor preconditions, constructor postconditions, and reinitialize dimension immutability. |
| `verification/verus/signals_invariant.rs` | VB-CORE-SIGNAL-001 | Replaced stale StepBudget proof with EngineSignal Finished canonical payload proof. |
| `verification/tla/LifecycleJournal.tla` | VB-REPLAY-001, VB-REPLAY-002, VB-REPLAY-003 | Added finite journal/replay model. |
| `verification/tla/LifecycleJournal.cfg` | VB-REPLAY-001, VB-REPLAY-002, VB-REPLAY-003 | Added TLC constants and invariant checks. |
| `verification/tla/RetryFSM.tla` | VB-REPLAY-004, VB-REPLAY-005 | Added bounded retry/backoff FSM model. |
| `verification/tla/RetryFSM.cfg` | VB-REPLAY-004, VB-REPLAY-005 | Added TLC constants, invariants, and finite time constraint. |
| `verification/tla/CapabilityLifecycle.tla` | VB-REPLAY-006, VB-REPLAY-007 | Replaced unrelated capability admission model with ownership/access model for planned obligations. |
| `verification/tla/CapabilityLifecycle.cfg` | VB-REPLAY-006, VB-REPLAY-007 | Added TLC constants and invariant checks. |
| `verification/tla/ConcurrencyControl.tla` | VB-CONC-001 through VB-CONC-005 | Added bounded shard/frame/lock model. |
| `verification/tla/ConcurrencyControl.cfg` | VB-CONC-001 through VB-CONC-005 | Added TLC constants, invariants, and finite wait-queue constraint. |
| `.beads/vb-qi37.4.2/proof-evidence.md` | all touched IDs | Added command evidence, assumptions, bounds, and status ledger. |
| `.beads/vb-qi37.4.2/proof-writer-report.md` | all touched IDs | This report. |

## Verification Summary

| Lane | Commands | Result |
|---|---:|---|
| Verus L4 | 6 | PASS, all exit 0 |
| TLA+ L3 | 4 | PASS, all exit 0 after repairs and finite bounds |
| Kani L3 | 0 | NOT_RUN, no Kani artifacts changed in this pass |
| Proptest/Differential L1 State 5 rows | 5 | 1 PASS, 4 BLOCKED_ARTIFACT_MISSING due exact planned filters selecting zero tests |
| Fuzz L2 | 0 | NOT_RUN, no fuzz artifacts changed in this pass |
| Loom L3 | 0 | NOT_RUN, no Loom artifacts changed in this pass |
| Static-scan L0 | 0 | NOT_RUN, no static-scan artifact changed in this pass |

## Exact Passing Commands

- `verus verification/verus/taint_lattice.rs` exited 0: `verification results:: 13 verified, 0 errors`.
- `verus verification/verus/signals_invariant.rs` exited 0: `verification results:: 3 verified, 0 errors`.
- `verus verification/verus/step_state_machine.rs` exited 0: `verification results:: 9 verified, 0 errors`.
- `verus verification/verus/step_budget.rs` exited 0: `verification results:: 6 verified, 0 errors`.
- `verus verification/verus/run_frame_invariant.rs` exited 0: `verification results:: 6 verified, 0 errors`.
- `verus verification/verus/resource_budget.rs` exited 0: `verification results:: 10 verified, 0 errors`.
- `tlc -config verification/tla/LifecycleJournal.cfg verification/tla/LifecycleJournal.tla` exited 0: no invariant violations, 941 generated states, 277 distinct states, depth 10.
- `tlc -config verification/tla/RetryFSM.cfg verification/tla/RetryFSM.tla` exited 0: no invariant violations, 83 generated states, 63 distinct states, depth 18.
- `tlc -config verification/tla/CapabilityLifecycle.cfg verification/tla/CapabilityLifecycle.tla` exited 0: no invariant violations, 81 generated states, 25 distinct states, depth 5.
- `tlc -config verification/tla/ConcurrencyControl.cfg verification/tla/ConcurrencyControl.tla` exited 0: no invariant violations, 1,195,009 generated states, 64,512 distinct states, depth 10.

## State 5 Non-Verus/TLA Command Evidence

| Obligation ID | Command | Exit | Classification | Evidence / limitation | Expiry / follow-up |
|---|---|---:|---|---|---|
| VB-CORE-STATE-003 | `cargo nextest run -p vb_core step_state_invalid` | 4 | BLOCKED_ARTIFACT_MISSING | Exact planned filter selected 0 tests across 9 binaries, 1795 skipped; nextest reported `error: no tests to run`. | Expires when a State 5/7 test writer adds or renames an executable `step_state_invalid` test/filter, then rerun exact command. |
| VB-CORE-RESOURCE-004-PROP | `cargo nextest run -p vb_core resource_policy` | 4 | BLOCKED_ARTIFACT_MISSING | Exact planned filter selected 0 tests across 9 binaries, 1795 skipped; nextest reported `error: no tests to run`. | Expires when an executable `resource_policy` property test/filter exists, then rerun exact command. |
| VB-EXPR-001 | `cargo nextest run -p vb_expr ast_bytecode_equiv` | 4 | BLOCKED_ARTIFACT_MISSING | Exact planned filter selected 0 tests across 1 binary, 339 skipped; nextest reported `error: no tests to run`. | Expires when an executable `ast_bytecode_equiv` differential test/filter exists, then rerun exact command. |
| VB-UI-MODEL-envelope-001 | `cargo nextest run -p vb_ui_model envelope_` | 0 | PASS | 18 tests run, 18 passed, 28 skipped. | None. |
| VB-UI-MODEL-envelope-002 | `cargo nextest run -p vb_ui_model serde_json_` | 4 | BLOCKED_ARTIFACT_MISSING | Exact planned filter selected 0 tests across 1 binary, 46 skipped; nextest reported `error: no tests to run`. | Expires when an executable `serde_json_` property test/filter exists, then rerun exact command. |

## Tooling

- `which verus` exited 0: `/home/lewis/.local/bin/verus`.
- `which tlc` exited 0: `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`.
- `which java` exited 0: `/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java`.
- `cargo kani --version` exited 0: `cargo-kani 0.67.0`.
- `cargo fuzz --version` exited 0: `cargo-fuzz 0.13.1`.
- `command -v verusfmt` exited 1; `verusfmt --check verification/verus/run_frame_invariant.rs` was not run because `verusfmt` is not installed/discoverable.
- Trusted-boundary scan `rg -n 'assume\(|#\[verifier::external_body\]|#\[verifier::external\]|axiom' verification/verus --glob '*.rs'` found no matches in Verus artifacts.
- Blocked required tooling: none. Optional `verusfmt` was unavailable.

## Assumptions And Reviewer Notes

- Verus artifacts are standalone proof models aligned to proof-kernel/contract semantics; they do not link against production crates.
- `verification/verus/run_frame_invariant.rs` models RunFrame dimensions as bounded integers and constructor defaults as abstract predicates; it proves the PRE-001, POST-001, and INV-007 proof-kernel obligations without accessing private production fields.
- TLA+ cfgs check safety invariants only. Temporal properties are present where planned, but this pass does not claim liveness proof evidence because the exact planned evidence text names invariant violations for these rows and no fairness cfg was added.
- `RetryFSM` uses `MaxRetries = 3` and `MaxTime = 6` for finite TLC exploration.
- `ConcurrencyControl` uses three shards, five frames, two resources, two machines, and `MaxQueue = 2`.
- `CapabilityLifecycle` treats `accessLog` as active access records; release is disabled while an active access exists for that capability.
- Existing old `CapabilityLifecycle*` cfg files were not edited; the planned command uses the new `CapabilityLifecycle.cfg`.
- The workspace root is not a Git repository from this path; `git status --short` failed with `fatal: not a git repository`.

## Remaining Planned Work

- Write or select Kani harness artifacts for VB-CORE-TAINT-006-KANI, VB-CORE-STATE-001-KANI, VB-CORE-STATE-002, VB-CORE-BUDGET-001, VB-CORE-BUDGET-002, VB-CORE-BUDGET-003-KANI, VB-CORE-IDX-001, VB-CORE-RESOURCE-004, VB-IPC-DECODE-001 through VB-IPC-DECODE-003, VB-STORAGE-DECODE-001 through VB-STORAGE-DECODE-005, and VB-EXPR-002.
- Write or select executable proptest/differential artifacts or correct exact filters for VB-CORE-STATE-003, VB-CORE-RESOURCE-004-PROP, VB-EXPR-001, and VB-UI-MODEL-envelope-002. VB-UI-MODEL-envelope-001 passed with the existing exact filter.
- Write or select fuzz artifacts for VB-IPC-DECODE-FUZZ, VB-STORAGE-DECODE-006, and VB-EXPR-003.
- Write or select Loom artifact for VB-CONC-LOOM.
- Later states must execute GATE-001 and GATE-002; this report does not claim gate completion.
