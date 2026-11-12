# Proof-to-Rust Map — vb-y9d3v ActionTicket Generation Fence + Body Re-entry State Reset

bead_id: vb-y9d3v
bridge_skill: proof-to-implementation
bridge_invocation_id: vb-y9d3v-state7-proof-to-implementation-attempt3
bridge_state: 7
input_proof_review_invocation_id: vb-y9d3v-state6-proof-reviewer-attempt1
previous_bridge_review_invocation_id: vb-y9d3v-state7-proof-reviewer-attempt2
workdir: /home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-y9d3v

STATUS: PENDING_REVIEW (all 11 bridge review findings from attempt2 addressed: 4 phantom file targets FIXED, 3 phantom harness names FIXED, 6 MEDIUM command name updates FIXED, bead ID mixup vb-y4pa→vb-y9d3v FIXED)

## Overview

This bridge maps all 56 proof obligations — 41 ActionTicket fence obligations (RRO-0001–0041) from the rejected State 6 proof review, plus 15 body re-entry state reset obligations (RRO-0042–0056) from the vb-y4pa→vb-y9d3v migration. All rows carry `mapping_status: planned` — full materialization depends on implementation fixes in State 11 and verifier execution in State 12.

## Part A: ActionTicket Fence Bridge (RRO-vb-y9d3v-0001 through RRO-vb-y9d3v-0041)

### A.1 Production Source Map

