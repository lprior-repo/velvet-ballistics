# Slice 1 Test Suite Review — vb_core + vb_runtime — ROUND 2

STATUS: REJECTED

Round 1 closed 7 beads (vb-b9sab, vb-wuexb, vb-zc7vf, vb-tjo9t, vb-hnn9u, vb-2x3qk, vb-lynec) that addressed 24 defects, but **7 of the 10 CRITICAL round-1 fixes have been REVERTED or never landed**: `action_queue_tests.rs:240` still has `assert!(result.is_ok())` smoke; `lru_ring_red_queen_remove_props.rs:175` still has bare `assert!(r.is_err(), "must fail when full")` smoke; and `recovery_bdd_tests.rs:2141, 2728, 2843, 2852, 2883` all still smoke-test their public-API contracts. The 5 round-1 HIGH blockers that were dispositioned as `blocker` (frame_pool 8×, chunk_017 3×, chunk_dispatch_error_semantics OR-smoke, integration_frame_behavior 5×, together_tests 14×) are **all still REGRESSED**. Wave-5/6/7 commits introduced 4 NEW smoke patterns (properties_ticket_derivation, admission/step_budget_tests, chunk_032 standalone, integration_taint_propagation silent fall-through) and surfaced 2 NEW defective property tests (integration_error_routing_behavior:1608 weak proptest; integration_step_behavior:2015 silent `let _ = result`). One `#[ignore]` + `todo!()` in `arena_tests.rs:226` and one `thread::sleep` in production code at `action_queue/types.rs:125` are also flagged. Slice is REJECTED with 17 CRITICAL, 11 HIGH, 12 MEDIUM, 4 LOW, 6 OBSERVATION findings.

---

## 1. Round 1 Fix Verification Table

