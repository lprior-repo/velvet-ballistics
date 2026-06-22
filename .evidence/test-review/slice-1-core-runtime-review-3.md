# Slice 1 Test Suite Review — vb_core + vb_runtime — ROUND 3

STATUS: REJECTED

Round 2 closed all 17 CRITICAL blocker beads and 24 owner_approved_debt items, and wave-8 (HEAD) left the working tree free of `is_ok()/is_err()` smoke patterns on the round-1+2 fix sites **as long as the working tree is preserved**. However, **3 round-2 fixes regressed in COMMITTED HEAD (`7586b096f`)**: wave-8 reverted F2-04 (`frame.step_count()`/`frame.slot_count()` in `watermark_preserves_snapshot_data_beyond_tail`), F2-06 (`matches!(divergent_result, Err(CompiledIrDigestMismatch { .. }))` in `check_compiled_ir_digest_accepts_matching_digest`), and F2-07 (`summary.workflow/steps_started/first_seq/last_seq` in `recover_runtime_summary_returns_recovery_hydration`). The current **working tree** has the fixes re-applied locally but **the changes are uncommitted**, so a `git reset --hard HEAD` or fresh checkout would leave the regressions live. Additionally, one committed test (`snapshot_plus_tail_rejects_tail_before_snapshot`) is **failing in `cargo test -p vb_runtime --test recovery_bdd_tests`** because the production error message changed wording without updating the test assertion (`detail.contains("not after snapshot seq")` vs production `"is not contiguous with snapshot seq"`). Wave-8 also introduced 3 new property-test files (`proptest_bound_enforcement.rs`, `proptest_bytecode_ast_parity.rs`, `proptest_for_each_ordering.rs`) plus a heavily restructured `section38_behavioral_properties.rs`; these contain paired-boundary `prop_assert!(...is_ok())` patterns that are acceptable per rubric rule 3 (concrete Err-match follow-up). The round-2 F2-27 finding (`route_error_handler_never_loses_error_context` still has `let _ = result;` silent discard) remains. Verdict is REJECTED because the committed HEAD contains 3 round-2 regressions and 1 failing behavior test that together make the slice non-shippable from a clean clone.

---

## 1. Round 1 + Round 2 Fix Verification Table