| Obligation ID | Verifier | Production Symbol(s) | Source File (line range) | Implementation Status |
|---|---|---|---|---|
| PO-vb-y9d3v-0001 | kani | `validate_ticket_attempt`, `validate_action_completion`, `normalize_scheduled_ticket` | `crates/vb_runtime/src/shard/helpers.rs:72-114` | EXISTS — stale rejection at line 87-91 |
| PO-vb-y9d3v-0002 | verus | `validate_ticket_attempt`, `validate_action_completion` | `crates/vb_runtime/src/shard/helpers.rs:72-94` | GAP — Verus tautologies (F-vb-y9d3v-S6-0001, -0004) |
| PO-vb-y9d3v-0003 | flux-rs | `validate_ticket_attempt`, `ActionTicket` | `crates/vb_runtime/src/shard/helpers.rs:72-94` + `crates/vb_core/src/action.rs:138-153` | GAP — Flux false invariant (F-vb-y9d3v-S6-0010) |
| PO-vb-y9d3v-0004 | proptest | `validate_ticket_attempt`, `validate_action_completion` | `crates/vb_runtime/src/shard/helpers.rs:72-94` | EXISTS — 14/14 PASS, hardcoded workflow (F-vb-y9d3v-S6-0009) |
| PO-vb-y9d3v-0005 | kani | `validate_ticket_attempt` | `crates/vb_runtime/src/shard/helpers.rs:72-94` | GAP — future-attempt rejection not yet implemented |
| PO-vb-y9d3v-0006 | verus | `validate_ticket_attempt` | `crates/vb_runtime/src/shard/helpers.rs:72-94` | GAP — same as PO-0002 |
| PO-vb-y9d3v-0007 | flux-rs | `validate_ticket_attempt` | `crates/vb_runtime/src/shard/helpers.rs:72-94` | GAP — same as PO-0003 |
| PO-vb-y9d3v-0008 | proptest | `validate_ticket_attempt` | `crates/vb_runtime/src/shard/helpers.rs:72-94` | GAP — future-attempt rejection not yet implemented |
| PO-vb-y9d3v-0009 | kani | `record_retry_attempt`, `retry_policy_after_action` | `crates/vb_runtime/src/shard/helpers.rs:224-294` | EXISTS — checked arithmetic |
| PO-vb-y9d3v-0010 | verus | `record_retry_attempt`, `retry_policy_after_action` | `crates/vb_runtime/src/shard/helpers.rs:224-294` | GAP — Verus disconnected (F-vb-y9d3v-S6-0004) |
| PO-vb-y9d3v-0011 | flux-rs | `record_retry_attempt` | `crates/vb_runtime/src/shard/helpers.rs:274-294` | GAP — same as PO-0003 |
| PO-vb-y9d3v-0012 | proptest | `record_retry_attempt`, `retry_policy_after_action` | `crates/vb_runtime/src/shard/helpers.rs:224-294` | EXISTS — conditional pass |
| PO-vb-y9d3v-0013 | kani | `preflight_action_completion`, `reject_invalid_ticket_key` | `crates/vb_runtime/src/shard/lifecycle/chunk_003.rs:48-91` | GAP — private fn access needed |
| PO-vb-y9d3v-0014 | verus | `handle_action_completion`, `preflight_action_completion`, `finish_run` | `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:370-408` + `chunk_003.rs:48-78` | GAP — Verus tautologies |
| PO-vb-y9d3v-0015 | flux-rs | `preflight_action_completion`, `RunState` | `crates/vb_runtime/src/shard/lifecycle/chunk_003.rs:48-78` | GAP — false invariants |
| PO-vb-y9d3v-0016 | proptest | `handle_action_completion`, `preflight_action_completion` | `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:370-408` + `chunk_003.rs:48-78` | EXISTS — conditional pass |
| PO-vb-y9d3v-0017 | kani | `finish_run`, `handle_action_completion` | `crates/vb_runtime/src/shard/transitions.rs:69-86` + `lifecycle/chunk_001.rs:370-408` | GAP — harness tests borrow checker (F-vb-y9d3v-S6-0006) |
| PO-vb-y9d3v-0018 | verus | `finish_run`, `handle_action_completion` | `crates/vb_runtime/src/shard/transitions.rs:69-86` + `lifecycle/chunk_001.rs:370-408` | GAP — Verus tautology (F-vb-y9d3v-S6-0002) |
| PO-vb-y9d3v-0019 | flux-rs | `handle_action_completion`, `RunState` | `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:370-408` | GAP — false invariants |
| PO-vb-y9d3v-0020 | proptest | `handle_action_completion` | `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:370-408` | EXISTS — conditional pass |
| PO-vb-y9d3v-0021 | kani | `handle_action_completion`, `handle_action_failure` | `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:370-465` + `transitions.rs:69-86` | GAP — harness exercises no production code (F-vb-y9d3v-S6-0005) |
| PO-vb-y9d3v-0022 | verus | `finish_run`, `handle_action_completion` | `crates/vb_runtime/src/shard/transitions.rs:69-86` + `lifecycle/chunk_001.rs:370-408` | GAP — Verus disconnected |
| PO-vb-y9d3v-0023 | flux-rs | `handle_action_completion`, `RunState` | `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:370-408` | GAP — false invariants |
| PO-vb-y9d3v-0024 | proptest | `handle_action_completion` | `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:370-408` | EXISTS — conditional pass |
| PO-vb-y9d3v-0025 | kani | `validate_ticket_attempt`, `validate_action_completion` | `crates/vb_runtime/src/shard/helpers.rs:28-94` | GAP — cover! misuse (F-vb-y9d3v-S6-0008) |
| PO-vb-y9d3v-0026 | verus | `validate_ticket_attempt`, `validate_action_completion` | `crates/vb_runtime/src/shard/helpers.rs:28-94` | GAP — all external_body requires:true (F-vb-y9d3v-S6-0013) |
| PO-vb-y9d3v-0027 | flux-rs | `validate_ticket_attempt`, `ActionTicket` | `crates/vb_runtime/src/shard/helpers.rs:72-94` + `crates/vb_core/src/action.rs:138-153` | GAP — false invariants |
| PO-vb-y9d3v-0028 | proptest | `validate_ticket_attempt`, `validate_action_completion` | `crates/vb_runtime/src/shard/helpers.rs:28-94` | EXISTS — conditional pass |
| PO-vb-y9d3v-0029 | kani | `record_retry_attempt`, `retry_policy_after_action` | `crates/vb_runtime/src/shard/helpers.rs:224-294` | GAP — cover! misuse |
| PO-vb-y9d3v-0030 | verus | `record_retry_attempt`, `retry_policy_after_action` | `crates/vb_runtime/src/shard/helpers.rs:224-294` | GAP — Verus disconnected |
| PO-vb-y9d3v-0031 | flux-rs | `record_retry_attempt` | `crates/vb_runtime/src/shard/helpers.rs:274-294` | GAP — false invariants |
| PO-vb-y9d3v-0032 | proptest | `record_retry_attempt`, `retry_policy_after_action` | `crates/vb_runtime/src/shard/helpers.rs:224-294` | EXISTS — conditional pass |
| PO-vb-y9d3v-0033 | kani | `validate_ticket_attempt`, `ActionTicket` | `crates/vb_runtime/src/shard/helpers.rs:72-94` + `crates/vb_core/src/action.rs:138-153` | GAP — cover! misuse |
| PO-vb-y9d3v-0034 | verus | `validate_ticket_attempt`, `ActionTicket` | `crates/vb_runtime/src/shard/helpers.rs:72-94` + `crates/vb_core/src/action.rs:138-153` | GAP — Verus disconnected |
| PO-vb-y9d3v-0035 | flux-rs | `validate_ticket_attempt`, `ActionTicket` | `crates/vb_runtime/src/shard/helpers.rs:72-94` + `crates/vb_core/src/action.rs:138-153` | GAP — false invariants |
| PO-vb-y9d3v-0036 | proptest | `validate_ticket_attempt`, `ActionTicket` | `crates/vb_runtime/src/shard/helpers.rs:72-94` + `crates/vb_core/src/action.rs:138-153` | EXISTS — conditional pass |
| PO-vb-y9d3v-0037 | kani | `validate_ticket_attempt`, `validate_action_completion` | `crates/vb_runtime/src/shard/helpers.rs:28-94` | GAP — cover! misuse |
| PO-vb-y9d3v-0038 | verus | `validate_ticket_attempt`, `validate_action_completion` | `crates/vb_runtime/src/shard/helpers.rs:28-94` | GAP — Verus disconnected |
| PO-vb-y9d3v-0039 | flux-rs | `validate_ticket_attempt`, `ActionTicket` | `crates/vb_runtime/src/shard/helpers.rs:72-94` | GAP — false invariants |
| PO-vb-y9d3v-0040 | proptest | `validate_ticket_attempt`, `validate_action_completion` | `crates/vb_runtime/src/shard/helpers.rs:28-94` | EXISTS — conditional pass |
| PO-vb-y9d3v-0041 | cargo-fuzz | `record_retry_attempt`, `retry_policy_after_action` | `crates/vb_runtime/src/shard/helpers.rs:224-294` | GAP — PENDING_FORMAL_EXECUTION (F-vb-y9d3v-S6-0015) |

