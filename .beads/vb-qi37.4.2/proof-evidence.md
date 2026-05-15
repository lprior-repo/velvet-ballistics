# Proof Evidence: vb-qi37.4.2

State 5 proof-writer evidence for verification artifacts only. Production behavior was not edited.

## Artifact Evidence

| Obligation IDs | Artifact paths | Status |
|---|---|---|
| VB-CORE-TAINT-001, VB-CORE-TAINT-002, VB-CORE-TAINT-003, VB-CORE-TAINT-004, VB-CORE-TAINT-005, VB-CORE-TAINT-006 | `verification/verus/taint_lattice.rs` | PASS |
| VB-CORE-SIGNAL-001 | `verification/verus/signals_invariant.rs` | PASS |
| VB-CORE-STATE-001-VERUS | `verification/verus/step_state_machine.rs` | PASS |
| VB-CORE-BUDGET-003-VERUS | `verification/verus/step_budget.rs` | PASS |
| VB-CORE-RUNFRAME-001, VB-CORE-RUNFRAME-002, VB-CORE-RUNFRAME-003 | `verification/verus/run_frame_invariant.rs` | PASS |
| VB-CORE-RESOURCE-001, VB-CORE-RESOURCE-002, VB-CORE-RESOURCE-003 | `verification/verus/resource_budget.rs` | PASS |
| VB-REPLAY-001, VB-REPLAY-002, VB-REPLAY-003 | `verification/tla/LifecycleJournal.tla`, `verification/tla/LifecycleJournal.cfg` | PASS |
| VB-REPLAY-004, VB-REPLAY-005 | `verification/tla/RetryFSM.tla`, `verification/tla/RetryFSM.cfg` | PASS |
| VB-REPLAY-006, VB-REPLAY-007 | `verification/tla/CapabilityLifecycle.tla`, `verification/tla/CapabilityLifecycle.cfg` | PASS |
| VB-CONC-001, VB-CONC-002, VB-CONC-003, VB-CONC-004, VB-CONC-005 | `verification/tla/ConcurrencyControl.tla`, `verification/tla/ConcurrencyControl.cfg` | PASS |

## Tool Discovery

| Command | Exit | Evidence |
|---|---:|---|
| `which verus` | 0 | `/home/lewis/.local/bin/verus` |
| `which tlc` | 0 | `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc` |
| `which java` | 0 | `/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java` |
| `cargo kani --version` | 0 | `cargo-kani 0.67.0` |
| `cargo fuzz --version` | 0 | `cargo-fuzz 0.13.1` |
| `command -v verusfmt` | 1 | `verusfmt` not installed/discoverable; `verusfmt --check verification/verus/run_frame_invariant.rs` was not run. |
| `rg -n 'assume\(|#\[verifier::external_body\]|#\[verifier::external\]|axiom' verification/verus --glob '*.rs'` | 1 | No matches found in Verus artifacts. |

## Passing Commands

| Command | Exit | Relevant output |
|---|---:|---|
| `verus verification/verus/taint_lattice.rs` | 0 | `verification results:: 13 verified, 0 errors` |
| `verus verification/verus/signals_invariant.rs` | 0 | `verification results:: 3 verified, 0 errors` |
| `verus verification/verus/step_state_machine.rs` | 0 | `verification results:: 9 verified, 0 errors` |
| `verus verification/verus/step_budget.rs` | 0 | `verification results:: 6 verified, 0 errors` |
| `verus verification/verus/run_frame_invariant.rs` | 0 | `verification results:: 6 verified, 0 errors`; explicit status capture rerun printed `EXIT_STATUS=0`. |
| `verus verification/verus/resource_budget.rs` | 0 | `verification results:: 10 verified, 0 errors` |
| `tlc -config verification/tla/LifecycleJournal.cfg verification/tla/LifecycleJournal.tla` | 0 | `Model checking completed. No error has been found.` 941 states generated, 277 distinct states, depth 10. |
| `tlc -config verification/tla/RetryFSM.cfg verification/tla/RetryFSM.tla` | 0 | `Model checking completed. No error has been found.` 83 states generated, 63 distinct states, depth 18. |
| `tlc -config verification/tla/CapabilityLifecycle.cfg verification/tla/CapabilityLifecycle.tla` | 0 | `Model checking completed. No error has been found.` 81 states generated, 25 distinct states, depth 5. |
| `tlc -config verification/tla/ConcurrencyControl.cfg verification/tla/ConcurrencyControl.tla` | 0 | `Model checking completed. No error has been found.` 1,195,009 states generated, 64,512 distinct states, depth 10. |
| `cargo nextest run -p vb_ui_model envelope_` | 0 | 18 tests run, 18 passed, 28 skipped; explicit status capture printed `EXIT_STATUS=0`. |

