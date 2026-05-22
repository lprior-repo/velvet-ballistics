# Assurance Bundle — vb-c1s0

bead_id: vb-c1s0
bead_title: bdd: Orchestration runtime acceptance scenarios
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/vb-c1s0-workspace
commit_or_change: vb_c1s0_orchestration_runtime_tests.rs (29 tests)
updated_at: 2026-05-20T00:10:00Z

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|-------------|----------------|---------------------|-----------------|--------|
| B1: Routing to correct shard | POST-001 | runtime_routes_run_to_correct_shard_by_run_id_modulo | test-suite-review.md (APPROVED) | ✅ PASS |
| B2: Same RunId → same shard | POST-001 | same_run_id_routes_to_same_shard_always | test-suite-review.md (APPROVED) | ✅ PASS |
| C1: Run reaches Finished | POST-002 | run_reaches_finished_state_when_workflow_complete | test-suite-review.md (APPROVED) | ✅ PASS |
| C2: Run reaches Failed | POST-002 | run_reaches_failed_state_when_action_fails | test-suite-review.md (APPROVED) | ✅ PASS |
| C3: Run reaches Cancelled | POST-002 | run_reaches_cancelled_state_when_cancel_called | test-suite-review.md (APPROVED) | ✅ PASS |
| C4: Terminal run ignores commands | POST-002 | terminal_run_ignores_subsequent_commands | test-suite-review.md (APPROVED) | ✅ PASS |
| D1: Action completion resumes | POST-003 | action_completion_resumes_at_correct_step_when_valid_ticket | test-suite-review.md (APPROVED) | ✅ PASS |
| D2: Invalid ticket error | POST-003 | complete_action_returns_invalid_ticket_error_when_ticket_unknown | test-suite-review.md (APPROVED) | ✅ PASS |
| D3: Fail action → Failed | POST-003 | fail_action_transitions_run_to_failed_state | test-suite-review.md (APPROVED) | ✅ PASS |
| E: Timer Authority | PRE-004, POST-004 | TimerWheel unit tests + TLA-WF-004 + Kani TIMER-001 | proof-review.md (APPROVED) | ✅ PASS |
| G1: tick_all → one cmd/shard | POST-005 | tick_all_processes_at_most_one_command_per_shard | test-suite-review.md (APPROVED) | ✅ PASS |
| G2: tick_all → false on shutdown | POST-005 | tick_all_returns_false_when_any_shard_shutting_down | test-suite-review.md (APPROVED) | ✅ PASS |
| G3: tick_all → true when alive | POST-005 | tick_all_returns_true_when_all_shards_alive | test-suite-review.md (APPROVED) | ✅ PASS |
| G4: FIFO order | POST-005 | shard_commands_processed_in_fifo_order + unit test L2 | test-suite-review.md (APPROVED) | ✅ PASS |
| H1: Budget respects step_budget | INV-006 | runtime_respects_step_budget_per_tick | test-suite-review.md (APPROVED) | ✅ PASS |
| H2: Budget try_take | INV-006 | step_budget_decrements_correctly_on_each_step | test-suite-review.md (APPROVED) | ✅ PASS |
| I1: answer_ask → correct shard | INV-001 | answer_ask_enqueues_to_correct_run_shard | test-suite-review.md (APPROVED) | ✅ PASS |
| I2: answer_ask → RunNotFound | INV-001 | answer_ask_returns_run_not_found_for_terminal_run | test-suite-review.md (APPROVED) | ✅ PASS |
| J1: tick_shard Continue | PRE-005 | tick_shard_continue_directive_processes_command | test-suite-review.md (APPROVED) | ✅ PASS |
| J2: tick_shard Shutdown | PRE-005 | tick_shard_shutdown_directive_returns_false | test-suite-review.md (APPROVED) | ✅ PASS |
| J3: tick_shard → ShardNotFound | PRE-005 | tick_shard_returns_shard_not_found_for_invalid_index | test-suite-review.md (APPROVED) | ✅ PASS |
| J4: tick_shard Migrate | PRE-005 | tick_shard_migrate_directive_transfers_commands | test-suite-review.md (APPROVED) | ✅ PASS |
| J5: migrate_shard → MigrateSelf | PRE-005 | migrate_shard_to_self_returns_migrate_self_error | test-suite-review.md (APPROVED) | ✅ PASS |
| K1: snapshot_run → ShardNotFound | INV-001 | snapshot_run_returns_shard_not_found_for_invalid_run | test-suite-review.md (APPROVED) | ✅ PASS |
| K2: snapshot_run → NotFound | INV-001 | snapshot_run_returns_not_found_for_unknown_run | test-suite-review.md (APPROVED) | ✅ PASS |
| K4: admission rejected | PRE-002 | submit_direct_returns_admission_rejected_for_missing_capability | test-suite-review.md (APPROVED) | ✅ PASS |
| K5: tick_all false after shutdown | POST-005 | tick_all_returns_false_after_graceful_shutdown | test-suite-review.md (APPROVED) | ✅ PASS |

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|------------|------|---------|----------|--------|--------|
| TLA-SHARD-ALL | TLA+ TLC | tla2tools.jar model check | tla-spec.md, proof-evidence.md | PASS | None |
| TLA-WF-TIMER | TLA+ + Kani | TLC + Kani harness TIMER-001 | proof-evidence.md | PASS | None |
| TLA-BUDGET | TLA+ | TLC model check | proof-evidence.md | PASS | None |

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|-----------|---------|----------|--------|
| 29 integration tests | cargo nextest run --test vb_c1s0_orchestration_runtime_tests | crates/workspace_tests/tests/vb_c1s0_orchestration_runtime_tests.rs | ✅ 29 PASS |
| Build | cargo build --package velvet-ballastics-workspace-tests | (no file output) | ✅ SUCCESS |
| Format | cargo fmt --check | (no file output) | ✅ SUCCESS |
| Clippy | cargo clippy | (pre-existing workspace issues) | ⚠ PRE-EXISTING |

## Review Evidence

| Review | Artifact | Status | Findings |
|--------|----------|--------|----------|
| Proof Review | proof-review.md | APPROVED | All proof obligations approved |
| Contract Verification | contract-verification-review.md | APPROVED | Contract adequate |
| Test Plan Review | test-plan-review.md | APPROVED (attempt 3/7) | J2 fixed, K3 removed acceptable |
| Test Suite Review | test-suite-review.md | APPROVED (attempt 3/7) | 29 tests, all tiers PASS |
| Black-Hat Review | black-hat-review.md | APPROVED | Coverage adequate, gaps documented |
| Formal Verification | formal-verification-report.md | PASS | 28/28 obligations PASS |

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|------|--------|-------|-----------------|----------------------|
| K3: timer_entry_fired integration test | Structural bug (finished_workflow has no timers) | N/A | N/A | TimerWheel unit tests + TLA-WF-004 + Kani TIMER-001 |
| Clippy: panic! in Result functions | Workspace-wide test pattern | N/A | Pre-existing | Not introduced by vb-c1s0 |
| D2: Ok(()) fallback | Contract gap documented | N/A | N/A | Exact error variant asserted |
| FIFO push_front/push_back gap | L1 integration + L2 unit test | N/A | N/A | Compensating evidence exists |

## Truth Serum Audit

- report: `.beads/vb-c1s0/truth-serum-report.md`
- status: See final-evidence-decision.md