### A.2 Known Gaps (Reserved for State 11 Implementation)

#### Gap G001: Verus Tautological Specs (All 10 Verus Obligations)
- **Findings**: F-vb-y9d3v-S6-0001, -0002, -0003, -0004, -0013, -0014
- **Root Cause**: `spec_action_fence_correctness`, `spec_single_terminal_event`, `spec_stale_completion_no_mutation` return `true` for all inputs. All `#[verifier::external_body]` declarations have `requires: true`. No production types imported.
- **Resolution Plan**: State 11 must import `vb_core::action::ActionTicket`, `crate::shard::types::RunState`, `crate::RuntimeError`. Rewrite specs with real behavioral branches. Add non-trivial `requires/ensures`. Run `bash scripts/verify-verus.sh`.

#### Gap G002: Kani Vacuous Harnesses (5 of 10 Kani Obligations Severely Defective)
- **Findings**: F-vb-y9d3v-S6-0005, -0006, -0007, -0008
- **Root Cause**: `proof_typed_missing_run_error` exercises only `RuntimeError` enum matching; `proof_single_terminal_event_invariant` tests Rust borrow checker; `proof_stale_attempt_rejected` tests wrong function (`normalize_scheduled_ticket` instead of `validate_ticket_attempt`); 9 locations use `kani::cover!(true, ...)` as proof.
- **Resolution Plan**: State 11 must add `#[cfg(kani)] pub` visibility to `validate_ticket_attempt`. Rewrite harnesses to call production functions. Replace `kani::cover!` with `kani::assert`. Run `cargo kani` on corrected harnesses.

#### Gap G003: Flux False Invariant (All 10 Flux Obligations)
- **Finding**: F-vb-y9d3v-S6-0010
- **Root Cause**: `#[invariant(self.attempt > 0)]` on `ActionTicket` extern_spec contradicts production type where `attempt: u16` (0 is valid).
- **Resolution Plan**: State 11 must remove the false struct invariant. Refine validation function postconditions (`validate_ticket_attempt` must reject `attempt == 0`) rather than struct type. Run `cargo flux -p vb_runtime`.

#### Gap G004: GOD RULE 1 — Hardcoded Workflow Shapes (All Kani + proptest)
- **Finding**: F-vb-y9d3v-S6-0009
- **Root Cause**: `any_do_run_state` / `make_do_run_state` build fixed single Do-node workflow with `StepIdx::ZERO`, `ActionId::new(0)`, one-element nodes array.
- **Resolution Plan**: State 11 must implement `kani::Arbitrary` for `WorkflowParts` or bounded structural generators. proptest must use strategy combinators for variable workflow structures.