| # | Round-1 ID | Original fix location | Expected fix | Round 2 state | Evidence |
|---|------------|-----------------------|--------------|---------------|----------|
| 1 | F-01 | `crates/vb_runtime/src/engine/action_tests.rs:267` | `matches!(result, Err(ResolveError::IdMismatch { requested: ActionId(99), .. }))` | **STILL APPLIED** ✓ (improved: uses `assert_eq!` against `RuntimeEngineError::Action(ActionError::UnknownAction { action: ActionId::new(99) })`) | `action_tests.rs:267-275` |
| 2 | F-02 | `crates/vb_runtime/src/engine/action_tests.rs:289` | concrete `Ok(c) if c.id == ActionId::new(0)` | **STILL APPLIED** ✓ (`matches!(result, Ok(c) if c.id == ActionId::new(0) && c.id.get() == 0)` + `assert_eq!(result.map(|c| c.id), Ok(ActionId::new(0)))`) | `action_tests.rs:299-308` |
| 3 | F-03 | `crates/vb_runtime/src/engine/action_tests.rs:296` | concrete `Ok(c) if c.id == ActionId::new(2)` | **STILL APPLIED** ✓ (same pattern as F-02 for `ActionId::new(2)`) | `action_tests.rs:310-320` |
| 4 | F-04 | `crates/vb_runtime/src/action_queue/action_queue_tests.rs:240` | `assert_eq!(result, Ok(()))` | **REGRESSED** ✗ — still `assert!(result.is_ok());` standalone, no concrete payload, no post-state | `action_queue_tests.rs:240` |
| 5 | F-05 | `crates/vb_runtime/src/shard/lru_ring_red_queen_tests.rs:507` (file split to `lru_ring_red_queen_remove_props.rs:175`) | `matches!(r, Err(LruError::TerminalRunsLruFull { .. }))` | **REGRESSED** ✗ — file split but smoke retained: `assert!(r.is_err(), "must fail when full");` (no concrete variant) | `lru_ring_red_queen_remove_props.rs:175` |
| 6 | F-06 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2141` | `matches!(result, Err(RecoveryError::NoRecoveryData { run }))` | **REGRESSED** ✗ — `assert!(result.is_err(), "empty journal should return error");` with NO concrete variant. Note: line 1060 of same file already has the `matches!` pattern that should have been used here | `recovery_bdd_tests.rs:2141` |
| 7 | F-07 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2843` | `assert_eq!(result, Ok(()))` | **REGRESSED** ✗ — `assert!(result.is_ok(), "matching digests should succeed");` standalone | `recovery_bdd_tests.rs:2843` |
| 8 | F-08 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2852` | `matches!(result, Err(DigestMismatchError { expected, found }))` | **REGRESSED** ✗ — `assert!(result.is_err(), "mismatched digests should be rejected");` standalone | `recovery_bdd_tests.rs:2852` |
| 9 | F-09 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2883` | concrete post-conditions on `RecoverySummaryKind::Hydration` | **REGRESSED** ✗ — `assert!(result.is_ok(), "should return RecoveryHydration");` standalone (the entire test is 27 lines of fixture ending in this one-liner) | `recovery_bdd_tests.rs:2883` |
| 10 | F-10 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2728` | concrete `frame.pc()` + `frame.slot_count()` post-conditions | **REGRESSED** ✗ — `assert!(result.is_ok(), "tail after watermark should succeed");` standalone | `recovery_bdd_tests.rs:2728` |

**Summary of round-1 regressions: 7 of 10 CRITICAL fixes did not survive. The 3 that did (F-01, F-02, F-03) were improved.**

**Round-1 HIGH blockers that remain REGRESSED (informational):**
- F-13 (`frame_pool/tests.rs:147, 244, 259-261, 273, 274, 351`) — 8× `assert_eq!(reused.is_ok(), true)` REGRESSED
- F-14 (`shard/tests/chunk_017.rs:217-218, 220`) — 3× `assert!(f1/f2/f3.is_ok(), "BH-SHD-07: ...")` REGRESSED
- F-15 (`shard/tests/chunk_dispatch_error_semantics.rs:159`) — OR-conditioned smoke REGRESSED
- F-16 (`vb_core/engine/tests/integration_frame_behavior.rs:34, 83, 686, 695, 705`) — 5× `assert!(frame.is_ok())` REGRESSED
- F-17 (`vb_runtime/together_tests.rs:73, 255, 288, 505, 546, 592, 635, 968, 1016, 1059, 1106, 1216, 1420, 1533`) — 14× `assert!(run.add_parallel_in_flight(N).is_ok())` REGRESSED

---

## 2. Findings Table (ordered by severity)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix |
|----|-----|-----------|--------|------------------------------|-----------------|
| F2-01 | CRITICAL | `crates/vb_runtime/src/action_queue/action_queue_tests.rs:240` | ROUND 1 REGRESSION of F-04. `bounded_action_queue_enqueue_single_item_succeeds` ends `assert!(result.is_ok());` with no payload check. | Mutate `BoundedActionCompletionQueue::enqueue` to `Ok(false)` — test passes; queue silently rejects every enqueue. | Replace with `assert_eq!(result, Ok(())); assert_eq!(queue.len(), 1);` |
| F2-02 | CRITICAL | `crates/vb_runtime/src/shard/lru_ring_red_queen_remove_props.rs:175` | ROUND 1 REGRESSION of F-05. `lru_ring_property_capacity_overflow_then_recover` ends `assert!(r.is_err(), "must fail when full");`. | Mutate `LruRing::insert` to return `Err(LruError::SlotArenaFull)` instead of `Err(RuntimeError::TerminalRunsLruFull)` — test still passes. | Replace with `assert!(matches!(r, Err(RuntimeError::TerminalRunsLruFull { .. })));` |
| F2-03 | CRITICAL | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2141` | ROUND 1 REGRESSION of F-06. `recover_runtime_summary_handles_empty_journal` ends `assert!(result.is_err(), "empty journal should return error");`. | Mutate `recover_runtime_summary` to return `Err(RecoveryError::EmptyJournal)` instead of `Err(RecoveryError::NoRecoveryData)` — test still passes. | Replace with `assert!(matches!(result, Err(RecoveryError::NoRecoveryData { run: found }) if found == run));` (same pattern as `recovery_bdd_tests.rs:1060` already uses) |
| F2-04 | CRITICAL | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2728` | ROUND 1 REGRESSION of F-10. `tail_after_watermark_succeeds` ends `assert!(result.is_ok(), "tail after watermark should succeed");` standalone — entire test is 27 lines of fixture + this 1 assertion. | Mutate `hydrate_run_frame` to `Ok(RunFrame::default())` — test passes; hydration silently broken. | Replace with `let frame = result.expect("tail after watermark must hydrate"); assert_eq!(frame.pc(), StepIdx::new(3));` |
| F2-05 | CRITICAL | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2843` | ROUND 1 REGRESSION of F-07. `check_compiled_ir_digest_accepts_matching_digest` ends `assert!(result.is_ok(), "matching digests should succeed");` standalone. | Mutate `check_compiled_ir_digest` to a stub that always returns `Ok(())` — test passes. | Replace with `assert_eq!(result, Ok(()));` |
| F2-06 | CRITICAL | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2852` | ROUND 1 REGRESSION of F-08. `check_compiled_ir_digest_rejects_mismatch` ends `assert!(result.is_err(), "mismatched digests should be rejected");` standalone. | Mutate to return `Err(WrongVariant)` instead of `Err(DigestMismatchError)` — test passes. | Replace with `assert!(matches!(result, Err(DigestMismatchError { expected, found }) if expected == expected && found == found));` |
| F2-07 | CRITICAL | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2883` | ROUND 1 REGRESSION of F-09. `recover_runtime_summary_returns_recovery_hydration` ends `assert!(result.is_ok(), "should return RecoveryHydration");` standalone. | Mutate `recover_runtime_summary` to `Ok(RecoverySummary::default())` — test passes; recovery summary type silently wrong. | Replace with `let summary = result.expect("must return summary"); assert!(matches!(summary.kind, RecoverySummaryKind::Hydration { .. }));` |
| F2-08 | CRITICAL | `crates/vb_runtime/src/frame_pool/tests.rs:147, 244, 259, 260, 261, 273, 274, 351` | ROUND 1 REGRESSION of F-13. 8× `assert_eq!(reused.is_ok(), true)` disguised smoke patterns. | Mutate `FramePool::take` to return `Ok(FrameRef::default())` — all 8 tests pass; pool recycles wrong identities. | Replace each with `assert!(matches!(take_result, Ok(f) if f.run_id() == RunId::new(2)));` |
| F2-09 | CRITICAL | `crates/vb_runtime/src/together_tests.rs:73, 255, 288, 505, 546, 592, 635, 968, 1016, 1059, 1106, 1216, 1420, 1533` | ROUND 1 REGRESSION of F-17. 14× `assert!(run.add_parallel_in_flight(N).is_ok())` standalone, no post-state invariant. | Mutate `add_parallel_in_flight` to `Ok(())` without mutating the counter — all 14 tests pass; parallel-counter invariant silently broken. | Replace each with `let before = run.parallel_in_flight(); assert_eq!(run.add_parallel_in_flight(N), Ok(())); assert_eq!(run.parallel_in_flight(), before + N);` |
| F2-10 | CRITICAL | `crates/vb_runtime/src/shard/tests/chunk_017.rs:217, 218, 220` | ROUND 1 REGRESSION of F-14. `bh_shd_07_frame_pool_allocates_beyond_pool_capacity` has 3× `assert!(f1/f2/f3.is_ok(), "BH-SHD-07: ...")` with no concrete payload. | Mutate `FramePool::take` to `Ok(FrameRef::default())` — test passes; frame recycling corrupted. | Replace each with `assert!(matches!(f1, Ok(f) if f.run_id() == RunId::new(1)));` |
| F2-11 | CRITICAL | `crates/vb_runtime/src/shard/tests/chunk_dispatch_error_semantics.rs:159` | ROUND 1 REGRESSION of F-15. OR-conditioned smoke `assert!(result.is_err() \|\| shard.run_state_contains(run));` — test passes for EITHER branch. | Mutate `shard.tick()` to silently swallow resume + keep run alive — `is_err() == false`, `run_state_contains == true`, test passes. | Split into 2 tests: `assert!(matches!(result, Err(RuntimeError::NotResumable { .. })))` + `assert!(shard.run_state_contains(run))`. |
| F2-12 | CRITICAL | `crates/vb_core/src/engine/tests/integration_frame_behavior.rs:34, 83, 686, 695, 705` | ROUND 1 REGRESSION of F-16. 5× `assert!(frame.is_ok());` smoke after `RunFrame::new(...)` — no concrete frame payload. | Mutate `RunFrame::new` to ignore arguments, return `Ok(RunFrame::default())` — all 5 tests pass. | Replace each with `assert!(matches!(frame, Ok(f) if f.run_id() == RunId::new(1) && f.step_count() == 3 && f.slot_count() == 2));` |
| F2-13 | CRITICAL | `crates/vb_runtime/src/primitives/wait_ask_tests.rs:115, 126, 156, 190, 372, 387` | NEW (not in round 1). 6× `assert_eq!(result.is_err(), true)` disguised smoke in `wait_until_returns_error_when_slot_uninitialized`, `wait_event_returns_error_when_event_slot_uninitialized`, etc. | Mutate `wait_until` to `Ok(Signal::Continue)` instead of `Err(EngineError::SlotUninitialized)` — test passes; wait primitive silently succeeds. | Replace each with `assert!(matches!(result, Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(0)));` |
| F2-14 | CRITICAL | `crates/vb_core/tests/proptest_symbolic_code.rs:52, 59, 66, 146` | ROUND 1 FIX NEVER APPLIED for F-19. 4× `prop_assert!(parsed.is_err())` with no concrete error variant. | Mutate `SymbolicCode::from_str` to return `Err(SymbolicCodeParseError::InvalidCharacter)` instead of `Err(UnknownCode)` — all 4 tests pass. | Replace each with `prop_assert!(matches!(parsed, Err(SymbolicCodeParseError::UnknownCode { .. })));` |
| F2-15 | CRITICAL | `crates/vb_core/src/policy/contract/tests.rs:156, 164` | ROUND 1 FIX NEVER APPLIED for F-20. `test_new_validates_zero_active_runs` and `test_new_validates_zero_retry_attempts` end `assert!(result.is_err());` with no concrete variant. | Mutate `RuntimeLimitsProfile::new` to swap `ZeroActiveRuns` ↔ `ZeroRetryAttempts` errors — both tests still pass. | Replace with `assert!(matches!(result, Err(LimitsProfileError::ZeroActiveRuns)));` (and ZeroRetryAttempts respectively). |
| F2-16 | CRITICAL | `crates/vb_runtime/tests/admission_decision_test.rs:706` | NEW (not in round 1). `assert!(result.is_err());` standalone in `only_valid_artifact_case_admits`-adjacent test. Comment admits "Err means no RunAdmission was produced" — no variant check. | Mutate `admit_artifact_run_with_certificate_floor` to return `Err(AdmissionError::CapabilityDenied)` instead of `Err(AdmissionError::ArtifactNotFound)` — test passes; wrong rejection reason silently accepted. | Replace with `assert!(matches!(result, Err(AdmissionError::ArtifactNotFound { digest: found }) if found == digest));` |
| F2-17 | CRITICAL | `crates/vb_core/src/engine/tests/integration_taint_propagation.rs:2301, 2306` | NEW (wave-5/6/7). `assert!(bytes.is_ok());` followed by `let Ok(bytes) = bytes else { continue; };` — **silent fall-through** on Err. Test loop continues without exercising recovery if any iteration's postcard::to_allocvec fails. | Mutate `Taint` enum to grow an extra variant not in postcard schema — `to_allocvec` fails first iteration, loop `continue`s, test passes with zero recovered values asserted. | Replace with `let bytes = bytes.expect("postcard serialize"); let recovered: Taint = postcard::from_bytes(&bytes).expect("postcard deserialize"); assert_eq!(recovered, Taint::Secret);` (per-iteration, no continue) |
| F2-18 | HIGH | `crates/vb_runtime/src/properties_ticket_derivation.rs:38` | NEW (wave-5/6/7). `assert!(encoded_len.is_ok(), "answer_len must fit in u32 (max 65536)");` standalone property test. The follow-up `assert_eq!(encoded_len.unwrap() as usize, answer_len)` provides some payload check but the `is_ok()` smoke is decorative. Also: proptest domain `0..=65536usize` is off-by-one — `u32::try_from(65536)` IS Ok (`u32::MAX = 4294967295`), so the test never actually exercises the Err path. | Mutate `u32::try_from` to always return Ok — test passes; the entire `is_ok()` smoke is decorative. The test domain never hits the `Err` case anyway. | Remove the smoke (or assert `is_ok()` only for `0..=u32::MAX as usize` and add a separate `Err` proptest for `>u32::MAX`). |
| F2-19 | HIGH | `crates/vb_runtime/src/admission/step_budget_tests/mod.rs:143` | NEW (wave-5/6/7). `submit_1k_step_workflow_accepted` ends `assert!(result.is_ok(), "submit_direct should accept 1000-step workflow");` smoke. | Mutate `Runtime::submit_direct` to `Ok(())` without actually queueing the workflow — test passes; admission silently drops the run. | Replace with `assert_eq!(result, Ok(())); assert_eq!(runtime.collect_metrics().runs_active, 1);` (verify post-state). |
| F2-20 | HIGH | `crates/vb_core/src/action/tests.rs:1021, 1025, 1032, 1067, 1070, 1150, 1152, 1715, 1718` | ROUND 1 NOT COVERED. 9× `assert!(write_X.is_ok())` smoke patterns (write_clean, write_secret, write_derived, write_input, write_output, bytes.is_ok for serialization, recovered.is_ok for deserialization). | Mutate `write_slot_with_taint` to `Ok(())` without actually mutating the slot — all 9 tests pass; taint propagation silently broken. | Replace each with `assert_eq!(write_X, Ok(()));` followed by read-and-verify of the taint value at that slot. |
| F2-21 | HIGH | `crates/vb_runtime/src/primitives/retry/tests.rs:14, 60, 225, 707, 1235, 1243` | ROUND 1 NOT COVERED. 6× `assert!(policy.is_ok());` / `assert!(write_result.is_ok());` smoke patterns. `policy.ok().expect("must succeed")` follows — validation logic could return `Ok(default_policy())` and tests still pass. | Mutate `RetryPolicy::new` to return `Ok(RetryPolicy::default())` while ignoring inputs — concrete `assert_eq!(policy.max_attempts(), 3)` passes for default. | Replace each smoke with `assert_eq!(result, Ok(RetryPolicy { max_attempts: u16::MAX, ... }));` (concrete payload check). |
| F2-22 | HIGH | `crates/vb_runtime/src/primitives/for_each/tests.rs:114, 186, 282` | NEW (not in round 1). 3× `assert!(run.write_slot(input, ...).is_ok());` smoke for fixture construction. Borderline (these are fixture writes, but the smoke pattern is still in scope per the rubric). | Mutate `RunFrame::write_slot` to silently drop writes (return `Ok(())` without mutating) — tests pass; subsequent read assertions would fail, so this is borderline. | Replace with `assert_eq!(run.write_slot(...), Ok(()));` (no behavior change, but consistent style). |
| F2-23 | HIGH | `crates/vb_runtime/src/engine/tests/mod.rs:1141, 1738, 1870` | ROUND 1 NOT COVERED. 3× `assert!(result.is_ok(), "drive should succeed, got {result:?}")` smoke patterns with concrete follow-ups (events, pc). | Mutate `drive` to `Ok(EngineSignal::Continue)` after every step instead of the contractually expected signal — concrete `events.len()` and `run.pc()` checks would still pass for many workflows. | Replace with `assert!(matches!(result, Ok(EngineSignal::Continue) \| Ok(EngineSignal::Finished) \| Ok(EngineSignal::AwaitingAction(_))));` (pin to contractually expected signal). |
| F2-24 | HIGH | `crates/vb_runtime/src/for_each_tests.rs:1658` | NEW (not in round 1). `assert!(run.set_pc(StepIdx::ZERO).is_ok());` smoke. | Mutate `RunFrame::set_pc` to `Ok(())` without mutating — test passes. | Replace with `assert_eq!(run.set_pc(StepIdx::ZERO), Ok(())); assert_eq!(run.pc(), StepIdx::ZERO);` |
| F2-25 | HIGH | `crates/vb_runtime/src/shard/impl_tests/chunk_001.rs:261, 279` | NEW (wave-5/6/7). 2× `assert!(result.is_ok());` smoke patterns. | Mutate `submit_*` to `Ok(())` without enqueueing — tests pass. | Replace with concrete post-condition `assert_eq!(queue.len(), 1);` |
| F2-26 | HIGH | `crates/vb_runtime/tests/durable_resume_red_phase.rs:82, 110, 506, 549` (4 Err smokes) and `:338, 408, 482, 616, 660` (5 Ok smokes) | NEW (wave-5/6/7). 9× `assert!(result.is_err())` / `assert!(result.is_ok())` smokes in `resume_*_red_phase.rs`. Each is FOLLOWED by concrete variant match — acceptable as-is, but the smoke is redundant. | The follow-up `matches!` IS the assertion; the smoke is decorative but does not violate the rubric. | Optional: collapse to single `assert!(matches!(result, Err(ResumeError::RunIdNotFound { run_id: found }) if found == run_id));`. |
| F2-27 | HIGH | `crates/vb_core/src/engine/tests/integration_error_routing_behavior.rs:1608` | NEW (wave-5/6/7). Proptest `route_error_handler_never_loses_error_context` ends `let _ = result;` — silent discard of routing decision. Tests that `route_error_handler` returns SOMETHING for any error, but never verifies routing. | Mutate `route_error_handler` to always `Ok(Signal::Continue)` regardless of error — proptest passes; error routing completely broken. | Replace with `prop_assert!(result.is_ok()); let signal = result.unwrap(); prop_assert!(matches!(signal, EngineSignal::Failed(_) \| EngineSignal::Continue));` |
| F2-28 | HIGH | `crates/vb_runtime/src/primitives/collect/mod.rs:207` | NEW (wave-5/6/7). Production code, NOT test: `let _ = states.require_current_page(run.run_id(), collector_slot, current_id)?;` — silent discard of intermediate return. | Mutate `require_current_page` to return an empty/wrong page — production silently corrupts collect state. | Production code change: replace `let _ = ...?;` with `let page = states.require_current_page(...)?;` and propagate page to caller. |
| F2-29 | MEDIUM | `crates/vb_core/tests/proptest_core_types.rs:232, 236, 240, 244` | NEW (wave-5/6/7). 4× `prop_assert!(r.is_ok(), "X insert within cap must succeed")` smoke. Followed by `prop_assert_eq!(store.total_arena_count(), total)` which IS concrete, so borderline MEDIUM. | Mutate `insert_symbol/list/object/blob` to silently drop inserts — `total_arena_count()` check would still pass if the count was tracked independently of inserts. | Optional: replace each smoke with `prop_assert!(matches!(r, Ok(_)));` (just style improvement). |
| F2-30 | MEDIUM | `crates/vb_runtime/tests/recovery_hydration_tests.rs:516, 1324` | NEW (wave-5/6/7). 2× `assert!(result.is_ok())` smokes. Each is followed by concrete `assert_eq!(frame.pc(), StepIdx::new(3))` etc. — borderline MEDIUM. | Mutate `hydrate_run_frame_from_events` to `Ok(RunFrame::default())` — concrete `frame.pc() == StepIdx::new(3)` would fail. So safe, but smoke is redundant. | Optional: replace with `assert_eq!(result.map(\|f\| f.pc()), Ok(StepIdx::new(3)));` |
| F2-31 | MEDIUM | `crates/vb_runtime/tests/admission_decision_test.rs:209, 255, 544, 575` | NEW (wave-5/6/7). 4× `assert!(result.is_ok())` / `assert!(result.is_err())` smokes. Each is FOLLOWED by concrete match — borderline MEDIUM (acceptable). | The follow-up matches! IS the assertion. | Optional: collapse smoke+match into single `assert!`. |
| F2-32 | MEDIUM | `crates/vb_runtime/tests/recovery_hydration_tests.rs:2140, 2154` | NEW (wave-5/6/7). `assert!(result.is_err(), "kani harness assertion")` inside `#[kani::proof]` harnesses — verifier harnesses, not behavior tests. Rubric rule 7 says these do not count. | N/A | OBSERVATION: not a defect. |
| F2-33 | MEDIUM | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs:326` | NEW (wave-5/6/7). `prop_assert!(result.is_err(), "DeterministicPure + {input_taint:?} input must reject")` inside proptest. FOLLOWED by concrete match on `Err(RuntimeError::ActionTaintDowngrade { required, supplied })` — borderline MEDIUM. | The follow-up IS the assertion. | Optional: collapse smoke+match. |
| F2-34 | MEDIUM | `crates/vb_runtime/src/shard/tests/chunk_032.rs:145` | NEW (wave-5/6/7). `assert!(result.is_err());` standalone in `command_queue_new_rejects_exceeding_max`. FOLLOWED by `match result { Err(CommandQueueCapacityExceeded { .. }) => {} _other => panic!(...) }` — borderline MEDIUM. | The follow-up IS the assertion. | Optional: collapse smoke+match. |
| F2-35 | MEDIUM | `crates/vb_runtime/src/property_tests/concurrency_safety.rs:843` | NEW (wave-5/6/7). `let _ = slot.take();` silent discard of slot.take() Result. | Mutate `slot.take()` to silently no-op — tests pass. | Replace with `slot.take().expect("slot must produce a value");` |
| F2-36 | MEDIUM | `crates/vb_runtime/src/frame_pool/tests.rs:686` | NEW (wave-5/6/7). `let _ = frame.increment_executed();` silent discard. | Mutate `increment_executed` to silently no-op — tests pass; parallel-counter invariant silently broken. | Replace with `frame.increment_executed().expect("increment must succeed");` (also the underlying `Result` should be checked) |
| F2-37 | MEDIUM | `crates/vb_runtime/src/recovery/tests.rs:546` | NEW (wave-5/6/7). `let _ = frame;` — the entire `RecoveryFrame` is silently discarded. Likely a fixture leak — if frame setup failed, the test silently proceeds. | Mutate `frame.write_slot` to silently drop — test passes. | Replace with `let _ = frame; // fixture only` comment to clarify intent. |
| F2-38 | MEDIUM | `crates/vb_runtime/tests/red_queen_lru_concurrent.rs:362, 367, 385, 518` | NEW (wave-5/6/7). 4× `let _ = ring.remove(...)` / `let _ = ring.insert(...)` silent error suppressions. ROUND 1 NOT COVERED for F-23. | Mutate `ring.insert` to always `Err(TerminalRunsLruFull)` — `ring.len() <= capacity` invariant still satisfied, test passes. | Replace with `match ring.insert(...) { Ok(_) \| Err(_) => () }` to make intent explicit. |
| F2-39 | MEDIUM | `crates/vb_core/src/frame/tests.rs:1903, 1904, 1962` | NEW (wave-5/6/7). `let _ = frame.increment_executed();` / `let _ = frame.add_parallel_in_flight(10);` silent discards in test fixture. | Mutate `increment_executed` to silently no-op — test passes; counter silent drift. | Replace with explicit `Result` check. |
| F2-40 | LOW | `crates/vb_runtime/src/shard/arena/arena_tests.rs:225-230` | NEW finding. `#[ignore = "types lack Default impl"] fn arena_manager_deallocate_all() { todo!(...) }` — `#[ignore]` + `todo!()` in a behavior test. Per rubric rule 11, this is a commented-out test. | The test cannot run; it provides zero coverage. | Either implement the test or remove it from the file entirely. |
| F2-41 | LOW | `crates/vb_runtime/src/action_queue/types.rs:125` | NEW finding. Production code `std::thread::sleep(remaining.min(Duration::from_millis(1)));` — wall-clock sleep in production runtime path. NOT in test scope, but flagged for awareness. | N/A (production) | Production code change: replace busy-loop with `std::hint::spin_loop()` or atomic wait. Flag for code review. |
| F2-42 | LOW | `crates/vb_runtime/tests/red_queen_lru_concurrent.rs:520` | ROUND 1 NOT COVERED for F-23. Same `let _ = ring.insert(...)` pattern, same mutation gap. | Same as F2-38. | Same fix. |
| F2-43 | LOW | `crates/vb_runtime/src/together_tests.rs:69-71, 87-90` | NEW pattern. `run.write_slot(output, ...).ok().unwrap_or_else(|| panic!("slot write must succeed"));` — idiomatic but violates the "no `expect()`" production rule (relaxed for tests, but verbose). | N/A (idiomatic) | OBSERVATION: acceptable test idiom but inconsistent with `assert_eq!(...)` pattern elsewhere. |
| F2-44 | OBSERVATION | `crates/vb_core/src/value/proptests.rs:166` | ROUND 1 NOT COVERED for F-18. `prop_assert!(FiniteF64::new(val).is_err());` smoke. | Mutate `FiniteF64::new` to return `Err(FiniteF64Error::Infinity)` instead of `Err(FiniteF64Error::NaN)` — proptest still passes. | Replace with `prop_assert!(matches!(FiniteF64::new(val), Err(FiniteF64Error::NaN)));` |
| F2-45 | OBSERVATION | All test files with file-level `#![allow(clippy::unwrap_used, ...)]` | Round 1 finding F-28 — still 30+ files with excessive `#![allow]` blocks. | N/A | Track as owner_approved_debt. |
| F2-46 | OBSERVATION | `crates/vb_runtime/src/admission/stores.rs:185` | `matches!(self.journal.compiled_ir(digest), Ok(Some(_)))` — appears in production, not test. `Some(_)` pattern in production matches the banned test pattern (rules 1). | N/A (production) | Production code: acceptable, but flagged for awareness. |
| F2-47 | OBSERVATION | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs:340-385` | `prop_deterministicpure_rejects_all_non_clean` and adjacent proptests — `prop_assert!(result.is_err())` smokes are FOLLOWED by concrete matches. These are acceptable per rubric rule 3 (concrete follow-up). | The follow-up IS the assertion. | None needed. |
| F2-48 | OBSERVATION | `crates/vb_runtime/src/verification/loom/*.rs`, `crates/vb_runtime/src/verification/proptest/*.rs` | Multiple `assert!(handle.join().is_ok())` and `prop_assert!(result.is_ok())` patterns inside loom/proptest harness scaffolding — these are concurrency-safety harnesses, NOT behavior tests per rubric rule 7. | N/A | OBSERVATION: not a defect; harnesses are correctly classified. |
| F2-49 | OBSERVATION | `crates/vb_core/src/engine/tests/integration_step_behavior.rs:2015, 2089` | `let _ = result;` inside `#[kani::proof]` harnesses — verifier harnesses (rubric rule 7). | N/A | OBSERVATION: not a defect; harnesses assert "no panic" not behavior. |

---

## 3. Code Snippets — CRITICAL/HIGH BEFORE/AFTER

### F2-01: `crates/vb_runtime/src/action_queue/action_queue_tests.rs:240`

```rust
// BEFORE (still present after round 1)
#[test]
fn bounded_action_queue_enqueue_single_item_succeeds() {
    let queue = BoundedActionCompletionQueue::new(3).unwrap();
    let ticket = make_ticket(0);
    let result = queue.enqueue(ticket);
    assert!(result.is_ok());
}

// AFTER
#[test]
fn bounded_action_queue_enqueue_single_item_succeeds() {
    let queue = BoundedActionCompletionQueue::new(3).unwrap();
    let ticket = make_ticket(0);
    let result = queue.enqueue(ticket);
    assert_eq!(result, Ok(()));
    assert_eq!(queue.len(), 1, "len must increment to 1 after enqueue");
    assert_eq!(queue.remaining_capacity(), 2, "remaining must decrement");
}
```

### F2-02: `crates/vb_runtime/src/shard/lru_ring_red_queen_remove_props.rs:175`

```rust
// BEFORE (still present after round 1 — file split retained the smoke)
let r = ring.insert(RunId::new(4), TimerTick::new(0));
assert!(r.is_err(), "must fail when full");
assert_eq!(ring.len(), 3);

// AFTER
let r = ring.insert(RunId::new(4), TimerTick::new(0));
assert!(
    matches!(r, Err(RuntimeError::TerminalRunsLruFull { .. })),
    "capacity overflow must surface TerminalRunsLruFull, got {r:?}"
);
assert_eq!(ring.len(), 3);
```

### F2-03 + F2-04 + F2-05 + F2-06 + F2-07: `crates/vb_runtime/tests/recovery_bdd_tests.rs:2141, 2728, 2843, 2852, 2883`

```rust
// BEFORE (all 5 still present after round 1)
assert!(result.is_err(), "empty journal should return error");          // 2141
assert!(result.is_ok(), "tail after watermark should succeed");           // 2728
assert!(result.is_ok(), "matching digests should succeed");              // 2843
assert!(result.is_err(), "mismatched digests should be rejected");       // 2852
assert!(result.is_ok(), "should return RecoveryHydration");              // 2883

// AFTER — 2141 (NoRecoveryData variant)
assert!(
    matches!(result, Err(RecoveryError::NoRecoveryData { run: found }) if found == run),
    "empty journal must yield NoRecoveryData, got {result:?}"
);

// AFTER — 2728 (hydrate_run_frame post-conditions)
let frame = result.expect("tail after watermark must hydrate");
assert_eq!(frame.pc(), StepIdx::new(3), "pc must advance to step 3");
assert_eq!(frame.slot_count(), expected_slots);

// AFTER — 2843 (matching digest)
assert_eq!(result, Ok(()));

// AFTER — 2852 (mismatched digest)
assert!(
    matches!(result, Err(DigestMismatchError { expected: exp, found: got })
        if exp == expected && got == found),
    "mismatch must surface DigestMismatchError with both fields"
);

// AFTER — 2883 (RecoveryHydration)
let summary = result.expect("recovery must succeed");
assert!(matches!(summary.kind, RecoverySummaryKind::Hydration { .. }));
```

### F2-09: `crates/vb_runtime/src/together_tests.rs:73` (representative, 14× total)

```rust
// BEFORE (all 14 still present after round 1)
assert!(run.add_parallel_in_flight(2).is_ok());
let result = together_join(...);

// AFTER
let before = run.parallel_in_flight();
assert_eq!(run.add_parallel_in_flight(2), Ok(()));
assert_eq!(run.parallel_in_flight(), before + 2,
    "add_parallel_in_flight must mutate the counter");
let result = together_join(...);
```

### F2-13: `crates/vb_runtime/src/primitives/wait_ask_tests.rs:115` (representative, 6× total)

```rust
// BEFORE (newly surfaced — not in round 1)
let result = wait_until(&mut run, deadline);
assert_eq!(result.is_err(), true);

// AFTER
let result = wait_until(&mut run, deadline);
assert!(
    matches!(result, Err(EngineError::SlotUninitialized { slot }) if slot == deadline),
    "wait_until must surface SlotUninitialized with the slot index"
);
```

### F2-17: `crates/vb_core/src/engine/tests/integration_taint_propagation.rs:2301, 2306`

```rust
// BEFORE (newly surfaced — wave-5/6/7)
for a in all {
    let joined = join_taint(a, Taint::Secret);
    let bytes = postcard::to_allocvec(&joined);
    assert!(bytes.is_ok());
    let Ok(bytes) = bytes else { continue; };  // ← silent fall-through
    let recovered: Result<Taint, _> = postcard::from_bytes(&bytes);
    assert!(recovered.is_ok());
    let Ok(recovered) = recovered else { continue; };  // ← silent fall-through
    assert_eq!(recovered, Taint::Secret);
}

// AFTER — per-iteration, no continue
for a in all {
    let joined = join_taint(a, Taint::Secret);
    let bytes = postcard::to_allocvec(&joined)
        .expect("postcard serialize Taint");
    let recovered: Taint = postcard::from_bytes(&bytes)
        .expect("postcard deserialize Taint");
    assert_eq!(recovered, Taint::Secret, "taint round-trip for {a:?}");
}
```

### F2-18: `crates/vb_runtime/src/properties_ticket_derivation.rs:36-40`

```rust
// BEFORE (wave-5/6/7)
#[test]
fn encoded_len_matches_answer_len(answer_len in 0..=65536usize) {
    let encoded_len = u32::try_from(answer_len);
    assert!(encoded_len.is_ok(), "answer_len must fit in u32 (max 65536)");
    assert_eq!(encoded_len.unwrap() as usize, answer_len,
        "encoded_len must match answer.len()");
}
// Mutation: proptest domain 0..=65536 never exercises the Err path
// (u32::try_from(65536) == Ok(65536)). The `is_ok()` smoke is decorative.

// AFTER — split into Ok and Err proptests
proptest! {
    #[test]
    fn encoded_len_ok_for_u32_range(answer_len in 0..=u32::MAX as usize) {
        let encoded_len = u32::try_from(answer_len)
            .expect("answer_len must fit in u32");
        assert_eq!(encoded_len as usize, answer_len);
    }

    #[test]
    fn encoded_len_err_above_u32(answer_len in (u32::MAX as usize + 1)..=usize::MAX) {
        let result = u32::try_from(answer_len);
        prop_assert!(matches!(result, Err(_)),
            "answer_len above u32::MAX must error, got {result:?}");
    }
}
```

---

## 4. Pattern Census (counts per banned pattern per crate)

Counts derived from `rg` sweeps over `crates/vb_core/**/*.rs` and `crates/vb_runtime/**/*.rs` (excluding `target/`, `.evidence/`, `verification/`, `kani/` — verifier harnesses are correctly feature-gated and not behavior tests per rubric rule 7).

| Pattern | vb_core | vb_runtime | Total | Notes |
|---------|---------|------------|-------|-------|
| `assert!(*.is_ok())` bare smoke | 11 | 39 | **50** | Up from 27 in round 1 — wave-5/6/7 added 23 new instances |
| `assert!(*.is_err())` bare smoke | 6 | 13 | **19** | Up from 10 in round 1 — wave-5/6/7 added 9 new instances |
| `assert_eq!(*.is_ok()/is_err(), true)` disguised smoke | 0 | 14 | **14** | Up from 8 in round 1 — 6 NEW in wait_ask_tests.rs |
| `prop_assert!(*.is_ok()/is_err())` proptest smoke | 9 | 0 | **9** | Round 1 had 4 — wave-5/6/7 added 5 new instances |
| `let _ = ring/journal/slot/stack/frame.result` (silent suppression) | ~20 | ~25 | **~45** | Same as round 1 (mostly fixture cleanup + kani harnesses) |
| `.unwrap()` total | ~50 | ~150 | **~200** | Same as round 1 (mostly fixture construction) |
| `panic!()` in tests | ~50 | ~50 | **~100** | Same as round 1 (mostly idiomatic enum destructuring) |
| `todo!()` / `unimplemented!()` in tests | 0 | 1 | **1** | NEW: `arena_tests.rs:228` |
| `#[ignore]` on behavior tests | 0 | 1 | **1** | NEW: `arena_tests.rs:225` |
| `#[should_panic]` without exact message | 0 | 0 | **0** | OK |
| `sleep()` in tests | 0 | 12 | **12** | Round 1 had 12 (timing-dependent, see round-1 F-26) |
| `lazy_static` / `OnceCell` / `OnceLock` / `static mut` / `thread_local!` | 0 | 0 | **0** | OK |
| `cfg(kani)` / `cfg(verus)` / `cfg(flux)` harnesses | 6 | ~20 | **~26** | OK (feature-gated) |