## Blocked / Waiver Evidence For State 5 Non-Verus/TLA Rows

These entries do not weaken the contract. They classify exact planned State 5 command rows that were executable but selected no test artifacts in the current workspace.

| Obligation ID | owner_state | rerun_from | Command | Exit | Classification | Limitation | Expiry / follow-up |
|---|---:|---:|---|---:|---|---|---|
| VB-CORE-STATE-003 | 3 | 5 | `cargo nextest run -p vb_core step_state_invalid` | 4 | BLOCKED_ARTIFACT_MISSING | Exact filter selected 0 tests across 9 binaries, 1795 skipped; `error: no tests to run`. | Add or rename executable `step_state_invalid` coverage, then rerun exact command before approval. |
| VB-CORE-RESOURCE-004-PROP | 5 | 5 | `cargo nextest run -p vb_core resource_policy` | 4 | BLOCKED_ARTIFACT_MISSING | Exact filter selected 0 tests across 9 binaries, 1795 skipped; `error: no tests to run`. | Add or rename executable `resource_policy` coverage, then rerun exact command before approval. |
| VB-EXPR-001 | 5 | 5 | `cargo nextest run -p vb_expr ast_bytecode_equiv` | 4 | BLOCKED_ARTIFACT_MISSING | Exact filter selected 0 tests across 1 binary, 339 skipped; `error: no tests to run`. | Add or rename executable `ast_bytecode_equiv` differential coverage, then rerun exact command before approval. |
| VB-UI-MODEL-envelope-002 | 5 | 5 | `cargo nextest run -p vb_ui_model serde_json_` | 4 | BLOCKED_ARTIFACT_MISSING | Exact filter selected 0 tests across 1 binary, 46 skipped; `error: no tests to run`. | Add or rename executable `serde_json_` coverage, then rerun exact command before approval. |

## Failed Or Repaired Attempts

| Command | Exit/status | Evidence | Repair |
|---|---:|---|---|
| `verus verification/verus/signals_invariant.rs` | nonzero | Rust type inference error for ambiguous `None` in `signals_invariant.rs`. | Replaced `None` with typed `Option::<SpecSlotValue>::None` and `Option::<SpecTaint>::None`; rerun passed. |
| `tlc -config verification/tla/RetryFSM.cfg verification/tla/RetryFSM.tla` | nonzero | `Invariant BackoffDurationPositive is violated` after `Tick` reached `now = backoffUntil` while still in `backoff`. | Repaired `EndBackoff` and `Tick` guards so state exits backoff at the bound; rerun then hit unbounded state growth. |
| `tlc -config verification/tla/RetryFSM.cfg verification/tla/RetryFSM.tla` | timeout after 120000 ms | 101,282,757 states generated, 75,962,026 distinct states, 5 states left on queue. | Added finite TLC time bound `MaxTime = 6` via `BoundedState`; rerun passed. |
| `tlc -config verification/tla/CapabilityLifecycle.cfg verification/tla/CapabilityLifecycle.tla` | nonzero | Semantic errors: dependent quantifier references `m` in `c \in held[m]`. | Split dependent quantifiers into nested `\E`; rerun found access-log invariant issue. |
| `tlc -config verification/tla/CapabilityLifecycle.cfg verification/tla/CapabilityLifecycle.tla` | nonzero | `Invariant ValidCapabilityAccess is violated` after releasing a capability with a historical access-log entry. | Added release precondition forbidding release while active access entries refer to that capability; rerun passed. |
| `tlc -config verification/tla/ConcurrencyControl.cfg verification/tla/ConcurrencyControl.tla` | nonzero | Semantic errors: dependent quantifier references `s` and `from`. | Split dependent quantifiers into nested `\E`; rerun then timed out due unbounded wait queue. |
| `tlc -config verification/tla/ConcurrencyControl.cfg verification/tla/ConcurrencyControl.tla` | timeout after 120000 ms | 70,024,953 states generated, 7,011,405 distinct states, 3,510,313 states left on queue. | Added finite `MaxQueue = 2` wait-queue bound via `BoundedState`; rerun passed. |
| `tlc -config ...` for RetryFSM, CapabilityLifecycle, ConcurrencyControl in parallel | nonzero | TLC timestamped `states/26-05-15-14-18-26` directory collision. | Reran exact TLC commands sequentially. |
| `verus ...; status=$?; ...` status-capture wrappers | nonzero | zsh reports `read-only variable: status`. | Replaced shell variable with `rc`; reruns passed with `EXIT_STATUS=0`. |