Verification performed against **COMMITTED HEAD `7586b096f`** (wave-8) and the **WORKING TREE** (uncommitted modifications present per `git status`). Round-1+2 fix status is "STILL APPLIED" only if both committed AND working-tree states show the fix; "REGRESSED (committed)" means the committed HEAD reverted the fix but the working tree has it re-applied (the user's running tests see the fix, but a fresh checkout sees the regression).

| # | Round | ID | Original fix location | Expected fix shape | Committed HEAD | Working tree | Evidence |
|---|-------|----|------------------------|--------------------|---------------|--------------|----------|
| 1 | R1 | F-01 | `crates/vb_runtime/src/engine/action_tests.rs:267` | `matches!(result, Err(...IdMismatch...))` | **STILL APPLIED** | STILL APPLIED | `action_tests.rs:267-275` uses `assert_eq!` against concrete `RuntimeEngineError::Action(ActionError::UnknownAction)` |
| 2 | R1 | F-02 | `crates/vb_runtime/src/engine/action_tests.rs:289` | concrete `Ok(c) if c.id == ActionId::new(0)` | **STILL APPLIED** | STILL APPLIED | `action_tests.rs:302` `matches!(result, Ok(c) if c.id == ActionId::new(0) && c.id.get() == 0)` |
| 3 | R1 | F-03 | `crates/vb_runtime/src/engine/action_tests.rs:296` | concrete `Ok(c) if c.id == ActionId::new(2)` | **STILL APPLIED** | STILL APPLIED | `action_tests.rs:314` same pattern for `ActionId::new(2)` |
| 4 | R1 | F-04 | `crates/vb_runtime/src/action_queue/action_queue_tests.rs:240` | `assert_eq!(result, Ok(()))` + len checks | **STILL APPLIED** | STILL APPLIED | `action_queue_tests.rs:240-242` shows `assert_eq!(result, Ok(()))` + `queue.len()` + `queue.remaining_capacity()` |
| 5 | R1 | F-05 | `crates/vb_runtime/src/shard/lru_ring_red_queen_remove_props.rs:175` | `matches!(r, Err(RuntimeError::TerminalRunsLruFull))` | **STILL APPLIED** | STILL APPLIED | `lru_ring_red_queen_remove_props.rs:175-178` correct `matches!` pattern |
| 6 | R1 | F-06 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2141` | `matches!(result, Err(RecoveryError::NoRecoveryData ...))` | **STILL APPLIED** | STILL APPLIED | `recovery_bdd_tests.rs:2141-2144` correct `matches!` with field binding |
| 7 | R1 | F-07 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2843` | `assert_eq!(result, Ok(()))` (or `matches!`) | **STILL APPLIED** | STILL APPLIED | `recovery_bdd_tests.rs:2858` `matches!(result, Ok(()))` |
| 8 | R1 | F-08 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2852` | `matches!(result, Err(DigestMismatchError ...))` | **STILL APPLIED** | STILL APPLIED | `recovery_bdd_tests.rs:2876-2879` correct `matches!` |
| 9 | R1 | F-09 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2883` | concrete `RecoverySummaryKind::Hydration` check | **STILL APPLIED** | STILL APPLIED | `recovery_bdd_tests.rs:2911-2917` correct `matches!` |
| 10 | R1 | F-10 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2728` | concrete `frame.pc()` + `frame.slot_count()` | **STILL APPLIED** | STILL APPLIED | `recovery_bdd_tests.rs:2730-2733` has `result.expect` + `frame.pc()` + `frame.run_id()` |
| 11 | R2 | F2-04 | `recovery_bdd_tests.rs:2731-2736` (step_count + slot_count assertions in `watermark_preserves_snapshot_data_beyond_tail`) | add `frame.step_count()` + `frame.slot_count()` post-conditions | **REGRESSED (committed)** ✗ | STILL APPLIED | HEAD lacks the assertions; working tree re-adds them (`git diff HEAD -- crates/vb_runtime/tests/recovery_bdd_tests.rs` lines 1-10 show re-add) |
| 12 | R2 | F2-06 | `recovery_bdd_tests.rs:2862-2865` (divergent-input sub-assertion in `check_compiled_ir_digest_accepts_matching_digest`) | `matches!(divergent_result, Err(CompiledIrDigestMismatch { expected, found }))` | **REGRESSED (committed)** ✗ | STILL APPLIED | HEAD line 2862-2864 has bare `divergent_result.is_err()`; working tree re-adds `matches!` |
| 13 | R2 | F2-07 | `recovery_bdd_tests.rs:2919-2937` (summary.workflow/steps_started/first_seq/last_seq in `recover_runtime_summary_returns_recovery_hydration`) | 4 concrete `assert_eq!` on summary fields | **REGRESSED (committed)** ✗ | STILL APPLIED | HEAD lacks the 4 assertions; working tree re-adds them |
| 14 | R2 | F2-08 | `crates/vb_runtime/src/frame_pool/tests.rs:147,244,259-261,273,274,351` | `matches!(reused, Ok(f) if f.run_id() == ...)` | **STILL APPLIED** | STILL APPLIED | All 8 sites use `matches!` with `run_id()` check (live + committed) |
| 15 | R2 | F2-09 | `crates/vb_runtime/src/together_tests.rs:14 sites` | concrete `parallel_in_flight()` post-state | **STILL APPLIED** | STILL APPLIED | `together_tests.rs:73-75` shows `before_pif` + `assert_eq!(run.parallel_in_flight(), before_pif + 2)` |
| 16 | R2 | F2-10 | `crates/vb_runtime/src/shard/tests/chunk_017.rs:217-220` | `matches!(f1, Ok(f) if f.run_id() == ...)` | **STILL APPLIED** | STILL APPLIED | `chunk_017.rs:217-219` all 3 use `matches!` with `run_id()` |
| 17 | R2 | F2-11 | `crates/vb_runtime/src/shard/tests/chunk_dispatch_error_semantics.rs:159` | `matches!(result, Err(NotResumable))` + separate `run_state_contains` | **STILL APPLIED** | STILL APPLIED | `chunk_dispatch_error_semantics.rs:159-166` correct two-assert split |
| 18 | R2 | F2-12 | `crates/vb_core/src/engine/tests/integration_frame_behavior.rs:34,83,686,695,705` | `matches!(frame, Ok(f) if ...)` | **STILL APPLIED** | STILL APPLIED | `integration_frame_behavior.rs:34,88,696,710,725` all 5 use `matches!` |
| 19 | R2 | F2-13 | `crates/vb_runtime/src/primitives/wait_ask_tests.rs:115,126,156,190,372,387` | `matches!(result, Err(SlotUninitialized { slot }))` | **STILL APPLIED** | STILL APPLIED | All 6 sites use `matches!` with `slot` field check |
| 20 | R2 | F2-14 | `crates/vb_core/tests/proptest_symbolic_code.rs:52,59,66,146` | `matches!(parsed, Err(SymbolicCodeParseError { .. }))` | **STILL APPLIED** | STILL APPLIED | All 4 sites use `matches!` with concrete variant |
| 21 | R2 | F2-15 | `crates/vb_core/src/policy/contract/tests.rs:156,164` | `matches!(result, Err(ProfileValidationError::ExceedsHardLimit { field, value }))` | **STILL APPLIED** | STILL APPLIED | `policy/contract/tests.rs:157-163` + `171-176` both use `matches!` with `field` check |
| 22 | R2 | F2-16 | `crates/vb_runtime/tests/admission_decision_test.rs:706` | `matches!(result, Err(AdmissionError::ArtifactNotFound { digest }))` | **STILL APPLIED** | STILL APPLIED | `admission_decision_test.rs:706-712` correct `matches!` with `digest` field check |
| 23 | R2 | F2-17 | `crates/vb_core/src/engine/tests/integration_taint_propagation.rs:2301/2306` silent fall-through via `let Ok(x) = x else { continue; };` | replace with `x.expect(...)` per-iteration | **PARTIALLY FIXED** ✗ | STILL APPLIED at lines 2301-2306; a NEW site at line 2266-2272 still has the defective pattern (see F3-02) | HEAD has the original silent-fall-through; working tree removes it; wave-8 NEW test (lines 2230-2285 in section38_behavioral_properties) introduced a NEW instance at 2266 |
| 24 | R2 | F2-27 | `crates/vb_core/src/engine/tests/integration_error_routing_behavior.rs:1608` `let _ = result;` silent discard | `prop_assert!(result.is_ok())` + concrete signal match | **STILL REGRESSED** ✗ | STILL REGRESSED | `integration_error_routing_behavior.rs:1607-1608` still `let _ = result;` in `route_error_handler_never_loses_error_context` proptest |
| 25 | R1 | F-13 | `frame_pool/tests.rs:147` `assert_eq!(reused.is_ok(), true)` | `matches!(reused, Ok(f) if f.run_id() == ...)` | **STILL APPLIED** | STILL APPLIED | covered by F2-08 above |
| 26 | R2 | F2-11 | `chunk_dispatch_error_semantics.rs:158` OR-smoke | split into two tests with concrete `matches!` | **STILL APPLIED** | STILL APPLIED | covered by #17 above |
| 27 | R2 | F2-12 | `integration_frame_behavior.rs:34` `assert!(frame.is_ok())` | `matches!(frame, Ok(f) if ...)` | **STILL APPLIED** | STILL APPLIED | covered by #18 above |

**Summary:**
- 24 of 27 round-1+2 fixes STILL APPLIED in BOTH committed HEAD and working tree.
- 3 round-2 fixes (F2-04, F2-06, F2-07) **REGRESSED IN COMMITTED HEAD but re-applied in working tree (uncommitted)** — wave-8 reverted these 3 fixes in its commit `7586b096f`; the user has re-applied them locally.
- 1 round-2 fix (F2-27) **STILL REGRESSED in both committed HEAD and working tree**.
- 1 round-2 fix (F2-17) is partially regressed (the original site at 2301/2306 is fixed; a NEW site at line 2266-2272 introduced by wave-8 has the same defective pattern).

**Round-1+2 regression count (committed HEAD): 4** (F2-04, F2-06, F2-07, F2-27). **Round-1+2 regression count (working tree): 1** (F2-27).

---

## 2. Findings Table (ordered by severity)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix |
|----|-----|-----------|--------|------------------------------|-----------------|
| F3-01 | CRITICAL | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2862` (committed HEAD `7586b096f`) | WAVE-8 REGRESSION of round-2 F2-06. Committed version: `assert!(divergent_result.is_err(), "divergent-input sub-assertion: mismatched digests must surface Err, got {divergent_result:?}")`. Working tree re-applies the fix but the change is uncommitted. | Mutate `check_compiled_ir_digest` to return `Err(RecoveryError::JournalCorruption)` instead of `Err(RecoveryError::CompiledIrDigestMismatch { .. })` — test passes; wrong error variant bubbles up to dispatch. | Commit the working-tree `matches!(divergent_result, Err(RecoveryError::CompiledIrDigestMismatch { expected: exp, found: got }) if exp == digest && got == divergent)` to `git`. |
| F3-02 | CRITICAL | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2731-2736` (committed HEAD) | WAVE-8 REGRESSION of round-2 F2-04. Committed `watermark_preserves_snapshot_data_beyond_tail` ends at line 2733 with only `frame.pc()` and `frame.run_id()` checks — the 2 added assertions (`frame.step_count() == 1` and `frame.slot_count() == 1`) are missing. | Mutate `hydrate_run_frame` to return `Ok(RunFrame::new(run, ZERO, 99, 99))` — `pc() == ZERO` still passes; `run_id()` still passes; but `step_count()` and `slot_count()` assertions (if present) would catch the dimension mismatch. | Commit the working-tree re-addition of `assert_eq!(frame.step_count(), 1, ...)` + `assert_eq!(frame.slot_count(), 1, ...)`. |
| F3-03 | CRITICAL | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2919-2937` (committed HEAD) | WAVE-8 REGRESSION of round-2 F2-07. Committed `recover_runtime_summary_returns_recovery_hydration` ends at line 2918 with only `assert_eq!(summary.run, run, ...)` — the 4 added assertions (`summary.workflow`, `summary.steps_started`, `summary.first_seq`, `summary.last_seq`) are missing. | Mutate `recover_runtime_summary` to construct a `Summary` with default `workflow=None, steps_started=0, first_seq=ZERO, last_seq=ZERO` — `summary.run` check passes; concrete digest/seq/count checks would catch the silent drift. | Commit the working-tree re-addition of the 4 `assert_eq!` blocks on summary fields. |
| F3-04 | CRITICAL | `crates/vb_runtime/tests/recovery_bdd_tests.rs:449-503` (`snapshot_plus_tail_rejects_tail_before_snapshot`) — **TEST IS FAILING in HEAD `7586b096f`** | Test asserts `detail.contains("not after snapshot seq")` (line 500) but production `hydrate_run_frame` error detail reads `"tail event seq 2 is not contiguous with snapshot seq 3 (expected 4)"`. Failure output: `panicked at crates/vb_runtime/tests/recovery_bdd_tests.rs:499:5: detail should mention snapshot seq violation: tail event seq 2 is not contiguous with snapshot seq 3 (expected 4)`. **64 pass / 1 fail in `cargo test -p vb_runtime --test recovery_bdd_tests`**. | This is a stringly-typed assertion drift — the production error message evolved and the test wasn't updated. The test continues to assert on the *correct contract* (snapshot seq violation must be reported) but on the wrong string fragment. | Update test to `assert!(detail.contains("not contiguous") \|\| detail.contains("not after snapshot seq"), "...")` to accept either wording, or pin the production error message to match the test expectation. |
| F3-05 | HIGH | `crates/vb_core/src/engine/tests/integration_error_routing_behavior.rs:1597-1609` | ROUND-2 F2-27 STILL REGRESSED. Proptest `route_error_handler_never_loses_error_context` ends `let _ = result;` — silent discard of routing decision. | Mutate `route_error_handler` to always `Ok(Signal::Continue)` regardless of error — proptest passes; error routing completely broken. | Replace with `prop_assert!(result.is_ok()); let signal = result.unwrap(); prop_assert!(matches!(signal, EngineSignal::Failed(_) \| EngineSignal::Continue));` |
| F3-06 | HIGH | `crates/vb_core/src/engine/tests/integration_taint_propagation.rs:2266-2272` | WAVE-8 NEW (not in round 1 or 2). `postcard_roundtrip_preserves_taint_variants` uses `assert!(bytes.is_ok(), ...)` (line 2266-2269) followed by `let Ok(bytes) = bytes else { continue; };` (line 2270-2272) — silent fall-through identical to F2-17. The same pattern repeats for `recovered.is_ok()` at line 2274-2280. | Mutate `Taint` enum to add a variant not in postcard schema — `to_allocvec` fails on first iteration, loop `continue`s, all subsequent iterations skipped, test passes with zero round-trip invariants asserted. | Replace with `let bytes = postcard::to_allocvec(&variant).expect("postcard serialize Taint"); let recovered: Taint = postcard::from_bytes(&bytes).expect("postcard deserialize Taint"); assert_eq!(recovered, variant, ...);` |
| F3-07 | HIGH | `crates/vb_runtime/src/primitives/wait_ask_tests.rs:115,126,156,190,372,387` + `lru_ring_red_queen_combined_props.rs:110` | Round-2 F2-13 fix retained (uses `matches!`) BUT a parallel pattern in `lru_ring_red_queen_combined_props.rs:110` is `if ring.insert(id, now).is_ok() { ... }` — silent suppression where insert failures silently drop the consistency check. The truth state is only updated on Ok path; if insert always returns Err, `truth` remains empty but no assertion fires. | Mutate `LruRing::insert` to always `Err(TerminalRunsLruFull)` — `ring.len() <= truth.len()` invariant still satisfied (`0 <= 0`), all subsequent `ring.contains(&id)` checks skipped, proptest passes with zero consistency verification. | Replace with `match ring.insert(id, now) { Ok(_) => { /* update truth */ }, Err(e) => prop_assert!(false, "insert must succeed: {e:?}") }` to make the invariant mandatory. |
| F3-08 | MEDIUM | `crates/vb_runtime/src/shard/lru_ring_red_queen_remove_props.rs:95-98` (test `lru_ring_property_remove_uses_free_list_correctly`) | NEW (not in round 1 or 2). `assert!(result.is_err(), "insert at capacity must return Err, got {result:?}");` — bare smoke, no concrete variant check. The test is the same family as F-05 (round-1) but located in the "remove" sub-test, not the "capacity overflow" sub-test. The sibling fix at line 175 (F-05) DOES use `matches!` with concrete `TerminalRunsLruFull { .. }`. | Mutate `LruRing::insert` to return `Err(LruError::SlotArenaFull)` — test passes; wrong error variant silently accepted. | Replace with `assert!(matches!(result, Err(RuntimeError::TerminalRunsLruFull { .. })), "...");` to match the sibling fix at line 175. |
| F3-09 | MEDIUM | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2290, 2430, 2616, 2651, 2688, 2970` | `assert!(result.is_ok(), "...")` smoke pattern at the end of `*_is_recoverable` cluster (Round-2 F2-26 family, but several are borderline MEDIUM). Each is followed by concrete field checks in adjacent lines (`assert_eq!(frame.pc(), ...)`, `assert_eq!(summary.run, ...)`), so the smoke is redundant but not the sole assertion. | Mutate `recover_runtime_summary` to return `Ok(RecoveryHydration::default())` — `is_ok()` smoke passes; but the subsequent `summary.run == run` or `frame.pc() == StepIdx(3)` checks would fail. Safe but verbose. | Collapse to `assert_eq!(result.map(|h| /* check field */), Ok(/* expected */));` |
| F3-10 | MEDIUM | `crates/vb_runtime/tests/durable_resume_red_phase.rs:82, 110, 338, 408, 482, 506, 549, 616, 660` | Round-2 F2-26 cluster: `assert!(result.is_err()/is_ok(), ...)` followed by `matches!(err, ResumeError::RunIdNotFound { ... })` — concrete follow-up IS present, smoke is redundant. | Follow-up `matches!` IS the assertion; smoke is decorative. | OBSERVATION: acceptable per rubric rule 3 (concrete follow-up). Optional cleanup. |
| F3-11 | MEDIUM | `crates/vb_runtime/src/action_queue/tests/bounded_queue_tests.rs:444, 460, 466, 485, 508, 538, 681` | Round-1 F-25 family: `assert!(warning.is_ok())` followed by `let w = warning.unwrap(); assert_eq!(w.depth, 8); assert_eq!(w.capacity, 10);` — smoke + 2 concrete field checks. | Mutate `BackpressureWarning` constructor to set `depth = 8` and `capacity = 10` correctly but omit `reason` field — concrete `depth`/`capacity` checks pass; behavior is verified. | OBSERVATION: acceptable per rubric rule 3. |
| F3-12 | MEDIUM | `crates/vb_runtime/tests/recovery_hydration_tests.rs:516, 1324` | Round-2 F2-30: `assert!(result.is_ok(), "hydration should succeed: {result:?}")` followed by `let frame = result.unwrap(); assert_eq!(frame.pc(), StepIdx::new(3)); assert_eq!(frame.step_count(), 4);` — concrete post-conditions present. | Mutate `hydrate_run_frame_from_events` to `Ok(RunFrame::default())` — `is_ok()` passes; `frame.pc()` check catches. Safe. | OBSERVATION: acceptable per rubric rule 3. |
| F3-13 | MEDIUM | `crates/vb_runtime/src/primitives/retry/tests.rs:14, 60, 225, 707, 1235, 1243` | Round-2 F2-21 cluster: `assert!(policy.is_ok()); assert!(policy.is_ok()); ...` followed by `let policy = policy.unwrap(); assert_eq!(policy.max_attempts(), 3);` — concrete field check IS present. The smoke is redundant but not the sole assertion. | Mutate `RetryPolicy::new` to return `Ok(RetryPolicy::default())` — concrete `max_attempts()` check on default would pass. Borderline. | Replace with `assert_eq!(policy, Ok(RetryPolicy { max_attempts: 3, ... }));` to pin the full payload. |
| F3-14 | MEDIUM | `crates/vb_runtime/src/engine/tests/mod.rs:1141, 1738, 1870` | Round-2 F2-23 cluster: `assert!(result.is_ok(), "drive should succeed, got {result:?}")` followed by concrete `events.len() == N` and `run.pc() == StepIdx(1)` checks. The smoke is redundant but follow-up assertions catch signal-variant mutations. | Mutate `drive` to `Ok(EngineSignal::Continue)` after every step — `events.len()` and `pc()` checks would catch incorrect step counts but not signal-variant drift. | Replace with `assert!(matches!(result, Ok(EngineSignal::Continue) \| Ok(EngineSignal::Finished) \| Ok(EngineSignal::AwaitingAction(_))));` |
| F3-15 | MEDIUM | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs:326` | Round-2 F2-33: `prop_assert!(result.is_err(), "DeterministicPure + {input_taint:?} input must reject")` inside proptest, FOLLOWED by concrete `match result { Err(RuntimeError::ActionTaintDowngrade { required, supplied }) => { ... } }`. | Follow-up `match` IS the assertion. | OBSERVATION: acceptable per rubric rule 3. |
| F3-16 | MEDIUM | `crates/vb_runtime/src/admission/step_budget_tests/mod.rs:143` | Round-2 F2-19: `assert!(result.is_ok(), "submit_direct should accept 1000-step workflow");` smoke. No post-state check. | Mutate `Runtime::submit_direct` to `Ok(())` without actually queueing — test passes; admission silently drops the run. | Replace with `assert_eq!(result, Ok(())); assert_eq!(runtime.collect_metrics().runs_active, 1);` |
| F3-17 | MEDIUM | `crates/vb_core/src/action/tests.rs:1021, 1025, 1032, 1067, 1070, 1150, 1152, 1715, 1718` | Round-2 F2-20 cluster: `assert!(write_clean.is_ok()); assert!(write_secret.is_ok()); ... assert!(bytes.is_ok(), "serialization should succeed"); assert!(recovered.is_ok(), "deserialization should succeed");` — smoke patterns for slot writes and postcard round-trip. | Mutate `write_slot_with_taint` to `Ok(())` without mutating slot — `is_ok()` passes; subsequent taint-read assertions would catch, but taint-reads are not asserted in these 9 sites. | Replace with `assert_eq!(write_clean, Ok(()));` + `assert_eq!(frame.read_slot(SlotIdx::new(0)), Ok(SlotValue::I64(42)));` |
| F3-18 | MEDIUM | `crates/vb_runtime/src/shard/tests/chunk_017.rs:147, 290, 307, 351, 372, 568` (frame_pool `assert!(false)` placeholder lines) | `assert!(false);` in 6 sites in `frame_pool/tests.rs` — these look like unfinished test stubs that intentionally fail. Investigate: are these dead-code tests or active failing tests? | N/A (intentional fail) | Verify these are commented out or removed; if active, they always fail and block CI. |
| F3-19 | LOW | `crates/vb_core/src/engine/tests/integration_taint_propagation.rs:2339, 2793, 2242-2244` | NEW (wave-8) `let _ = recovered;` / `let _ = constant_taint;` / `assert!(disc_chain >= disc_a);` (3× boundary checks with no message) in taint discrimination tests. | Mutate the chain ordering — concrete `>=` checks would catch reordering, but `let _ =` patterns discard the recovered value entirely. | Replace with explicit `assert_eq!(recovered, expected)` or annotate intent. |
| F3-20 | LOW | `crates/vb_runtime/src/properties_ticket_derivation.rs:38` | Round-2 F2-18 (still present): `assert!(encoded_len.is_ok(), "answer_len must fit in u32 (max 65536)");` smoke in proptest with domain `0..=65536usize`. Proptest domain `0..=65536` never exercises the Err path (`u32::try_from(65536) == Ok(65536)`), so the `is_ok()` smoke is decorative. | Mutate `u32::try_from` to always return Ok — test passes; the entire smoke is decorative. The test never exercises Err anyway. | Replace with split: `prop_assert!(matches!(u32::try_from(answer_len), Ok(_)))` for `0..=u32::MAX` + separate Err proptest for `>u32::MAX`. |
| F3-21 | LOW | `crates/vb_core/src/budget/tests/chunk_015.rs:378` | NEW `let _ = budget;` — silent discard of computed budget value. | Mutate budget computation to silently return a default — test passes. | Replace with `assert!(!budget.is_empty() \|\| budget.is_zero(), "budget must be populated");` |
| F3-22 | LOW | `crates/vb_core/tests/proptest_core_types.rs:232, 236, 240, 244` | Round-2 F2-29 cluster: `prop_assert!(r.is_ok(), "X insert within cap must succeed, got {:?}", r);` smoke, FOLLOWED by `prop_assert_eq!(store.total_arena_count(), total)` which IS concrete. | Mutate `insert_symbol/list/object/blob` to silently drop inserts — `total_arena_count()` check would catch if tracking is independent of inserts. Borderline safe. | OBSERVATION: acceptable per rubric rule 3. |
| F3-23 | LOW | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2965-3003` | 5× `result.is_ok()` checks at lines 2290, 2347, 2398, 2430, 2473, 2515, 2548, 2580 (the recoverable summary cluster). Each is a bare smoke in a `match result { Ok(h) => { /* concrete field checks */ } Err(e) => panic!() }` block. The concrete field checks ARE present. | The follow-up `match` IS the assertion. | OBSERVATION: acceptable per rubric rule 3. |
| F3-24 | LOW | `crates/vb_runtime/src/together_tests.rs:1212, 1412` | `assert!(result.is_err(), ...)` + `match result { Err(EngineError::TypeMismatch { expected, found }) => { ... } other => panic!() }` — concrete follow-up. | Mutate `together_branch` to `Err(Generic)` instead of `Err(TypeMismatch)` — `is_err()` smoke passes; concrete `match` catches. Safe. | OBSERVATION: acceptable per rubric rule 3. |
| F3-25 | LOW | `crates/vb_runtime/src/primitives/wait_ask_tests.rs` | `Some(_)` smoke pattern was NOT found in this slice (0 matches for `Some(_)` pattern). | N/A | OBSERVATION: confirmed clean. |
| F3-26 | LOW | `crates/vb_runtime/src/shard/arena/arena_tests.rs:225-228` (round-2 F2-40) | `#[ignore = "types lack Default impl"] fn arena_manager_deallocate_all() { todo!(...) }` — `#[ignore]` + `todo!()` dormant test, still present. | The test cannot run. | Either implement the test or remove from file entirely. |
| F3-27 | LOW | `crates/vb_runtime/src/action_queue/types.rs:125` (round-2 F2-41) | PRODUCTION CODE: `std::thread::sleep(remaining.min(Duration::from_millis(1)));` — wall-clock sleep in runtime path. Not in test scope but flagged for awareness. | N/A | Production code change: replace busy-loop with `std::hint::spin_loop()` or atomic wait. |
| F3-28 | OBSERVATION | `crates/vb_core/tests/proptest_bound_enforcement.rs` (NEW wave-8 file, 471 lines) | Uses paired-boundary pattern: `prop_assert!(try_from_parts(exact).is_ok())` + `prop_assert!(matches!(try_from_parts(overflow), Err(WorkflowError::ResourceContractExceeded { resource: "..." })))`. 8 `is_ok()` smokes paired with 8 concrete `matches!` overflow asserts. | The paired overflow test IS the assertion; the Ok smoke is decorative but not sole. | OBSERVATION: acceptable per rubric rule 3. |
| F3-29 | OBSERVATION | `crates/vb_core/tests/proptest_bytecode_ast_parity.rs` (NEW wave-8 file, 297 lines) | 3 `prop_assert!(from_X.is_ok())` smokes at lines 185, 193, 222. Each is followed by concrete `prop_assert_eq!` on `max_stack`, `ops`, or specific variant matches. | The follow-up `prop_assert_eq!` IS the assertion. | OBSERVATION: acceptable per rubric rule 3. |
| F3-30 | OBSERVATION | `crates/vb_core/tests/proptest_for_each_ordering.rs` (NEW wave-8 file, 295 lines) | Uses `_ => prop_assert!(false, "expected ForEachX variant after round-trip")` patterns + concrete `prop_assert_eq!` on field values. | The field-value `prop_assert_eq!` IS the assertion. | OBSERVATION: well-designed property test. |
| F3-31 | OBSERVATION | `crates/vb_core/tests/section38_behavioral_properties.rs` (wave-8 restructure, -573 +519 lines) | Heavy restructuring from hand-written fixtures to proptest generators. Uses `prop_assert_eq!` on `frame.executed()`, `frame.pc()`, `signal_taint`, etc. — concrete post-conditions throughout. | Concrete assertions are present. | OBSERVATION: net improvement in test quality. |
| F3-32 | OBSERVATION | All test files | 30+ test files still opt out of `clippy::unwrap_used`, `clippy::panic`, etc. via file-level `#![allow(...)]`. Round-1 F-28 / round-2 F2-45. | N/A | Track as owner_approved_debt. |
| F3-33 | OBSERVATION | `crates/vb_runtime/src/shard/arena/arena.rs:179` (PRODUCTION) | `self.validated_handle_index(handle).is_ok()` — production `is_ok()` in dispatch path. | N/A | OBSERVATION: production code, not test scope. |

---

## 3. Pattern Census (counts per banned pattern per crate)

Counts derived from `rg` sweeps over `crates/vb_core/**/*.rs` and `crates/vb_runtime/**/*.rs` (excluding `target/`, `.evidence/`, `verification/`, `kani/` — verifier harnesses are correctly feature-gated and not behavior tests per rubric rule 7).

| Pattern | vb_core | vb_runtime | Total | Δ from R2 | Notes |
|---------|---------|------------|-------|-----------|-------|
| `assert!(*.is_ok())` bare smoke | 13 | 32 | **45** | -5 (R2: 50) | Wave-8 conversion of `assert_eq!(reused.is_ok(), true)` patterns to `matches!` reduced count by 5 |
| `assert!(*.is_err())` bare smoke | 7 | 11 | **18** | -1 (R2: 19) | F2-13 (wait_ask_tests) fully converted to `matches!` |
| `assert_eq!(*.is_ok()/is_err(), true)` disguised smoke | 0 | 0 | **0** | -14 (R2: 14) | F2-08 (frame_pool ×8) + F2-10 (chunk_017 ×3) + F2-13 (wait_ask_tests ×6) all converted to `matches!` — **0 instances remaining in HEAD** |
| `prop_assert!(*.is_ok()/is_err())` proptest smoke | 9 | 0 | **9** | +1 (R2: 8) | Wave-8 added 1 NEW instance in `proptest_bound_enforcement.rs:396` (paired boundary, acceptable per rule 3) |
| `let _ = ring/journal/slot/stack/frame.result` (silent suppression) | ~20 | ~25 | **~45** | 0 (R2: ~45) | Same as R2 — F2-27 (`route_error_handler_never_loses_error_context`) STILL has `let _ = result;` |
| `.unwrap()` total | ~50 | ~150 | **~200** | 0 (R2: ~200) | Mostly fixture construction |
| `.expect()` total | ~15 | ~14 | **~29** | 0 (R2: ~29) | Mostly fixture construction |
| `panic!()` in tests | ~50 | ~50 | **~100** | 0 (R2: ~100) | Mostly idiomatic enum destructuring |
| `todo!()` / `unimplemented!()` in tests | 0 | 1 | **1** | 0 (R2: 1) | `arena_tests.rs:228` (still ignored) |
| `#[ignore]` on behavior tests | 0 | 1 | **1** | 0 (R2: 1) | `arena_tests.rs:225` (still ignored) |
| `#[should_panic]` without exact message | 0 | 0 | **0** | 0 | OK |
| `sleep()` in tests | 0 | 12 | **12** | 0 (R2: 12) | Same — timing-dependent, see R1 F-26 |
| `lazy_static` / `OnceCell` / `OnceLock` / `static mut` / `thread_local!` | 0 | 0 | **0** | 0 | OK |
| `cfg(kani)` / `cfg(verus)` / `cfg(flux)` harnesses | 6 | ~20 | **~26** | 0 | OK (feature-gated) |
| **Bare `Some(_)` smoke pattern** | **0** | **0** | **0** | **0** | **0 matches across entire slice — clean** |

**Net improvement from R2 → R3:**
- `assert!(*.is_ok())` smokes: 50 → 45 (-5, all from F2-08 + F2-13 conversions)
- `assert!(*.is_err())` smokes: 19 → 18 (-1, F2-13)
- `assert_eq!(*.is_ok(), true)` disguised smokes: 14 → **0** (-14, complete removal of F2-08 + F2-10 + F2-13 disguised smokes)

**Total smoke patterns requiring attention: 63** (45 is_ok + 18 is_err + 0 disguised + 9 prop_assert smokes). Down from 92 in R2 (-29, -32%).

**Critical regression in `recovery_bdd_tests.rs` (committed HEAD):** 3 round-2 fixes (F2-04, F2-06, F2-07) reverted by wave-8. Working tree re-applies but changes are uncommitted.

---

## 4. Mutation Gaps — 5 most dangerous mutations NOT caught by current tests

| # | Production code location | Mutation | Why current tests miss it |
|---|--------------------------|----------|----------------------------|
| 1 | `crates/vb_runtime/src/recovery/recover_runtime_summary.rs::recover_runtime_summary` — replace `Ok(RecoveryHydration::Summary(summary { workflow: Some(digest), steps_started: 1, first_seq: EventSeq::new(0), last_seq: EventSeq::new(1), .. }))` with `Ok(RecoveryHydration::Summary(RecoverySummary::default()))` | Round-2 F2-07 was re-applied in working tree but the COMMITTED HEAD (`7586b096f`) only checks `summary.run == run` and `matches!(hydration, RecoveryHydration::Summary(_))`. The 4 concrete post-conditions (`workflow`, `steps_started`, `first_seq`, `last_seq`) are missing in HEAD. | Test passes against COMMITTED HEAD (working tree only has the fix). Recovery silently returns empty summary. Anyone cloning the repo without the uncommitted patch is unprotected. |
| 2 | `crates/vb_runtime/src/recovery/check_compiled_ir_digest.rs` (production) — replace `Err(RecoveryError::CompiledIrDigestMismatch { expected, found })` with `Err(RecoveryError::JournalCorruption)` | Round-2 F2-06 was re-applied in working tree but COMMITTED HEAD only checks `divergent_result.is_err()`. | Test passes against COMMITTED HEAD. Wrong error variant silently bubbles up to dispatch — recovery dispatcher cannot distinguish "ir digest mismatch" from "journal corruption" and may misroute the run. |
| 3 | `crates/vb_runtime/src/recovery/hydrate_run_frame.rs` — replace `Ok(RunFrame { pc: first_step, step_count: tail_len, slot_count: snapshot.slot_count + tail_len, .. })` with `Ok(RunFrame { pc: first_step, step_count: 99, slot_count: 99, .. })` | Round-2 F2-04 was re-applied in working tree but COMMITTED HEAD only checks `frame.pc()` and `frame.run_id()`. The `step_count` and `slot_count` assertions are missing in HEAD. | Test passes against COMMITTED HEAD. Hydrated frame has wrong dimensions; downstream primitives reading slot_count would index out of bounds. |
| 4 | `crates/vb_core/src/engine/error_routing.rs::route_error_handler` — replace `Ok(EngineSignal::Failed(error))` with `Ok(EngineSignal::Continue)` | Round-2 F2-27 STILL not fixed in either HEAD or working tree. Proptest `route_error_handler_never_loses_error_context` has `let _ = result;` silent discard. | Proptest passes silently. Error routing completely broken — failed runs silently continue instead of terminating. |
| 5 | `crates/vb_core/src/engine/integration_taint_propagation.rs::postcard_roundtrip_preserves_taint_variants` (lines 2264-2285) — production silently adds a new `Taint` variant not in postcard schema | Wave-8 NEW site at line 2266-2272 has `assert!(bytes.is_ok())` + `let Ok(bytes) = bytes else { continue; };` silent fall-through (same pattern as F2-17). | First iteration's `to_allocvec` fails, loop `continue`s, ALL subsequent iterations skipped, test passes with zero round-trip invariants asserted. |

A sixth class worth flagging: **`crates/vb_runtime/src/primitives/lru_ring_red_queen_combined_props.rs::LruRing::insert` — replace `Ok(())` with `Err(TerminalRunsLruFull)`**. Currently the `if ring.insert(id, now).is_ok() { /* update truth */ }` pattern at line 110 silently drops the consistency check on Err. Combined with `ring.len() <= truth.len()` (which is satisfied when both are 0), a "silent return-Err mutation" would slip past the entire consistency-check invariant.

---

## 5. Top 5 Fixes Ranked by Impact-per-Effort

1. **F3-04 (`recovery_bdd_tests.rs:449` `snapshot_plus_tail_rejects_tail_before_snapshot`)** — TEST IS FAILING IN HEAD. Either pin the production error message to "not after snapshot seq" OR update the test to accept the new wording `"not contiguous with snapshot seq"`. Single-line change. Effort: 1 minute. Catches a real production/test drift that has been live since wave-8. **MUST BE FIXED BEFORE ANY OTHER ACTION.**

2. **F3-01 + F3-02 + F3-03 (`recovery_bdd_tests.rs` committed HEAD regressions)** — commit the working-tree re-applications of F2-04, F2-06, F2-07 to git. 32-line diff already exists in `git diff HEAD`. Effort: 1 minute (single `git add` + `git commit`). Catches 3 wave-8 regressions that anyone cloning fresh will see.

3. **F3-06 (`integration_taint_propagation.rs:2266-2272` wave-8 NEW silent fall-through)** — replace `assert!(bytes.is_ok())` + `let Ok(bytes) = bytes else { continue; };` with `let bytes = postcard::to_allocvec(&variant).expect("postcard serialize Taint");` per-iteration. Two-line change. Effort: 5 minutes. Catches wave-8 re-introduction of the F2-17 defective pattern.

4. **F3-05 (`integration_error_routing_behavior.rs:1607-1608` round-2 F2-27 STILL regressed)** — replace `let _ = result;` with `prop_assert!(result.is_ok()); let signal = result.unwrap(); prop_assert!(matches!(signal, EngineSignal::Failed(_) \| EngineSignal::Continue));`. Three-line change. Effort: 5 minutes. Catches complete error-routing silence.

5. **F3-07 + F3-08 (`lru_ring_red_queen_combined_props.rs:110` + `lru_ring_red_queen_remove_props.rs:95-98`)** — replace `if ring.insert(id, now).is_ok() { ... }` with explicit `match ... { Ok(_) => { ... } Err(e) => prop_assert!(false, "insert must succeed: {e:?}") }`; and replace the F3-08 `assert!(result.is_err())` at line 96 with the sibling `matches!(result, Err(RuntimeError::TerminalRunsLruFull { .. }))` pattern from line 175. Two edits. Effort: 10 minutes. Catches LRU ring silent-failure class.

---

## 6. Verdict Line

STATUS: REJECTED

Wave-8 (`7586b096f`) committed 3 round-2 fixes regressions in `recovery_bdd_tests.rs` (F2-04 at lines 2731-2736, F2-06 at line 2862, F2-07 at lines 2919-2937); the working tree re-applies them locally but the changes are **uncommitted**, so a fresh clone sees the regressions. Additionally, the committed HEAD contains a failing behavior test (`snapshot_plus_tail_rejects_tail_before_snapshot` — 1 of 64 tests failing in `recovery_bdd_tests`). Wave-8 also re-introduced the F2-17 silent fall-through pattern in a NEW site (`integration_taint_propagation.rs:2266-2272`) and left the F2-27 proptest silent-discard unrepaired. Wave-8's net effect is **+5 NEW issues** (4 regressions + 1 failing test) and **-20 smoke patterns** (frame_pool/chunk_017/wait_ask_tests conversions to `matches!`). Slice must be re-fixed by committing the working-tree re-applications (F3-01..03), fixing the failing test (F3-04), and re-applying F2-17 (F3-06) and F2-27 (F3-05) before approval.

---

## 7. Disposition

| ID | Disposition |
|----|-------------|
| F3-01 | blocker (wave-8 committed regression of F2-06) |
| F3-02 | blocker (wave-8 committed regression of F2-04) |
| F3-03 | blocker (wave-8 committed regression of F2-07) |
| F3-04 | blocker (committed HEAD has failing behavior test) |
| F3-05 | blocker (round-2 F2-27 still not fixed) |
| F3-06 | blocker (wave-8 NEW F2-17-style silent fall-through) |
| F3-07 | owner_approved_debt (LRU ring silent suppression) |
| F3-08 | owner_approved_debt (LRU ring smoke at line 95) |
| F3-09 | owner_approved_debt (recovery_bdd_tests is_ok smoke cluster) |
| F3-10 | owner_approved_no_action (acceptable per rubric rule 3) |
| F3-11 | owner_approved_no_action (acceptable per rubric rule 3) |
| F3-12 | owner_approved_no_action (acceptable per rubric rule 3) |
| F3-13 | owner_approved_debt (retry tests smoke cluster) |
| F3-14 | owner_approved_debt (engine drive smoke cluster) |
| F3-15 | owner_approved_no_action (acceptable per rubric rule 3) |
| F3-16 | owner_approved_debt (admission step_budget smoke) |
| F3-17 | owner_approved_debt (action tests smoke cluster) |
| F3-18 | owner_approved_debt (frame_pool `assert!(false)` placeholders) |
| F3-19 | owner_approved_debt (taint silent discards) |
| F3-20 | owner_approved_debt (properties_ticket_derivation domain off-by-one) |
| F3-21 | owner_approved_debt (budget silent discard) |
| F3-22 | owner_approved_no_action (acceptable per rubric rule 3) |
| F3-23 | owner_approved_no_action (acceptable per rubric rule 3) |
| F3-24 | owner_approved_no_action (acceptable per rubric rule 3) |
| F3-25 | owner_approved_no_action (clean — 0 Some(_) matches) |
| F3-26 | owner_approved_debt (arena_tests `#[ignore]` + `todo!()`) |
| F3-27 | owner_approved_debt (production `thread::sleep` in action_queue/types.rs) |
| F3-28 | owner_approved_no_action (proptest_bound_enforcement paired boundary, acceptable) |
| F3-29 | owner_approved_no_action (proptest_bytecode_ast_parity, acceptable) |
| F3-30 | owner_approved_no_action (proptest_for_each_ordering, well-designed) |
| F3-31 | owner_approved_no_action (section38 restructuring, net improvement) |
| F3-32 | owner_approved_debt (30+ `#![allow(...)]` blocks, round-1 F-28) |
| F3-33 | owner_approved_no_action (production `is_ok()`, flagged for awareness) |

**Summary by disposition:**
- blocker: F3-01, F3-02, F3-03, F3-04, F3-05, F3-06 (6 CRITICAL blockers)
- owner_approved_debt: F3-07, F3-08, F3-09, F3-13, F3-14, F3-16, F3-17, F3-18, F3-19, F3-20, F3-21, F3-26, F3-27, F3-32 (14 owner-approved debt items requiring bead filing)
- owner_approved_no_action: F3-10, F3-11, F3-12, F3-15, F3-22, F3-23, F3-24, F3-25, F3-28, F3-29, F3-30, F3-31, F3-33 (13 no-action observations)

**Required actions before re-review:**
1. **Commit the working-tree re-applications** of F2-04, F2-06, F2-07 to `crates/vb_runtime/tests/recovery_bdd_tests.rs` (F3-01..03). The 32-line diff is already staged in the working tree.
2. **Fix the failing test** `snapshot_plus_tail_rejects_tail_before_snapshot` at `recovery_bdd_tests.rs:449-503` (F3-04).
3. **Apply round-2 F2-27 fix** to `integration_error_routing_behavior.rs:1607-1608` (F3-05).
4. **Re-apply F2-17 fix** to NEW wave-8 site `integration_taint_propagation.rs:2266-2272` (F3-06).
5. File 14 beads for the `owner_approved_debt` items.
6. Re-run Tier 0 → Tier 3 of the test-review pipeline on the 6 affected files.

**Round-3 summary metrics:**
- CRITICAL: 6 (all blockers; 4 wave-8 regressions + 1 failing test + 1 long-standing F2-27)
- HIGH: 2 (F3-07, F3-08) — both LOW-priority debt items
- MEDIUM: 9 (mixed quality-of-test issues, mostly redundant smokes with concrete follow-ups)
- LOW: 5 (silent discards, off-by-one domains, dormant ignored tests)
- OBSERVATION: 11 (clean patterns, well-designed new files)
- Total findings: 33
- Round-1+2 regressions (committed HEAD): 4 (F2-04, F2-06, F2-07, F2-27)
- Round-1+2 regressions (working tree): 1 (F2-27)
- Net improvement R2→R3: -29 smoke patterns (R2: 92 → R3: 63) — but wave-8's source tree is contaminated with 3 regressions and 1 failing test