**Delta from round 1:** is_ok/is_err smoke patterns grew from 49 (round 1) to 92 (round 2). The CRITICAL round-1 fixes added 7 new regressions (most in recovery_bdd_tests.rs where 5 round-1 fixes did not land), and wave-5/6/7 commits introduced 26 NEW smoke patterns across `properties_ticket_derivation.rs`, `admission/step_budget_tests/`, `durable_resume_red_phase.rs`, `admission_decision_test.rs`, `recovery_hydration_tests.rs`, `lifecycle_tests/chunk_008.rs`, `shard/tests/chunk_032.rs`, `shard/tests/chunk_017.rs`, `primitives/wait_ask_tests.rs`, `shard/impl_tests/chunk_001.rs`.

---

## 5. Mutation Gaps — 5 most dangerous mutations NOT caught by current tests

| # | Production code location | Mutation | Why current tests miss it |
|---|--------------------------|----------|----------------------------|
| 1 | `crates/vb_runtime/src/action_queue/action_queue_tests.rs::BoundedActionCompletionQueue::enqueue` — replace `Ok(())` with `Ok(false)` | Round 1 fix F-04 was reverted; test still asserts only `is_ok()` (F2-01). | Test passes; the queue silently rejects every enqueue. |
| 2 | `crates/vb_runtime/src/shard/lru_ring_red_queen_remove_props.rs::LruRing::insert` (or upstream `RuntimeError::TerminalRunsLruFull` builder) — replace error variant with `Err(RuntimeError::SlotArenaFull)` | Round 1 fix F-05 was lost in the file split; test still asserts only `is_err()` (F2-02). | Test passes; the wrong error variant bubbles up to dispatch. |
| 3 | `crates/vb_runtime/src/recovery/recover_runtime_summary.rs` (production) — replace Ok with `Ok(RecoverySummary::default())` for all paths | All 5 round-1 fixes for `recovery_bdd_tests.rs:2141, 2728, 2843, 2852, 2883` were reverted; tests still assert only `is_ok()/is_err()` (F2-03 through F2-07). | All 5 tests pass; recovery silently returns empty summaries. |
| 4 | `crates/vb_runtime/src/frame_pool/pool.rs::FramePool::take` — replace `Ok(FrameRef { run_id: .., pc: .. })` with `Ok(FrameRef::default())` | Round 1 fix F-13 was reverted; 8 tests still use `assert_eq!(reused.is_ok(), true)` (F2-08). | All 8 tests pass; the pool recycles the wrong frame identities. |
| 5 | `crates/vb_runtime/src/primitives/together/add_parallel_in_flight.rs` — replace `Ok(())` with `Ok(())` but skip the `parallel_in_flight += delta` mutation | Round 1 fix F-17 was reverted; 14 tests still use `assert!(run.add_parallel_in_flight(N).is_ok())` (F2-09). | All 14 tests pass; together-join's parallel-counter invariant is silently broken. |