## Assumptions And Bounds

| Artifact | Assumption/bound |
|---|---|
| `verification/verus/taint_lattice.rs` | Models `Taint` as a closed three-value lattice with ranks Clean=0, DerivedFromSecret=1, Secret=2. No I/O, async, storage, or wall-clock behavior modeled. |
| `verification/verus/signals_invariant.rs` | Models the Rust enum shape only: `Finished(SpecSlotValue, SpecTaint)` exists only with both payload components. It does not prove runtime constructors beyond the closed enum representation. |
| `verification/verus/step_state_machine.rs` | Models closed `StepState` variants and the planned valid transition matrix. No dispatcher side effects modeled. |
| `verification/verus/step_budget.rs` | Models `remaining` and `requested` as bounded integers in `[0, u64::MAX]`, matching no-underflow arithmetic obligations. |
| `verification/verus/run_frame_invariant.rs` | Models RunFrame constructor/reinitialize proof kernel with dimensions as bounded integers. Constructor defaults are represented as abstract predicates (`all_states_pending`, `all_slots_empty`, `all_taint_clean`); no production private fields are accessed. |
| `verification/verus/resource_budget.rs` | Models resource dimensions as bounded integers in `[0, u64::MAX]` and composition with saturating add/multiply plus max. |
| `verification/tla/LifecycleJournal.cfg` | TLC finite model: `ActionIds = {a1, a2, a3}`, `MaxSeq = 3`; no liveness properties checked in cfg. |
| `verification/tla/RetryFSM.cfg` | TLC finite model: `MaxRetries = 3`, `MaxTime = 6`; `BoundedState` constrains time and backoff deadline. No liveness property checked in cfg. |
| `verification/tla/CapabilityLifecycle.cfg` | TLC finite model: two machines and two capabilities. Access log represents active access records; release is disabled while an access record for the capability remains. |
| `verification/tla/ConcurrencyControl.cfg` | TLC finite model: three shards, five frames, two resources, two machines, `MaxQueue = 2`. Temporal liveness formulae are present in the module but not checked in cfg because fairness/liveness checking was not in the exact planned command rows' invariant evidence. |

## Untouched Planned Obligations

Kani, fuzz, static-scan, Loom, and gauntlet rows in `proof-obligations.planned.jsonl` were not modified or executed in this State 5 pass because no corresponding harness/fuzz/static artifacts were written here. State 5 proptest/differential rows were executed above: one passed and four are blocked by missing exact-filter test artifacts.

No required verifier tooling was blocked: Verus, TLC, Java, Kani, and cargo-fuzz were discoverable. Optional `verusfmt` was not discoverable.

## State 5 Attempt 4 Repair Evidence

### Repaired exact nextest filters

| Obligation ID | Command | Exit | Evidence |
|---|---|---:|---|
| VB-CORE-STATE-003 | `cargo nextest run -p vb_core step_state_invalid` | 0 | 1 test run: 1 passed, 1796 skipped |
| VB-CORE-RESOURCE-004-PROP | `cargo nextest run -p vb_core resource_policy` | 0 | 1 test run: 1 passed, 1796 skipped |
| VB-EXPR-001 | `cargo nextest run -p vb_expr ast_bytecode_equiv` | 0 | 1 test run: 1 passed, 339 skipped |
| VB-UI-MODEL-envelope-002 | `cargo nextest run -p vb_ui_model serde_json_` | 0 | 1 test run: 1 passed, 46 skipped |

### Repaired TLA property/deadlock selection