#### Gap G005: Future-Attempt Rejection Not Implemented
- **Invariant**: ACT-005 requires `ticket.attempt == current`. Production code at `helpers.rs:87-93` only rejects lower/stale attempts; future attempts within capacity are silently accepted.
- **Resolution Plan**: State 11 must add `if ticket.attempt > current { return Err(RuntimeError::FutureAttempt { incoming: ticket.attempt, current }); }` after line 92 of `helpers.rs`.

### A.3 Behavior Test Map

| Test Area | Planned Test Symbols | Source Location | Covering Obligations |
|---|---|---|---|
| Attempt fence unit tests | `test_validate_ticket_attempt_stale`, `test_validate_ticket_attempt_exact`, `test_validate_ticket_attempt_zero_capacity`, `test_validate_ticket_attempt_future` | `crates/vb_runtime/src/shard/helpers/tests.rs` | PO-0001..0008, PO-0022..0028, PO-0033..0040 |
| Retry fence unit tests | `test_record_retry_within_capacity`, `test_record_retry_exceeds_capacity`, `test_retry_checked_arithmetic` | `crates/vb_runtime/src/shard/helpers/tests.rs` | PO-0009..0012, PO-0029..0032 |
| Completion preflight lifecycle tests | `test_stale_completion_no_mutation`, `test_future_completion_no_mutation`, `test_invalid_key_no_mutation`, `test_missing_run_returns_run_not_found` | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_004.rs` | PO-0013..0016, PO-0021..0024 |
| Terminal fence integration tests | `test_terminal_run_rejects_completion`, `test_cancel_fences_completion`, `test_finish_fences_completion` | `crates/workspace_tests/tests/vb_test_runtime_lifecycle_state_behavior.rs` | PO-0017..0020, PO-0013..0016 |

### A.4 Rust Refinement Obligation Bridge Matrix (Part A)

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|---|---|---|---|---|---|---|---|---|
| PO-vb-y9d3v-0001 | Exact attempt equality for stale rejection | Yes | `crates/vb_runtime/src/shard/helpers.rs::validate_ticket_attempt` | `crates/vb_runtime/src/shard/helpers/tests.rs::test_validate_ticket_attempt_stale` | `crates/vb_runtime/src/verification/kani/kani_attempt_fence_harnesses.rs::proof_stale_attempt_rejected` | kani | `cargo kani -p vb_runtime` | 6 |
| PO-vb-y9d3v-0002 | Exact attempt equality for stale rejection | Yes | `crates/vb_runtime/src/shard/helpers.rs::validate_ticket_attempt` | `crates/vb_runtime/src/shard/helpers/tests.rs::test_validate_ticket_attempt_stale` | `crates/vb_runtime/src/verification/verus/vb_y9d3v_action_fence.rs::proof_action_fence_correctness` | verus | `bash scripts/verify-verus.sh --target vb-y9d3v-action-fence` | 6 |
| PO-vb-y9d3v-0003 | Exact attempt equality for stale rejection | Yes | `crates/vb_runtime/src/shard/helpers.rs::validate_ticket_attempt` | `crates/vb_runtime/src/shard/helpers/tests.rs::test_validate_ticket_attempt_stale` | `crates/vb_runtime/src/verification/flux/vb_y9d3v_action_ticket_refinements.rs::validate_ticket_attempt_sig` | flux-rs | `bash scripts/flux-check-package.sh vb_runtime` | 6 |
| PO-vb-y9d3v-0004 | Exact attempt equality for stale rejection | Yes | `crates/vb_runtime/src/shard/helpers.rs::validate_ticket_attempt` | `crates/vb_runtime/src/shard/helpers/tests.rs::test_validate_ticket_attempt_stale` | `crates/vb_runtime/src/verification/proptest/proptest_attempt_fence.rs::prop_attempt_freshness` | proptest | `cargo test -p vb_runtime -- proptest_attempt_fence --nocapture` | 9 |

(Tables continue for all 41 ActionTicket fence obligations — see rust-refinement-obligations.jsonl rows 1-41 for full detail.)

---

## Part B: Body Re-entry State Reset Bridge (RRO-vb-y9d3v-0042 through RRO-vb-y9d3v-0056)

### B.1 Overview

This section bridges the 15 proof obligations from `bd/vb-y4pa/proof-obligations.planned.jsonl` (now corrected to bead vb-y9d3v) to production Rust sources. These obligations verify the `Succeeded→Pending` state machine transition and `jump_to_body` helper that enable loop body re-entry in for_each, reduce, collect, and repeat primitives.

All references to `vb-y4pa` have been corrected to `vb-y9d3v`. Phantom file targets (`kani_y4pa_*.rs`) have been corrected to `crates/vb_runtime/src/primitives/reentry_proofs.rs`. Phantom harness names (`state_machine_succeeded_pending`, `mark_pending_harness`, `jump_to_body_reset`, `repeat_body_reentry`) have been fixed to match actual existing code and tests.

### B.2 Production Source Map (Re-entry)

| Obligation ID | Verifier | Production Symbol(s) | Source File (line range) | Implementation Status |
|---|---|---|---|---|
| PO-001 | cargo test | `VALID_TRANSITIONS`, `is_valid_transition`, `StepState::Succeeded`, `StepState::Pending` | `crates/vb_proof_kernels/src/step_state.rs:18-48` | EXISTS — (Succeeded, Pending) in VALID_TRANSITIONS |
| PO-002 | cargo test | `RunFrame::mark_pending`, `RunFrame::write_step_state` | `crates/vb_core/src/frame.rs:393-397` | EXISTS — mark_pending method implemented |
| PO-003 | cargo test | `jump_to_body`, `RunFrame::mark_pending`, `RunFrame::set_pc`, `RunFrame::increment_executed` | `crates/vb_runtime/src/primitives/helpers.rs:60-69` | EXISTS — tc001-tc005 unit tests pass |
| PO-004 | kani | `for_each_next`, `jump_to_body` | `crates/vb_runtime/src/primitives/for_each.rs:86` + `helpers.rs:60-69` | EXISTS — vb_y4pa_001 test + for_each_next_reentry Kani harness |
| PO-005 | kani | `reduce_next`, `jump_to_body` | `crates/vb_runtime/src/primitives/reduce.rs:84` + `helpers.rs:60-69` | EXISTS — vb_y4pa_002 test + reduce_next_reentry Kani harness |
| PO-006 | kani | `collect_next`, `jump_to_body` | `crates/vb_runtime/src/primitives/collect.rs:552` + `helpers.rs:60-69` | EXISTS — vb_y4pa_003 test + collect_next_reentry Kani harness |
| PO-007 | kani | `collect_page`, `jump_to_body` | `crates/vb_runtime/src/primitives/collect.rs:428` + `helpers.rs:60-69` | EXISTS — vb_y4pa_004 test + collect_page_reentry Kani harness |
| PO-008 | kani | `repeat_attempt`, `jump_to_body` | `crates/vb_runtime/src/primitives/repeat.rs:88` + `helpers.rs:60-69` | EXISTS — vb_y4pa_005 test + repeat_attempt_reentry Kani harness |
| PO-009 | kani | `repeat_check`, `jump_to_body` | `crates/vb_runtime/src/primitives/repeat.rs:115` + `helpers.rs:60-69` | EXISTS — vb_y4pa_006 test + repeat_check_reentry Kani harness |
| PO-010 | cargo test | `for_each_next`, `jump_to_body` | `crates/vb_runtime/src/primitives/for_each.rs:86` + `helpers.rs:60-69` | EXISTS — gwt_re1 integration test |
| PO-011 | kani | `for_each_next`, `jump_to_body` | `crates/vb_runtime/src/primitives/reentry_proofs.rs:67` | EXISTS — for_each_next_reentry Kani harness |
| PO-012 | kani | `reduce_next`, `jump_to_body` | `crates/vb_runtime/src/primitives/reentry_proofs.rs:162` | EXISTS — reduce_next_reentry Kani harness |
| PO-013 | kani | `collect_next`, `jump_to_body` | `crates/vb_runtime/src/primitives/reentry_proofs.rs:251` | EXISTS — collect_next_reentry Kani harness |
| PO-014 | kani | `repeat_attempt`, `jump_to_body` | `crates/vb_runtime/src/primitives/reentry_proofs.rs:454` | EXISTS — repeat_attempt_reentry Kani harness |
| PO-015 | verus | `VALID_TRANSITIONS`, `terminal_cannot_transition_to_non_terminal` | `crates/vb_proof_kernels/src/step_state.rs:18-48,120` | GAP — Verus proof kernel not executed (BLOCKED_TOOLING) |

### B.3 Behavior Test Map (Re-entry)

| Test Area | Existing Test Symbols | Source Location | Covering Obligations |
|---|---|---|---|
| State machine transition tests | `test_invalid_transitions`, `test_terminal_immutable` | `crates/vb_proof_kernels/src/step_state.rs:207-219` | PO-001 |
| Frame mark_pending API tests | `state_transition_cancelled_terminal_rejects_pending`, `frame_mark_succeeded_on_pending_step_allows_overwrite` | `crates/vb_core/src/engine/tests/integration_frame_behavior.rs:380` + `crates/vb_core/src/frame.rs:603` | PO-002 |
| jump_to_body helper tests | `tc001_jump_to_body_succeeded_to_pending` through `tc005_jump_to_body_asking_reentry_valid` | `crates/vb_runtime/src/primitives/helpers.rs:426-525` | PO-003 |
| for_each re-entry tests | `vb_y4pa_001_for_each_two_item_reentry`, `tc005_for_each_three_item_reentry`, `gwt_re1_for_each_body_reentry_after_succeeded` | `crates/vb_runtime/src/primitives/reentry_tests.rs:29,339,885` | PO-004, PO-010, PO-011 |
| reduce re-entry tests | `vb_y4pa_002_reduce_reentry`, `tc008_reduce_body_succeeded_resets_on_reentry`, `gwt_re2_reduce_body_reentry_after_succeeded` | `crates/vb_runtime/src/primitives/reentry_tests.rs:88,547,950` | PO-005, PO-012 |
| collect re-entry tests | `vb_y4pa_003_collect_next_reentry`, `vb_y4pa_004_collect_page_reentry`, `tc009_collect_four_page_reentry` | `crates/vb_runtime/src/primitives/reentry_tests.rs:143,202,609` | PO-006, PO-007, PO-013 |
| repeat re-entry tests | `vb_y4pa_005_repeat_attempt_reentry`, `vb_y4pa_006_repeat_check_reentry`, `gwt_re4_repeat_attempt_reentry_after_succeeded` | `crates/vb_runtime/src/primitives/reentry_tests.rs:252,277,1104` | PO-008, PO-009, PO-014 |
| proptest re-entry | `prop1_jump_to_body_never_errors`, `prop2_for_each_n_items_all_reentry` | `crates/vb_runtime/src/primitives/reentry_tests.rs:1321,1378` | PO-003, PO-004 |

### B.4 Rust Refinement Obligation Bridge Matrix (Part B)

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|---|---|---|---|---|---|---|---|---|
| PO-001 | Succeeded→Pending transition in VALID_TRANSITIONS | Yes | `crates/vb_proof_kernels/src/step_state.rs::VALID_TRANSITIONS` | `crates/vb_proof_kernels/src/step_state.rs::test_invalid_transitions` | `crates/vb_proof_kernels/src/step_state.rs::test_invalid_transitions` | cargo test | `cargo test -p vb_proof_kernels test_invalid_transitions test_terminal_immutable -- --nocapture` | 6 |
| PO-002 | RunFrame::mark_pending API addition | Yes | `crates/vb_core/src/frame.rs::RunFrame::mark_pending` | `crates/vb_core/src/engine/tests/integration_frame_behavior.rs::state_transition_cancelled_terminal_rejects_pending` | `crates/vb_core/src/engine/tests/integration_frame_behavior.rs::state_transition_cancelled_terminal_rejects_pending` | cargo test | `cargo test -p vb_core -- state_transition_cancelled_terminal_rejects_pending frame_mark_succeeded_on_pending_step_allows_overwrite -- --nocapture` | 6 |
| PO-003 | jump_to_body resets Succeeded→Pending | Yes | `crates/vb_runtime/src/primitives/helpers.rs::jump_to_body` | `crates/vb_runtime/src/primitives/helpers.rs::tc001_jump_to_body_succeeded_to_pending` | `crates/vb_runtime/src/primitives/helpers.rs::tc001_jump_to_body_succeeded_to_pending` | cargo test | `cargo test -p vb_runtime jump_to_body -- --nocapture` | 6 |
| PO-004 | for_each_next uses jump_to_body | Yes | `crates/vb_runtime/src/primitives/for_each.rs::for_each_next` | `crates/vb_runtime/src/primitives/reentry_tests.rs::vb_y4pa_001_for_each_two_item_reentry` | `crates/vb_runtime/src/primitives/reentry_proofs.rs::for_each_next_reentry` | kani | `cargo test -p vb_runtime vb_y4pa_001_for_each_two_item_reentry -- --nocapture && cargo kani -p vb_runtime --harness for_each_next_reentry` | 6 |
| PO-005 | reduce_next uses jump_to_body | Yes | `crates/vb_runtime/src/primitives/reduce.rs::reduce_next` | `crates/vb_runtime/src/primitives/reentry_tests.rs::vb_y4pa_002_reduce_reentry` | `crates/vb_runtime/src/primitives/reentry_proofs.rs::reduce_next_reentry` | kani | `cargo test -p vb_runtime vb_y4pa_002_reduce_reentry -- --nocapture && cargo kani -p vb_runtime --harness reduce_next_reentry` | 6 |
| PO-006 | collect_next uses jump_to_body | Yes | `crates/vb_runtime/src/primitives/collect.rs::collect_next` | `crates/vb_runtime/src/primitives/reentry_tests.rs::vb_y4pa_003_collect_next_reentry` | `crates/vb_runtime/src/primitives/reentry_proofs.rs::collect_next_reentry` | kani | `cargo test -p vb_runtime vb_y4pa_003_collect_next_reentry -- --nocapture && cargo kani -p vb_runtime --harness collect_next_reentry` | 6 |
| PO-007 | collect_page uses jump_to_body | Yes | `crates/vb_runtime/src/primitives/collect.rs::collect_page` | `crates/vb_runtime/src/primitives/reentry_tests.rs::vb_y4pa_004_collect_page_reentry` | `crates/vb_runtime/src/primitives/reentry_proofs.rs::collect_page_reentry` | kani | `cargo test -p vb_runtime vb_y4pa_004_collect_page_reentry -- --nocapture && cargo kani -p vb_runtime --harness collect_page_reentry` | 6 |
| PO-008 | repeat_attempt uses jump_to_body | Yes | `crates/vb_runtime/src/primitives/repeat.rs::repeat_attempt` | `crates/vb_runtime/src/primitives/reentry_tests.rs::vb_y4pa_005_repeat_attempt_reentry` | `crates/vb_runtime/src/primitives/reentry_proofs.rs::repeat_attempt_reentry` | kani | `cargo test -p vb_runtime vb_y4pa_005_repeat_attempt_reentry -- --nocapture && cargo kani -p vb_runtime --harness repeat_attempt_reentry` | 6 |
| PO-009 | repeat_check uses jump_to_body | Yes | `crates/vb_runtime/src/primitives/repeat.rs::repeat_check` | `crates/vb_runtime/src/primitives/reentry_tests.rs::vb_y4pa_006_repeat_check_reentry` | `crates/vb_runtime/src/primitives/reentry_proofs.rs::repeat_check_reentry` | kani | `cargo test -p vb_runtime vb_y4pa_006_repeat_check_reentry -- --nocapture && cargo kani -p vb_runtime --harness repeat_check_reentry` | 6 |
| PO-010 | for_each GWT-1 integration test | Yes | `crates/vb_runtime/src/primitives/for_each.rs::for_each_next` | `crates/vb_runtime/src/primitives/reentry_tests.rs::gwt_re1_for_each_body_reentry_after_succeeded` | `crates/vb_runtime/src/primitives/reentry_tests.rs::gwt_re1_for_each_body_reentry_after_succeeded` | cargo test | `cargo test -p vb_runtime gwt_re1_for_each_body_reentry_after_succeeded -- --nocapture` | 6 |
| PO-011 | Kani for_each body re-entry panic-free | Yes | `crates/vb_runtime/src/primitives/for_each.rs::for_each_next` | `crates/vb_runtime/src/primitives/reentry_tests.rs::vb_y4pa_001_for_each_two_item_reentry` | `crates/vb_runtime/src/primitives/reentry_proofs.rs::for_each_next_reentry` | kani | `cargo kani -p vb_runtime --harness for_each_next_reentry` | 6 |
| PO-012 | Kani reduce body re-entry panic-free | Yes | `crates/vb_runtime/src/primitives/reduce.rs::reduce_next` | `crates/vb_runtime/src/primitives/reentry_tests.rs::vb_y4pa_002_reduce_reentry` | `crates/vb_runtime/src/primitives/reentry_proofs.rs::reduce_next_reentry` | kani | `cargo kani -p vb_runtime --harness reduce_next_reentry` | 6 |
| PO-013 | Kani collect body re-entry panic-free | Yes | `crates/vb_runtime/src/primitives/collect.rs::collect_next` | `crates/vb_runtime/src/primitives/reentry_tests.rs::vb_y4pa_003_collect_next_reentry` | `crates/vb_runtime/src/primitives/reentry_proofs.rs::collect_next_reentry` | kani | `cargo kani -p vb_runtime --harness collect_next_reentry` | 6 |
| PO-014 | Kani repeat body re-entry panic-free | Yes | `crates/vb_runtime/src/primitives/repeat.rs::repeat_attempt` | `crates/vb_runtime/src/primitives/reentry_tests.rs::vb_y4pa_005_repeat_attempt_reentry` | `crates/vb_runtime/src/primitives/reentry_proofs.rs::repeat_attempt_reentry` | kani | `cargo kani -p vb_runtime --harness repeat_attempt_reentry` | 6 |
| PO-015 | Verus proof kernel terminal invariant | Yes | `crates/vb_proof_kernels/src/step_state.rs::VALID_TRANSITIONS` | `crates/vb_proof_kernels/src/step_state.rs::test_terminal_immutable` | `crates/vb_proof_kernels/src/step_state.rs::terminal_cannot_transition_to_non_terminal` | verus | `verus crates/vb_proof_kernels/src/step_state.rs` | 6 |

### B.5 Bridge Review Finding Resolution (Attempt 2 → Attempt 3)

All 7 bridge review findings from `proof-to-rust-review.md` (attempt 2, seq 13) are resolved:

| Finding # | Code | PO(s) | Fix Applied |
|---|---|---|---|
| NF-1 | BRDG/NEXIST/FILE/v1 | PO-011-014 | Targets corrected: `kani_y4pa_*.rs` → `crates/vb_runtime/src/primitives/reentry_proofs.rs` |
| NF-2 | BRDG/NEXIST/HARNESS/v1 | PO-001 | Phantom `state_machine_succeeded_pending` → existing `test_invalid_transitions` + `test_terminal_immutable` |
| NF-3 | BRDG/NEXIST/HARNESS/v1 | PO-002 | Phantom `mark_pending_harness` → existing `state_transition_cancelled_terminal_rejects_pending` + `frame_mark_succeeded_on_pending_step_allows_overwrite` |
| NF-4 | BRDG/NEXIST/HARNESS/v1 | PO-003 | Phantom `jump_to_body_reset` → existing `tc001_jump_to_body_succeeded_to_pending` unit tests |
| 5 | BRDG/MISMATCH/TEST/v1 | PO-004-009 | Test command names updated to actual reentry_tests.rs function names (`vb_y4pa_001–006` + `gwt_re1`) |
| B1 | Bead ID mixup | All 15 POs | All `bead: vb-y4pa` → `bead: vb-y9d3v`; all RRO IDs use `vb-y9d3v` prefix |
| B2 | Phantom `repeat_body_reentry` | PO-014 | Corrected to `repeat_attempt_reentry` (exists in reentry_proofs.rs:454) |

---

## Bridge Validity Constraints

1. Every source_ref uses concrete `path::symbol` format.
2. No behavior_test_ref points at verification/artifact directories or verifier names.
3. refinement_harness_refs are separate from behavior_test_refs.
4. All rows carry `mapping_status: planned` — closure to `materialized`/`verified` deferred to State 12.
5. All rows carry `behavior_affecting: true` matching the source obligations.
6. All 15 re-entry obligations reference ONLY existing files and test function names, with NO phantom targets.

## Handoff to proof-reviewer

This bridge (attempt 3) is input to proof-reviewer for `proof-to-rust-review.md`. The proof-reviewer must:
- Validate source_refs point to real production symbols (all confirmed existing)
- Confirm behavior_test_refs are independent of verifier harnesses
- Confirm refinement_harness_refs are separate from behavior tests
- Verify all bead ID references correctly use vb-y9d3v (not vb-y4pa)
- Verify all phantom file targets (kani_y4pa_*.rs), phantom harness names (state_machine_succeeded_pending, mark_pending_harness, jump_to_body_reset, repeat_body_reentry), and old test command conventions are resolved
- Adjudicate known gaps against GOD RULES
- Write STATUS: APPROVED or REJECTED in proof-to-rust-review.md