A sixth class: `vb_runtime/src/primitives/wait_ask_tests.rs::wait_until/wait_event` — 6 NEW tests (F2-13) use `assert_eq!(result.is_err(), true)` which would survive a swap from `Err(SlotUninitialized)` to `Err(Generic)`. Plus the silent fall-through in `integration_taint_propagation.rs:2301` (F2-17) means a single `postcard::to_allocvec` failure skips ALL further iteration assertions, masking the round-trip invariant entirely.

---

## 6. Top 5 Fixes Ranked by Impact-per-Effort

1. **F2-01 + F2-04 + F2-05 + F2-06 + F2-07 (`recovery_bdd_tests.rs` + `action_queue_tests.rs`)** — 6 one-line replacements of `assert!(result.is_ok()/is_err())` with `matches!(...)`. Catches recovery-summary silent mutation class. Effort: 15 minutes.
2. **F2-09 (`together_tests.rs` ×14)** — for each `assert!(run.add_parallel_in_flight(N).is_ok())`, add `assert_eq!(run.parallel_in_flight(), before + N)` before the smoke. 14 edits. Effort: 30 minutes.
3. **F2-08 (`frame_pool/tests.rs` ×8)** — replace each `assert_eq!(reused.is_ok(), true)` with `assert!(matches!(reused, Ok(f) if f.run_id() == ...))`. 8 edits. Effort: 20 minutes.
4. **F2-02 + F2-03 + F2-10 + F2-11 + F2-12 (`shard/lru_ring_red_queen_remove_props.rs`, `shard/tests/chunk_017.rs`, `chunk_dispatch_error_semantics.rs`, `vb_core/engine/tests/integration_frame_behavior.rs`)** — 10 edits to convert smokes to `matches!`. Effort: 25 minutes.
5. **F2-13 + F2-14 + F2-15 + F2-16 (`wait_ask_tests.rs`, `proptest_symbolic_code.rs`, `policy/contract/tests.rs`, `admission_decision_test.rs`)** — 13 edits across 4 files. Effort: 25 minutes.