| Obligation IDs | Command | Exit | Evidence |
|---|---|---:|---|
| VB-REPLAY-002 | `tlc -config verification/tla/LifecycleJournal.cfg verification/tla/LifecycleJournal.tla` | 0 | `PROPERTY EventuallyReplayComplete`; `CHECK_DEADLOCK TRUE`; no error; 941 states generated, 277 distinct |
| VB-REPLAY-004 | `tlc -config verification/tla/RetryFSM.cfg verification/tla/RetryFSM.tla` | 0 | `PROPERTY EventuallyExhaustedOrDone`; `CHECK_DEADLOCK TRUE`; no error; 83 states generated, 63 distinct |
| VB-CONC-003, VB-CONC-004, VB-CONC-005 | `tlc -config verification/tla/ConcurrencyControl.cfg verification/tla/ConcurrencyControl.tla` | 0 | `PROPERTY NoDeadlockOnLocks`, `PROPERTY NoStarvation`, `PROPERTY LockNoStarvation`; `CHECK_DEADLOCK TRUE`; no error; 275457 states generated, 15360 distinct |

### Still-blocking required lanes

| Obligation IDs | Command | Exit/status | Classification | Evidence |
|---|---|---:|---|---|
| VB-CORE-STATE-001-KANI, VB-CORE-STATE-002 | `cargo kani --harness kani_step_state` | nonzero | REQUIRED_OBLIGATION_FAIL | `error: no harnesses matched the harness filter: kani_step_state`; `cargo kani list` reports 0 standard harnesses |
| VB-EXPR-003 | `cargo fuzz run expr_eval -- -runs=1000` | nonzero | FAIL_LOCAL_TOOLCHAIN | `sanitizer is incompatible with statically linked libc` for target `x86_64-unknown-linux-musl` |
| VB-STORAGE-DECODE-006 | `cargo fuzz run decode_record -- -runs=1000` | nonzero | FAIL_LOCAL_ARTIFACT | `no bin target named decode_record`; available fuzz target uses different name |
| VB-CONC-LOOM | `RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime bounded_queue` | nonzero | FAIL_LOCAL | missing `Arc` imports in `timer_fired_cancel.rs` and `shutdown_drain.rs` under cfg loom |

State 5 attempt 4 repairs the prior zero-test and TLA cfg blockers, but State 6 approval remains blocked by unexecuted or failing required Kani/fuzz/Loom obligations.

## State 5 Current-Session Evidence Repair

Current-session rerun used non-`/tmp` build/temp locations to remove the prior Kani disk-quota blocker.

| Obligation IDs | Command | Exit | Evidence |
|---|---|---:|---|
| VB-CORE-STATE-001-KANI, VB-CORE-STATE-002 | `TMPDIR=/home/lewis/src/tmp_build/vb-qi37.4.2-kani CARGO_TARGET_DIR=/home/lewis/src/tmp_build/vb-qi37.4.2-cargo-target SCCACHE_DIR=/home/lewis/src/tmp_build/vb-qi37.4.2-sccache SCCACHE_TMPDIR=/home/lewis/src/tmp_build/vb-qi37.4.2-kani RUSTC_WRAPPER= cargo kani -p vb_core --harness kani_step_state` | 0 | `.beads/vb-qi37.4.2/kani-report-current-session.md`: `VERIFICATION:- SUCCESSFUL`; `0 of 293 failed (65 unreachable)`; `Complete - 1 successfully verified harnesses, 0 failures, 1 total.`; `EXIT_STATUS=0` |
| VB-EXPR-003 | `cargo fuzz run expr_eval --target x86_64-unknown-linux-gnu -- -runs=1000` | 0 | `.beads/vb-qi37.4.2/fuzz-expr-eval-report.md`: `#1000 DONE`; `EXIT_STATUS=0` |
| VB-STORAGE-DECODE-006 | `cargo fuzz run decode_record --target x86_64-unknown-linux-gnu -- -runs=1000` | 0 | `.beads/vb-qi37.4.2/fuzz-decode-record-report.md`: `#1000 DONE`; `EXIT_STATUS=0` |
| VB-CONC-LOOM | `RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime bounded_queue` | 0 | `.beads/vb-qi37.4.2/loom-report.md`: `2 passed, 1467 filtered out`; `EXIT_STATUS=0` |

The previous Kani `/tmp` disk-quota blocker is repaired in the current session. Fuzz/Loom reports are validated from existing raw artifacts; fuzz run counts remain at 1000 runs and must be judged by State 6 against contract thresholds.