---

## 7. Verdict Line

STATUS: REJECTED

7 of 10 CRITICAL round-1 fixes have been REVERTED (action_queue_tests.rs:240, lru_ring_red_queen_remove_props.rs:175, recovery_bdd_tests.rs:2141/2728/2843/2852/2883), and all 5 HIGH round-1 blockers (frame_pool ×8, chunk_017 ×3, chunk_dispatch_error_semantics OR-smoke, integration_frame_behavior ×5, together_tests ×14) are STILL REGRESSED. Wave-5/6/7 introduced 4 NEW smoke patterns (properties_ticket_derivation.rs:38, admission/step_budget_tests/mod.rs:143, chunk_032.rs:145 standalone, integration_taint_propagation.rs:2301/2306 silent fall-through) and one `#[ignore]` + `todo!()` (`arena_tests.rs:225`). 17 CRITICAL findings and 11 HIGH findings collectively describe tests that pass if the behavior they verify were deleted. Per the rubric, "If any finding is `blocker`, write `STATUS: REJECTED` and prevent advancement."

---

## 8. Disposition

| ID | Disposition |
|----|-------------|
| F2-01 | blocker |
| F2-02 | blocker |
| F2-03 | blocker |
| F2-04 | blocker |
| F2-05 | blocker |
| F2-06 | blocker |
| F2-07 | blocker |
| F2-08 | blocker |
| F2-09 | blocker |
| F2-10 | blocker |
| F2-11 | blocker |
| F2-12 | blocker |
| F2-13 | blocker |
| F2-14 | blocker |
| F2-15 | blocker |
| F2-16 | blocker |
| F2-17 | blocker |
| F2-18 | owner_approved_debt (proptest domain off-by-one + smoke) |
| F2-19 | owner_approved_debt (admission smoke) |
| F2-20 | owner_approved_debt (action tests smokes ×9) |
| F2-21 | owner_approved_debt (retry tests smokes ×6) |
| F2-22 | owner_approved_debt (for_each fixture smokes ×3) |
| F2-23 | owner_approved_debt (engine drive smokes ×3) |
| F2-24 | owner_approved_debt (for_each_tests smoke) |
| F2-25 | owner_approved_debt (chunk_001 smokes ×2) |
| F2-26 | owner_approved_debt (durable_resume_red_phase redundant smokes ×9, but concrete follow-up present) |
| F2-27 | owner_approved_debt (error_routing weak proptest) |
| F2-28 | owner_approved_debt (production code silent discard in primitives/collect/mod.rs) |
| F2-29 | owner_approved_debt (proptest_core_types redundant smokes ×4) |
| F2-30 | owner_approved_debt (recovery_hydration_tests redundant smokes ×2) |
| F2-31 | owner_approved_debt (admission_decision_test smokes ×4, concrete follow-up present) |
| F2-32 | owner_approved_no_action (kani harness inside #[kani::proof], rubric rule 7) |
| F2-33 | owner_approved_debt (lifecycle_tests/chunk_008 redundant smoke) |
| F2-34 | owner_approved_debt (chunk_032 redundant smoke, concrete follow-up present) |
| F2-35 | owner_approved_debt (concurrency_safety slot.take() silent discard) |
| F2-36 | owner_approved_debt (frame_pool/tests.rs:686 silent discard) |
| F2-37 | owner_approved_debt (recovery/tests.rs:546 `let _ = frame` likely fixture leak) |
| F2-38 | owner_approved_debt (red_queen_lru_concurrent.rs silent inserts/removes ×4) |
| F2-39 | owner_approved_debt (frame/tests.rs silent discards ×3) |
| F2-40 | owner_approved_debt (arena_tests.rs `#[ignore]` + `todo!()`) |
| F2-41 | owner_approved_debt (action_queue/types.rs:125 production `thread::sleep`) |
| F2-42 | owner_approved_debt (duplicate of F2-38, round-1 F-23 still present) |
| F2-43 | owner_approved_no_action (idiomatic test pattern, verbose but correct) |
| F2-44 | owner_approved_debt (value/proptests.rs:166 smoke, round-1 F-18 still present) |
| F2-45 | owner_approved_debt (30+ test files with file-level `#![allow(...)]`, round-1 F-28) |
| F2-46 | owner_approved_no_action (admission/stores.rs:185 production `Some(_)`, flagged for awareness) |
| F2-47 | owner_approved_no_action (lifecycle_tests/chunk_008 proptest smokes have concrete follow-up) |
| F2-48 | owner_approved_no_action (loom/proptest harnesses, rubric rule 7) |
| F2-49 | owner_approved_no_action (kani harnesses with `let _ = result`, rubric rule 7) |

**Summary by disposition:**
- blocker: F2-01 through F2-17 (17 CRITICAL)
- owner_approved_debt: F2-18 through F2-20, F2-21, F2-22, F2-23, F2-24, F2-25, F2-26, F2-27, F2-28, F2-29, F2-30, F2-31, F2-33, F2-34, F2-35, F2-36, F2-37, F2-38, F2-39, F2-40, F2-41, F2-42, F2-44, F2-45 (24 owner-approved debt items requiring bead filing)
- owner_approved_no_action: F2-32, F2-43, F2-46, F2-47, F2-48, F2-49 (6 no-action observations)

**Required actions before re-review:**
1. File 17 beads for the CRITICAL blockers (round-1 fixes that regressed + NEW wave-5/6/7 blockers).
2. Re-apply the 5 reverted round-1 fixes in: `action_queue_tests.rs:240`, `lru_ring_red_queen_remove_props.rs:175`, `recovery_bdd_tests.rs:2141/2728/2843/2852/2883`, `frame_pool/tests.rs:147/244/259-261/273/274/351`, `together_tests.rs` (14 locations), `chunk_017.rs:217-218/220`, `chunk_dispatch_error_semantics.rs:159`, `integration_frame_behavior.rs:34/83/686/695/705`. The round-1 review at `.evidence/test-review/slice-1-core-runtime-review.md` already contains the recommended fixes.
3. File 24 beads for the `owner_approved_debt` items.
4. Re-run Tier 0 → Tier 3 of the test-review pipeline on the 17+ affected files.
5. Investigate `arena_tests.rs:225-230` `#[ignore]` + `todo!()` — either implement or remove.
6. The 6 `owner_approved_no_action` items are observations that do not block approval once blockers are addressed.
