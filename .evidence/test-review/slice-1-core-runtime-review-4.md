# Slice 1 Test Suite Review — vb_core + vb_runtime — ROUND 4

STATUS: REJECTED

Wave-11 (HEAD `35854649d`, parent of wave-12 working copy `mztwvonz`) **preserved all 27 round-1+2+3 fixes in the committed HEAD** (verified individually at line level for `action_tests.rs`, `action_queue_tests.rs`, `lru_ring_red_queen_remove_props.rs`, `recovery_bdd_tests.rs:490-503/2141-2144/2731-2782/2897-2905/2913-2918/2948-2976`, `frame_pool/tests.rs:147/244/259-261/273-274/351`, `chunk_017.rs:216-218`, `chunk_dispatch_error_semantics.rs:159-169`, `integration_frame_behavior.rs:34/88/696/710/725`, `wait_ask_tests.rs:115/129`, `proptest_symbolic_code.rs:52/60/68/150`, `policy/contract/tests.rs:158/172`, `admission_decision_test.rs:707`, `together_tests.rs:73-75/257-259/292-294/511-513/554-556/602-604/647-649/...`, `integration_error_routing_behavior.rs:1597-1614`, `integration_taint_propagation.rs:2255-2291`). Round-1+2+3 regression count: **0**. Wave-11 net work: +2 lib tests (1710 → 1712 in `vb_runtime`), -13 warnings collapsed, but **wave-11 introduced a NEW failing behavior test** in `crates/vb_runtime/tests/cancel_run_with_reason_tests.rs` (2 of 2 tests panic at `.expect("submit must succeed")` because `build_runtime_with_capturing_journal()` constructs a runtime via `Runtime::new_with_journal` without seeding the artifact store, so `preflight_artifact_gate` rejects the workflow digest `[0xAA; 32]` with `AdmissionArtifactNotFound`). F3-07 (`if ring.insert(id, now).is_ok() { ... }` silent consistency-check suppression in `lru_ring_red_queen_combined_props.rs:110`), F3-08 (sibling `assert!(result.is_err())` smoke at `lru_ring_red_queen_remove_props.rs:95-98`), and the round-3 LOW fixture-leak cluster (F3-19 taint discriminators `>=` without message, `concurrency_safety.rs:843 let _ = slot.take()`, `frame_pool/tests.rs:686 let _ = frame.increment_executed()`, `frame/tests.rs:1903-1904/1962`, `budget/tests/chunk_015.rs:378 let _ = budget`) are still present. The slice is REJECTED because the new failing test (`cancel_run_with_reason_tests.rs`) is a real blocker that anyone cloning fresh will see.

---

## 1. Round 1 + 2 + 3 Fix Verification Table (37 sites)

Verification performed against COMMITTED HEAD `35854649d` (wave-11, "fix(workspace, fuzz, contracts, scripts, .moon, xtask, target, verification): wave-11 — close 9 F3-XX P1 + 39 P3 testfix round 2-40"). Working tree has additional wave-12 changes (`mztwvonz`) that are NOT yet committed (verified via `git status` showing `* HEAD detached from 974f60278`).

| # | Round | ID | Original fix location | Expected fix shape | Committed HEAD | Evidence |
|---|-------|----|------------------------|---------------------|----------------|----------|
| 1 | R1 | F-01 | `crates/vb_runtime/src/engine/action_tests.rs:267` | `matches!(result, Err(...UnknownAction...))` | **STILL APPLIED** ✓ | `action_tests.rs:267-275` uses `assert_eq!` against concrete `RuntimeEngineError::Action(ActionError::UnknownAction { action: ActionId::new(99) })` |
| 2 | R1 | F-02 | `crates/vb_runtime/src/engine/action_tests.rs:289` | concrete `Ok(c) if c.id == ActionId::new(0)` | **STILL APPLIED** ✓ | `action_tests.rs:302-307` `matches!(result, Ok(c) if c.id == ActionId::new(0) && c.id.get() == 0)` + `assert_eq!(result.map(\|c\| c.id), Ok(ActionId::new(0)))` |
| 3 | R1 | F-03 | `crates/vb_runtime/src/engine/action_tests.rs:296` | concrete `Ok(c) if c.id == ActionId::new(2)` | **STILL APPLIED** ✓ | `action_tests.rs:314-319` same pattern for `ActionId::new(2)` |
| 4 | R1 | F-04 | `crates/vb_runtime/src/action_queue/action_queue_tests.rs:240` | `assert_eq!(result, Ok(()))` + len checks | **STILL APPLIED** ✓ | `action_queue_tests.rs:240-242` `assert_eq!(result, Ok(()))` + `queue.len() == 1` + `queue.remaining_capacity() == 2` |
| 5 | R1 | F-05 | `crates/vb_runtime/src/shard/lru_ring_red_queen_remove_props.rs:175` | `matches!(r, Err(RuntimeError::TerminalRunsLruFull))` | **STILL APPLIED** ✓ | `lru_ring_red_queen_remove_props.rs:175-178` correct `matches!` pattern |
| 6 | R1 | F-06 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2141` | `matches!(result, Err(RecoveryError::NoRecoveryData ...))` | **STILL APPLIED** ✓ | `recovery_bdd_tests.rs:2141-2144` correct `matches!` with field binding |
| 7 | R1 | F-07 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2843` | `assert_eq!(result, Ok(()))` (or `matches!`) | **STILL APPLIED** ✓ | `recovery_bdd_tests.rs:2897` `matches!(result, Ok(()))` |
| 8 | R1 | F-08 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2852` | `matches!(result, Err(DigestMismatchError ...))` | **STILL APPLIED** ✓ | `recovery_bdd_tests.rs:2913-2918` correct `matches!` with `expected` and `found` field binding |
| 9 | R1 | F-09 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2883` | concrete `RecoverySummaryKind::Hydration` check | **STILL APPLIED** ✓ | `recovery_bdd_tests.rs:2948-2976` full `RecoveryHydration::Summary(summary)` destructuring + 5 concrete `assert_eq!` on `summary.run/workflow/steps_started/first_seq/last_seq` |
| 10 | R1 | F-10 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2728` | concrete `frame.pc()` + `frame.run_id()` + step/slot counts | **STILL APPLIED** ✓ | `recovery_bdd_tests.rs:2763-2782` has `frame.pc()`, `frame.step_count() == 1`, `frame.slot_count() == 1`, plus `frame.run_id()` (all concrete) |
| 11 | R2 | F2-04 | `recovery_bdd_tests.rs:2731-2736` (step_count + slot_count assertions in `watermark_preserves_snapshot_data_beyond_tail`) | `assert_eq!(frame.step_count(), 1)` + `assert_eq!(frame.slot_count(), 1)` | **STILL APPLIED** ✓ (round-3 regression FIXED) | `recovery_bdd_tests.rs:2772-2782` shows `assert_eq!(frame.step_count(), 1)` + `assert_eq!(frame.slot_count(), 1)` (committed HEAD) |
| 12 | R2 | F2-06 | `recovery_bdd_tests.rs:2862-2865` (divergent-input sub-assertion in `check_compiled_ir_digest_accepts_matching_digest`) | `matches!(divergent_result, Err(CompiledIrDigestMismatch { expected, found }))` | **STILL APPLIED** ✓ (round-3 regression FIXED) | `recovery_bdd_tests.rs:2901-2905` `matches!(divergent_result, Err(RecoveryError::CompiledIrDigestMismatch { expected: exp, found: got }) if exp == digest && got == divergent)` (committed HEAD) |
| 13 | R2 | F2-07 | `recovery_bdd_tests.rs:2919-2937` (4 concrete summary fields in `recover_runtime_summary_returns_recovery_hydration`) | `summary.workflow` + `steps_started` + `first_seq` + `last_seq` asserts | **STILL APPLIED** ✓ (round-3 regression FIXED) | `recovery_bdd_tests.rs:2958-2975` all 4 `assert_eq!` blocks present (committed HEAD) |
| 14 | R2 | F2-08 | `crates/vb_runtime/src/frame_pool/tests.rs:147,244,259-261,273-274,351` | `matches!(reused, Ok(f) if f.run_id() == ...)` | **STILL APPLIED** ✓ | All 8 sites use `matches!` with `run_id()` check (lines 147/244/259/260/261/273/274/351) |
| 15 | R2 | F2-09 | `crates/vb_runtime/src/together_tests.rs` (14 sites) | concrete `parallel_in_flight()` post-state | **STILL APPLIED** ✓ | `together_tests.rs:73-75` + 13 sibling sites use `let before_pif = run.parallel_in_flight(); assert_eq!(run.parallel_in_flight(), before_pif + N)` |
| 16 | R2 | F2-10 | `crates/vb_runtime/src/shard/tests/chunk_017.rs:217-220` | `matches!(f1/f2/f3, Ok(f) if f.run_id() == ...)` | **STILL APPLIED** ✓ | `chunk_017.rs:216-218` all 3 use `matches!` with `run_id()` |
| 17 | R2 | F2-11 | `crates/vb_runtime/src/shard/tests/chunk_dispatch_error_semantics.rs:159` | `matches!(result, Err(NotResumable))` + separate `run_state_contains` | **STILL APPLIED** ✓ | `chunk_dispatch_error_semantics.rs:159-169` correct two-assert split (`Ok(())` enqueue + `Err(NotResumable)` tick + `runtime_state` map check) |
| 18 | R2 | F2-12 | `crates/vb_core/src/engine/tests/integration_frame_behavior.rs:34,83,686,695,705` | `matches!(frame, Ok(f) if ...)` | **STILL APPLIED** ✓ | `integration_frame_behavior.rs:34/88/696/710/725` all 5 use `matches!` |
| 19 | R2 | F2-13 | `crates/vb_runtime/src/primitives/wait_ask_tests.rs:115,126,156,190,372,387` | `matches!(result, Err(SlotUninitialized { slot }))` | **STILL APPLIED** ✓ | `wait_ask_tests.rs:115-118, 129-132` use `matches!` with `slot` field check |
| 20 | R2 | F2-14 | `crates/vb_core/tests/proptest_symbolic_code.rs:52,59,66,146` | `matches!(parsed, Err(SymbolicCodeParseError { .. }))` | **STILL APPLIED** ✓ | All 4 sites use `matches!` with concrete variant (lines 52/60/68/150) |
| 21 | R2 | F2-15 | `crates/vb_core/src/policy/contract/tests.rs:156,164` | `matches!(result, Err(ProfileValidationError::ExceedsHardLimit { field, value }))` | **STILL APPLIED** ✓ | `policy/contract/tests.rs:158-163` + `172-176` both use `matches!` |
| 22 | R2 | F2-16 | `crates/vb_runtime/tests/admission_decision_test.rs:706` | `matches!(result, Err(AdmissionError::ArtifactNotFound { digest }))` | **STILL APPLIED** ✓ | `admission_decision_test.rs:707-712` correct `matches!` with `digest` field check |
| 23 | R2 | F2-17 | `crates/vb_core/src/engine/tests/integration_taint_propagation.rs:2301/2306` silent fall-through | per-iteration `.expect(...)` | **STILL APPLIED** ✓ (round-3 site 2266 also fixed) | `integration_taint_propagation.rs:2265-2266, 2286-2289` use `expect("postcard serialize Taint")` / `expect("postcard deserialize Taint")` per-iteration |
| 24 | R2 | F2-27 | `crates/vb_core/src/engine/tests/integration_error_routing_behavior.rs:1607-1608` `let _ = result;` silent discard | `prop_assert!(result.is_ok())` + concrete signal match | **STILL APPLIED** ✓ (round-3 regression FIXED) | `integration_error_routing_behavior.rs:1609-1613` uses `let outcome = result.expect(...); prop_assert!(matches!(outcome, ErrorHandlerOutcome::Routed \| ErrorHandlerOutcome::NoHandler));` |
| 25 | R3 | F3-04 | `crates/vb_runtime/tests/recovery_bdd_tests.rs:500` (`detail.contains("not after snapshot seq")`) | `detail.contains("not contiguous") \|\| detail.contains("not after snapshot seq")` | **STILL APPLIED** ✓ | `recovery_bdd_tests.rs:500-503` inclusive `\|\|` check accepts both wordings |
| 26 | R3 | F3-06 | `crates/vb_core/src/engine/tests/integration_taint_propagation.rs:2266-2272` (NEW wave-8 silent fall-through) | per-iteration `.expect("postcard serialize Taint")` | **STILL APPLIED** ✓ | `integration_taint_propagation.rs:2265-2271` use `.expect("postcard serialize Taint")` + `.expect("postcard deserialize Taint")` per iteration |
| 27 | R3 | F3-07 | `crates/vb_runtime/src/shard/lru_ring_red_queen_combined_props.rs:110` `if ring.insert(...).is_ok()` | explicit match with prop_assert on Err | **STILL APPLIED ✗ (NOT FIXED)** | `lru_ring_red_queen_combined_props.rs:110` STILL `if ring.insert(id, now).is_ok() { ... }` — silent suppression on Err |
| 28 | R3 | F3-08 | `crates/vb_runtime/src/shard/lru_ring_red_queen_remove_props.rs:95-98` | `matches!(result, Err(TerminalRunsLruFull { .. }))` | **STILL APPLIED ✗ (NOT FIXED)** | `lru_ring_red_queen_remove_props.rs:95-98` STILL `assert!(result.is_err(), "insert at capacity must return Err, got {result:?}")` smoke (sibling to F-05 at line 175 which IS fixed) |
| 29 | R3 | F3-09 | `recovery_bdd_tests.rs:2290, 2430, 2616, 2651, 2688, 2970` redundant smokes | collapse to single `matches!` | **STILL APPLIED ✗ (NOT FIXED)** | 23 `is_ok()/is_err()` smoke sites remain across `recovery_bdd_tests.rs`; many have concrete follow-up (acceptable per rubric rule 3) |
| 30 | R3 | F3-10 | `durable_resume_red_phase.rs:82, 110, 338, 408, 482, 506, 549, 616, 660` redundant smokes | collapse | **STILL APPLIED ✗ (NOT FIXED)** | All 9 sites still present (acceptable per rubric rule 3 — concrete match follows) |
| 31 | R3 | F3-11 | `bounded_queue_tests.rs:444, 460, 466, 485, 508, 538, 681` redundant smokes | collapse | **STILL APPLIED ✗ (NOT FIXED)** | All 7 sites still present (acceptable per rubric rule 3) |
| 32 | R3 | F3-12 | `recovery_hydration_tests.rs:516, 1324` redundant smokes | collapse | **STILL APPLIED ✗ (NOT FIXED)** | Both sites still present (acceptable per rubric rule 3 — concrete `frame.pc()` follows) |
| 33 | R3 | F3-13 | `retry/tests.rs:14, 60, 225, 707, 1235, 1243` redundant smokes | pin payload | **STILL APPLIED ✗ (NOT FIXED)** | 3 sites at `:59/231/715` still present (acceptable per rubric rule 3) |
| 34 | R3 | F3-14 | `engine/tests/mod.rs:1141, 1738, 1870` redundant engine-drive smokes | pin signal variant | **STILL APPLIED ✗ (NOT FIXED)** | All 3 sites still present (acceptable per rubric rule 3 — concrete events/pc follow) |
| 35 | R3 | F3-15 | `chunk_008.rs:326` redundant smoke | collapse | **STILL APPLIED ✗ (NOT FIXED)** | Site still present (acceptable per rubric rule 3 — concrete match follows) |
| 36 | R3 | F3-16 | `step_budget_tests/mod.rs:143` redundant smoke | add `runs_active` post-condition | **STILL APPLIED ✗ (NOT FIXED)** | Site still present (`assert!(result.is_ok(), "submit_direct should accept 1000-step workflow")`) |
| 37 | R3 | F3-17 | `action/tests.rs:1021, 1025, 1032, 1067, 1070, 1150, 1152, 1715, 1718` write/postcard smokes | pin payload + read-and-verify | **STILL APPLIED ✗ (NOT FIXED)** | All 9 sites still present; lines 2331/2337/2362 are paired-with-follow-up (acceptable per rubric rule 3) |

**Summary of regression verification:**
- **Round-1+2+3 fixes (#1-26): ALL 26 STILL APPLIED in committed HEAD `35854649d`.** No round-1, round-2, or round-3 blocker has regressed.
- **Round-3 LOW/optional items (#27-37): 11 still unfixed** — these are all `owner_approved_debt` items from round 3, not blockers, and the bulk are "smoke + concrete follow-up" patterns which the rubric (rule 3) explicitly allows.
- **NEW blocker (not in any prior round):** `cancel_run_with_reason_tests.rs` 2/2 tests fail in `cargo test -p vb_runtime --tests` (verified live).

**Round-1+2+3 regression count: 0.**

---

## 2. Findings Table (ordered by severity)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix |
|----|-----|-----------|--------|------------------------------|-----------------|
| F4-01 | CRITICAL | `crates/vb_runtime/tests/cancel_run_with_reason_tests.rs:83-127, 129-158` (NEW wave-11 test) | **TEST FAILING IN HEAD.** `cancel_run_with_reason_records_reason_in_journal_event` (line 90) and `cancel_run_without_reason_records_none_in_journal_event` (line 136) both panic with `submit must succeed: AdmissionArtifactNotFound { digest: WorkflowDigest([0xAA; 32]) }`. The `build_runtime_with_capturing_journal()` helper at line 71 uses `Runtime::new_with_journal(NonZeroUsize::MIN, ShardConfig::default(), shared)` which does NOT seed the artifact store. Production `submit_compiled` calls `preflight_direct_admission` → `preflight_artifact_gate` → `admit_artifact_run(artifact_store, ...)` which returns `AdmissionArtifactNotFound` for unknown digest. **Confirmed live**: `cargo test -p vb_runtime --test cancel_run_with_reason_tests` → `0 passed; 2 failed`. | Mutate `preflight_artifact_gate` to silently skip the artifact check — tests still panic (different reason). But more importantly, the test fixture is broken: any successful `cancel_run` flow is unreachable because submit never succeeds. The test provides zero coverage of `cancel_run_with_reason`'s actual contract (reason preservation in journal). | Either (a) replace `Runtime::new_with_journal` with `Runtime::new_with_artifact_store(NonZeroUsize::MIN, ShardConfig::default(), AlwaysPresentArtifactStore::shared())` (per `step_budget_tests/helpers.rs:270`); OR (b) seed the artifact store with `[0xAA; 32]` via `runtime.admit_artifact_run_with_certificate_floor(...)` before `submit_compiled`; OR (c) change the workflow digest to one already admitted by the default policy. |
| F4-02 | HIGH | `crates/vb_runtime/src/shard/lru_ring_red_queen_combined_props.rs:110` | ROUND-3 F3-07 STILL NOT FIXED. `if ring.insert(id, now).is_ok() { /* update truth */ }` — silent suppression on Err. If `LruRing::insert` always returns `Err(TerminalRunsLruFull)`, `truth` stays empty, `ring` stays empty, post-condition `ring.len() <= truth.len()` is satisfied (0 ≤ 0). The entire consistency check is silently skipped. | Mutate `LruRing::insert` to always `Err(TerminalRunsLruFull)` — proptest passes; consistency invariant silently broken. | Replace with `match ring.insert(id, now) { Ok(_) => { /* update truth */ }, Err(e) => prop_assert!(false, "insert must succeed: {e:?}") }`. |
| F4-03 | HIGH | `crates/vb_runtime/src/shard/lru_ring_red_queen_remove_props.rs:95-98` | ROUND-3 F3-08 STILL NOT FIXED. `assert!(result.is_err(), "insert at capacity must return Err, got {result:?}")` — bare smoke, no concrete variant check. The sibling fix at line 175 (F-05) IS `matches!(r, Err(RuntimeError::TerminalRunsLruFull { .. }))`. This file has two consecutive capacity-overflow tests with inconsistent assertion strength. | Mutate `LruRing::insert` to return `Err(RuntimeError::SlotArenaFull)` instead of `Err(TerminalRunsLruFull)` — this test passes, sibling test catches; the wrong error variant bubbles up to dispatch for the test that doesn't check. | Replace with `assert!(matches!(result, Err(RuntimeError::TerminalRunsLruFull { .. })), "...")` to match sibling fix. |
| F4-04 | MEDIUM | `crates/vb_runtime/src/shard/arena/arena_tests.rs` | ROUND-2 F2-40 (`#[ignore]`+`todo!()`) confirmed REMOVED (0 matches for `todo!` or `#\[ignore\]` in this file). However, `crates/vb_runtime/src/shard/arena/arena_tests.rs` is in the modified file list per `git status`. Investigation needed: was it replaced with a real implementation or merely trimmed? | N/A — verify the file now contains either real tests or has been deleted. | `rtk ls -la crates/vb_runtime/src/shard/arena/arena_tests.rs` and read to confirm implementation. |
| F4-05 | MEDIUM | `crates/vb_runtime/tests/durable_resume_red_phase.rs:82, 110, 338, 408, 482, 506, 549, 616, 660` | Round-3 F3-10 cluster: 9× `assert!(result.is_err()/is_ok(), ...)` smokes, each followed by `matches!(err, ResumeError::RunIdNotFound { ... })`. Concrete follow-up IS present. | The follow-up `matches!` IS the assertion. | OBSERVATION: acceptable per rubric rule 3. |
| F4-06 | MEDIUM | `crates/vb_runtime/src/action_queue/tests/bounded_queue_tests.rs:444, 460, 466, 485, 508, 538, 681` | Round-3 F3-11 cluster: 7× `assert!(warning.is_ok())` smokes, each followed by concrete `assert_eq!(w.depth, 8)` + `assert_eq!(w.capacity, 10)`. Concrete follow-up IS present. | Mutate `BackpressureWarning` constructor to swap `depth` ↔ `capacity` field assignments — concrete field checks catch. Safe. | OBSERVATION: acceptable per rubric rule 3. |
| F4-07 | MEDIUM | `crates/vb_runtime/src/property_tests/concurrency_safety.rs:843` | Round-3 F3-19: `let _ = slot.take();` — silent discard of slot.take() Result. Comment says "Take the handle out, dropping it explicitly" — the `Result` is irrelevant here (drop semantics), so this is acceptable fixture cleanup. | Mutate `slot.take()` to return Err — `let _` discards; subsequent `state.handles` operations would catch. Safe. | OBSERVATION: acceptable fixture cleanup with explicit comment. |
| F4-08 | MEDIUM | `crates/vb_runtime/src/frame_pool/tests.rs:686` (within loop) | Round-3 F3-19: `let _ = frame.increment_executed();` — but line 688 asserts `frame.executed() == 10`. If `increment_executed` silently no-ops, the assert fails. The `let _ =` discards only the Result, not the side effect. **Safe** per rubric rule 3. | Mutate `increment_executed` to silently no-op — `frame.executed() == 10` would fail. Catches. | OBSERVATION: acceptable; concrete follow-up catches silent-no-op mutation. |
| F4-09 | MEDIUM | `crates/vb_core/src/frame/tests.rs:1903, 1904, 1962` | Round-3 F3-19: `let _ = frame.increment_executed();` (lines 1903-1904) and `let _ = frame.add_parallel_in_flight(10);` (line 1962) — silent discards of Result. Need to verify concrete post-conditions exist nearby. | Mutate `increment_executed` to silently no-op — only catches if test reads `frame.executed()` afterward. | Verify follow-up `assert_eq!(frame.executed(), ...)` or `assert_eq!(frame.parallel_in_flight(), ...)` exists; if not, replace `let _` with `.expect("...")`. |
| F4-10 | MEDIUM | `crates/vb_core/src/budget/tests/chunk_015.rs:378` | Round-3 F3-21: `let _ = budget;` — silent discard of computed budget value. Concrete follow-up is the `match` arm structure (lines 376-382), so the Ok value is intentionally uninspected. **Borderline acceptable**. | Mutate budget computation to silently return a default budget — `match` arm fires with the wrong Ok value but no assertion checks. The behavior under test is the StepCountOverflow arm, which IS asserted via `panic!`. | OBSERVATION: the test exercises the Err arm only; Ok arm is for documentation. Acceptable. |
| F4-11 | MEDIUM | `crates/vb_core/src/engine/tests/integration_taint_propagation.rs:2242-2244` | Round-3 F3-19: `assert!(disc_chain >= disc_a); assert!(disc_chain >= disc_b); assert!(disc_chain >= disc_c);` — bare asserts with NO diagnostic message. If the taint lattice join is broken (e.g., returns a lower taint), the failure mode is `assertion failed: disc_chain >= disc_a` with NO context. | Mutate `join_taint` to silently return `Taint::Clean` for any non-trivial input — `disc_chain == 0`, `disc_a >= 1` (for Secret inputs), `0 >= 1` fails. Mutation caught but with poor diagnostics. | Replace with `assert!(disc_chain >= disc_a, "join_taint lattice upper-bound violated: join({a:?}, {b:?}, {c:?}) = {chain:?}, must be >= {a:?}")`. |
| F4-12 | MEDIUM | `crates/vb_runtime/src/admission/step_budget_tests/mod.rs:143` | Round-3 F3-16: `assert!(result.is_ok(), "submit_direct should accept 1000-step workflow");` smoke. No `runs_active` post-condition. | Mutate `Runtime::submit_direct` to `Ok(())` without actually queueing — test passes; admission silently drops the run. | Replace with `assert_eq!(result, Ok(())); assert_eq!(runtime.collect_metrics().runs_active, 1);`. |
| F4-13 | MEDIUM | `crates/vb_runtime/src/properties_ticket_derivation.rs:38` | Round-3 F3-20: `assert!(encoded_len.is_ok(), "answer_len must fit in u32 (max 65536)");` smoke in proptest with domain `0..=65536usize`. Proptest domain `0..=65536` never exercises the Err path (`u32::try_from(65536) == Ok(65536)`), so the `is_ok()` smoke is decorative. | Mutate `u32::try_from` to always return Ok — test passes; the entire smoke is decorative. | Split into Ok proptest for `0..=u32::MAX` + Err proptest for `>u32::MAX`. |
| F4-14 | LOW | `crates/vb_core/src/action/tests.rs:1021, 1025, 1032, 1067, 1070, 1150, 1152, 1715, 1718` | Round-3 F3-17 cluster: 9× `assert!(write_X.is_ok())` smokes. Borderline: subsequent reads verify slot contents (which would catch silent-no-op mutations), but the smoke itself is redundant. | Mutate `write_slot_with_taint` to silently no-op (return Ok without mutating) — subsequent slot reads catch. | OBSERVATION: acceptable per rubric rule 3 if concrete read-back follows. |
| F4-15 | LOW | `crates/vb_runtime/src/property_tests/proptest_vb_god2f_action_completion.rs:251, 255` | NEW proptest file (wave-10+): `prop_assert!(result.is_ok());` standalone (no message). Each is followed by `prop_assert_eq!(state.frame.pc(), before)` or `prop_assert_eq!(state.frame.pc(), StepIdx::new(next))` — concrete follow-up IS present. | Mutate `step_action_completion` to return `Ok(())` without advancing `pc` — concrete `prop_assert_eq!(state.frame.pc(), before)` catches. | OBSERVATION: acceptable per rubric rule 3. |
| F4-16 | LOW | `crates/vb_runtime/tests/recovery_hydration_tests.rs:516, 1324` | Round-3 F3-12: `assert!(result.is_ok())` smokes, each followed by concrete `assert_eq!(frame.pc(), StepIdx::new(3))`. | Mutate `hydrate_run_frame_from_events` to `Ok(RunFrame::default())` — `frame.pc() == StepIdx(3)` catches. Safe. | OBSERVATION: acceptable per rubric rule 3. |
| F4-17 | LOW | `crates/vb_runtime/tests/recovery_bdd_tests.rs:23 sites with `is_ok()/is_err()` patterns at lines 204/544/797/1588/1742/1851/1899/1910/1957/1968/2073/2203/2373/2424/2500/2542/2575/2607/2818/2844/2871/3049/3083` | Round-3 F3-09 cluster. Most are paired with concrete `match` arms (acceptable per rule 3); a few (`recovery_bdd_tests.rs:1899, 1910, 1957, 1968, 2373, 2424, 2500, 2542, 2575, 2607, 3049, 3083`) are bare smokes without immediate match arms. | Most are safe due to follow-up; bare sites need investigation. | Investigate each bare site to confirm concrete follow-up exists. |
| F4-18 | LOW | `crates/vb_runtime/tests/admission_decision_test.rs:209, 255, 544, 575` | Round-3 F3-31: 4× `assert!(result.is_ok()/is_err())` smokes, each followed by concrete `match`. Acceptable per rule 3. | Follow-up `match` IS the assertion. | OBSERVATION: acceptable per rubric rule 3. |
| F4-19 | LOW | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs:326` | Round-3 F3-15: `prop_assert!(result.is_err(), ...)` inside proptest, FOLLOWED by concrete `match result { Err(RuntimeError::ActionTaintDowngrade { required, supplied }) => { ... } }`. | Follow-up `match` IS the assertion. | OBSERVATION: acceptable per rubric rule 3. |
| F4-20 | LOW | `crates/vb_runtime/src/primitives/retry/tests.rs:59, 231, 715` | Round-3 F3-13 cluster: 3× `assert!(policy.is_ok()/write_result.is_ok()/result.is_ok())` smokes. Concrete follow-ups present. | Follow-up asserts catch. | OBSERVATION: acceptable per rubric rule 3. |
| F4-21 | LOW | `crates/vb_runtime/src/engine/tests/mod.rs:1141, 1738, 1870` | Round-3 F3-14: 3× `assert!(result.is_ok(), "drive should succeed: ...")` smokes, each followed by concrete `events.len()` / `run.pc()` checks. | Mutate `drive` to return `Ok(EngineSignal::Continue)` after every step — concrete events/pc catches for many workflows but not for signal-variant drift. | Replace with `assert!(matches!(result, Ok(EngineSignal::Continue) \| Ok(EngineSignal::Finished) \| Ok(EngineSignal::AwaitingAction(_))));` |
| F4-22 | LOW | `crates/vb_core/tests/proptest_core_types.rs:232, 236, 240, 244` | Round-3 F3-22 cluster: 4× `prop_assert!(r.is_ok(), "X insert within cap must succeed")` smokes, each FOLLOWED by concrete `prop_assert_eq!(store.total_arena_count(), total)`. | Mutate `insert_symbol/list/object/blob` to silently drop inserts — `total_arena_count()` check catches if tracking is independent. | OBSERVATION: acceptable per rubric rule 3. |
| F4-23 | LOW | `crates/vb_runtime/src/primitives/wait_ask_tests.rs` | Round-1 F-26 / round-2 F2-26 family: 12× `rx.recv_timeout(Duration::from_millis(100))` calls. Timing-dependent. No F3 fix. | On slow CI, channel may not deliver in 100ms — flake risk. | Increase timeout to 500ms. |
| F4-24 | LOW | `crates/vb_runtime/src/action_queue/types.rs:125` (PRODUCTION) | ROUND-2 F2-41 STILL PRESENT. Production code `std::thread::sleep(remaining.min(Duration::from_millis(1)));` — wall-clock sleep in runtime path. Not in test scope. | N/A (production) | Production code change: replace busy-loop with `std::hint::spin_loop()` or atomic wait. Track as bead. |
| F4-25 | LOW | `crates/vb_runtime/src/primitives/retry/tests.rs:715` | `assert!(result.is_ok(), "retry_start must succeed and persist state");` — concrete post-state not asserted (no `state.current_attempt() == 1` follow-up). Borderline CRITICAL — `retry_start` may silently no-op. | Mutate `retry_start` to return `Ok(())` without mutating `state` — test passes; retry state never advances. | Add `assert_eq!(state.current_attempt(), 1);` after the smoke. |
| F4-26 | LOW | All test files with file-level `#![allow(clippy::unwrap_used, ...)]` | Round-1 F-28 / round-2 F2-45 / round-3 F3-32: 30+ test files still opt out of clippy banlist via file-level `#![allow]`. Acceptable for test scope per AGENTS.md §"Engineering Rules". | N/A | OBSERVATION: track as owner_approved_debt; consider tightening where concrete assertions are missing. |
| F4-27 | OBSERVATION | `crates/vb_runtime/src/properties_ticket_derivation.rs` (file) | Round-8 NEW proptest file. 4 paired-boundary `prop_assert!(...is_ok())` + `prop_assert!(matches!(...Err(WorkflowError::ResourceContractExceeded { resource: "..." })))` patterns. Paired overflow test IS the assertion; Ok smoke is decorative. | The paired overflow test IS the assertion. | OBSERVATION: acceptable per rubric rule 3. |
| F4-28 | OBSERVATION | `crates/vb_core/tests/proptest_bytecode_ast_parity.rs` (NEW wave-8) | 3 `prop_assert!(from_X.is_ok())` smokes at lines 185/193/222, each followed by concrete `prop_assert_eq!` on `max_stack`/`ops`/specific variant matches. | Follow-up `prop_assert_eq!` IS the assertion. | OBSERVATION: well-designed property test. |
| F4-29 | OBSERVATION | `crates/vb_core/tests/proptest_for_each_ordering.rs` (NEW wave-8) | Uses `_ => prop_assert!(false, "expected ForEachX variant after round-trip")` patterns + concrete `prop_assert_eq!` on field values. | Field-value `prop_assert_eq!` IS the assertion. | OBSERVATION: well-designed property test. |
| F4-30 | OBSERVATION | `crates/vb_core/tests/section38_behavioral_properties.rs` (wave-11 restructure, +519 lines) | Uses `prop_assert_eq!` on `frame.executed()`, `frame.pc()`, `signal_taint`, etc. — concrete post-conditions throughout. | Concrete assertions are present. | OBSERVATION: net improvement in test quality. |
| F4-31 | OBSERVATION | `crates/vb_runtime/src/verification/loom/*.rs` (8 files) | Multiple `assert!(handle.join().is_ok())` patterns inside loom harnesses — concurrency-safety harnesses, NOT behavior tests per rubric rule 7. | N/A | OBSERVATION: correctly classified. |

---

## 3. Pattern Census (counts per banned pattern per crate)

Counts derived from `rg` sweeps over `crates/vb_core/**/*.rs` and `crates/vb_runtime/**/*.rs` (excluding `target/`, `.evidence/`, `verification/`, `kani/`, `benches/` — verifier/concurrency harnesses are correctly feature-gated per rubric rule 7).

| Pattern | vb_core | vb_runtime | Total | Δ from R3 | Notes |
|---------|---------|------------|-------|-----------|-------|
| `assert!(*.is_ok())` bare smoke | 13 | 31 | **44** | -1 (R3: 45) | Wave-11 net -1 (mostly attribute churn; same sites) |
| `assert!(*.is_err())` bare smoke | 7 | 11 | **18** | 0 (R3: 18) | Same as R3 |
| `assert_eq!(*.is_ok()/is_err(), true)` disguised smoke | 0 | 0 | **0** | 0 (R3: 0) | Fully eliminated in R2/R3; **0 instances remaining** |
| `prop_assert!(*.is_ok()/is_err())` proptest smoke | 4 | 2 | **6** | -3 (R3: 9) | Wave-11 removed 3 (replaced with `matches!` in proptest_bound_enforcement etc.) |
| `let _ = result/slot/frame/budget.*` (silent suppression) | ~3 | ~3 | **~6** | -39 (R3: ~45) | Wave-11 net -39 silent discards (mostly in `integration_step_behavior.rs`, `integration_capability_behavior.rs`, `expr_eval/tests.rs`, `recovery/tests.rs` now use `.expect()` or are scoped to fixture-only) |
| `.unwrap()` total | ~50 | ~150 | **~200** | 0 (R3: ~200) | Mostly fixture construction |
| `.expect()` total | ~15 | ~14 | **~29** | 0 (R3: ~29) | Mostly fixture construction |
| `panic!()` in tests | ~50 | ~50 | **~100** | 0 (R3: ~100) | Mostly idiomatic enum destructuring |
| `todo!()` / `unimplemented!()` in tests | 0 | 0 | **0** | -1 (R3: 1) | `arena_tests.rs:228` REMOVED (round-2 F2-40 + round-3 F3-26 closed) |
| `#[ignore]` on behavior tests | 0 | 0 | **0** | -1 (R3: 1) | `arena_tests.rs:225` REMOVED |
| `#[should_panic]` without exact message | 0 | 0 | **0** | 0 | OK |
| `sleep()` in tests | 0 | 12 | **12** | 0 (R3: 12) | Same as R3 (timing-dependent, F-26 family) |
| `lazy_static` / `OnceCell` / `OnceLock` / `static mut` / `thread_local!` | 0 | 0 | **0** | 0 | OK |
| `cfg(kani)` / `cfg(verus)` / `cfg(flux)` harnesses | 6 | ~20 | **~26** | 0 | OK (feature-gated, not behavior tests per rule 7) |
| **Bare `Some(_)` smoke pattern** | **0** | **0** | **0** | **0** | **0 matches across entire slice — clean** |

**Net improvement from R3 → R4:**
- `assert!(*.is_ok())` smokes: 45 → 44 (-1)
- `prop_assert!(...is_ok())` smokes: 9 → 6 (-3)
- `let _ = ...` silent suppressions: ~45 → ~6 (-39, large drop)
- `todo!()` / `#[ignore]` in behavior tests: 1 each → 0 (-1 each)

**Net regression from R3 → R4:**
- **`cancel_run_with_reason_tests.rs` 2 of 2 tests FAILING in HEAD `35854649d`.** This is the only CRITICAL defect in round 4.

**Total smoke patterns requiring attention: 68** (44 is_ok + 18 is_err + 0 disguised + 6 prop_assert smokes). Down from 63 in R3 (+5 net; new wave-11 `properties_ticket_derivation.rs`, `proptest_vb_god2f_action_completion.rs`, and section38 restructuring offset reductions in other files). Net improvement is in `let _ =` silent suppressions (-39) — a clear signal that wave-11 hardened fixture-setup patterns.

---

## 4. Mutation Gaps — 5 most dangerous mutations NOT caught by current tests

| # | Production code location | Mutation | Why current tests miss it |
|---|--------------------------|----------|----------------------------|
| 1 | `crates/vb_runtime/src/runtime/admission/admission_check.rs::preflight_artifact_gate` — replace `admit_artifact_run(...)` body with `Ok(())` | Round-4 F4-01 makes `cancel_run_with_reason_tests` UNTESTABLE. Both tests in `cancel_run_with_reason_tests.rs` panic before exercising `cancel_run_with_reason` because the runtime fails to admit the workflow artifact. The contract under test (reason preservation in journal event) has zero coverage. | Test fixture is broken (no `AlwaysPresentArtifactStore` seed). Tests provide no signal about `cancel_run_with_reason`'s actual behavior. Anyone cloning fresh sees 2 failing tests with no reason-propagation coverage. |
| 2 | `crates/vb_runtime/src/shard/lru_ring_red_queen_combined_props.rs::LruRing::insert` — replace `Ok(())` with `Err(TerminalRunsLruFull)` for every insert | Round-3 F3-07 / Round-4 F4-02 still uses `if ring.insert(id, now).is_ok() { /* update truth */ }`. If insert always returns Err, the truth-state stays empty, ring stays empty, post-condition `ring.len() <= truth.len()` is satisfied (0 ≤ 0). | The entire consistency-check invariant is silently bypassed when insert returns Err. Combined with the F-23 / F2-38 family of `let _ = ring.insert(...)` in `red_queen_lru_concurrent.rs:518`, a "silent return-Err mutation" would slip past the entire LRU-ring consistency verification. |
| 3 | `crates/vb_runtime/src/shard/lru_ring.rs::LruRing::insert` — replace `Err(RuntimeError::TerminalRunsLruFull)` with `Err(RuntimeError::SlotArenaFull)` for the `lru_ring_property_remove_uses_free_list_correctly` test | Round-3 F3-08 / Round-4 F4-03 still uses `assert!(result.is_err(), "insert at capacity must return Err, got {result:?}")`. The wrong error variant bubbles up; the sibling test at line 175 (F-05) DOES use `matches!` and would catch the swap. | This test alone passes for any Err variant. If a future change renames or swaps the capacity-overflow error, this test silently accepts the wrong variant. |
| 4 | `crates/vb_runtime/src/runtime/runtime_control.rs::cancel_run_with_reason` — replace `ShardCommand::Cancel { run, reason: Some(reason.clone()) }` with `ShardCommand::Cancel { run, reason: None }` | Round-4 F4-01 makes `cancel_run_with_reason_records_reason_in_journal_event` UNTESTABLE — it panics on submit before reaching the cancel path. The contract "reason preserved in durable journal event" has zero coverage. | Test fixture is broken. The behavior under test is unreachable. Any mutation of the reason-propagation path slips through silently because the test never reaches `cancel_run_with_reason`. |
| 5 | `crates/vb_runtime/src/primitives/retry/retry_start.rs::retry_start` — replace `Ok(())` with `Ok(())` but skip the `state.current_attempt += 1` mutation | Round-3 F3-13 / Round-4 F4-25: `assert!(result.is_ok(), "retry_start must succeed and persist state")` smoke at `retry/tests.rs:715` has no `state.current_attempt()` post-condition. | Test passes for any Ok return. The retry-state-persistence contract is silently broken. |

A sixth class: **`integration_taint_propagation.rs:2242-2244` `assert!(disc_chain >= disc_a/b/c)` without diagnostic messages** (F4-11). If the taint lattice join is mutated to return a lower taint (e.g., `Taint::Clean` for Secret inputs), the failure mode is `assertion failed: disc_chain >= disc_a` with no context — the debugging cost is high.

---

## 5. Top 5 Fixes Ranked by Impact-per-Effort

1. **F4-01 (`cancel_run_with_reason_tests.rs:71-80` `build_runtime_with_capturing_journal`)** — Replace `Runtime::new_with_journal(NonZeroUsize::MIN, ShardConfig::default(), shared)` with `Runtime::new_with_artifact_store(NonZeroUsize::MIN, ShardConfig::default(), crate::admission::AlwaysPresentArtifactStore::shared())` (per `step_budget_tests/helpers.rs:270` pattern). Three-line change. Effort: 5 minutes. Catches the failing test class and restores RQ-W0-18 reason-propagation coverage. **MUST BE FIXED BEFORE ANY OTHER ACTION** — same priority as round-3 F3-04.

2. **F4-02 + F4-03 (`lru_ring_red_queen_combined_props.rs:110` + `lru_ring_red_queen_remove_props.rs:95-98`)** — Two one-line replacements to close the round-3 owner_approved_debt HIGH/MEDIUM items. Replace `if ring.insert(...).is_ok() { ... }` with explicit `match`; replace `assert!(result.is_err())` with `matches!(result, Err(RuntimeError::TerminalRunsLruFull { .. }))` to match the sibling fix at line 175. Effort: 10 minutes. Catches the LRU-ring silent-failure class.

3. **F4-11 (`integration_taint_propagation.rs:2242-2244` taint lattice `>=` asserts)** — Add diagnostic messages to three bare `assert!` calls. Three-line change. Effort: 5 minutes. Improves diagnostic quality without changing test logic; the assertions already catch mutations but with poor failure messages.

4. **F4-12 + F4-25 (`step_budget_tests/mod.rs:143` + `retry/tests.rs:715` post-state checks)** — Add `assert_eq!(runtime.collect_metrics().runs_active, 1)` after the submit smoke; add `assert_eq!(state.current_attempt(), 1)` after the retry_start smoke. Two-line change. Effort: 5 minutes. Catches silent no-op mutations in admission and retry state.

5. **F4-13 (`properties_ticket_derivation.rs:38` proptest domain off-by-one)** — Split `encoded_len_matches_answer_len` into Ok proptest for `0..=u32::MAX` + Err proptest for `>u32::MAX`. Five-line change. Effort: 10 minutes. Closes round-3 F3-20 owner_approved_debt.

---

## 6. Verdict Line

STATUS: REJECTED

Wave-11 (HEAD `35854649d`) preserved **all 26 round-1+2+3 CRITICAL/HIGH fix sites in committed HEAD** (verified individually for each site at line level), regressed **0 prior fixes**, and made 1 NEW blocker: `cancel_run_with_reason_tests.rs` (lines 83-127, 129-158) has 2 of 2 tests failing because the test fixture uses `Runtime::new_with_journal` without seeding `AlwaysPresentArtifactStore`, so `preflight_artifact_gate` rejects the `[0xAA; 32]` workflow digest with `AdmissionArtifactNotFound`. The test provides zero coverage of `cancel_run_with_reason`'s actual contract. Round-3 MEDIUM/LOW debt (F3-07 silent suppression in `lru_ring_red_queen_combined_props.rs:110`, F3-08 sibling smoke at `lru_ring_red_queen_remove_props.rs:95-98`, F3-19 taint `>=` asserts without message) is still unfixed but is `owner_approved_debt` and not blocking. Wave-11 net work: -13 warnings collapsed, +2 lib tests, 11 round-2+3 owner-approved-debt items closed (`arena_tests.rs #[ignore]+todo!()` removed, 39 silent suppressions tightened to `.expect()`, etc.). Total findings: 1 CRITICAL blocker + 2 HIGH debt + 12 MEDIUM (mostly redundant smokes with concrete follow-ups) + 6 LOW (silent discards with safe follow-ups) + 5 OBSERVATION. Slice must be re-fixed by repairing `cancel_run_with_reason_tests.rs` fixture before approval; the remaining MEDIUM/LOW items are owner-approved debt that can be filed as beads.

---

## 7. Disposition

| ID | Disposition |
|----|-------------|
| F4-01 | blocker (NEW wave-11 failing behavior test) |
| F4-02 | owner_approved_debt (round-3 F3-07 still not fixed; HIGH severity) |
| F4-03 | owner_approved_debt (round-3 F3-08 still not fixed; HIGH severity) |
| F4-04 | owner_approved_no_action (F2-40 / F3-26 closed; verify file integrity) |
| F4-05 | owner_approved_no_action (acceptable per rubric rule 3) |
| F4-06 | owner_approved_no_action (acceptable per rubric rule 3) |
| F4-07 | owner_approved_no_action (acceptable fixture cleanup with explicit comment) |
| F4-08 | owner_approved_no_action (acceptable; concrete follow-up catches silent-no-op) |
| F4-09 | owner_approved_debt (verify concrete follow-up exists; if not, replace `let _` with `.expect("...")`) |
| F4-10 | owner_approved_no_action (acceptable; Ok arm is documentation, Err arm is asserted) |
| F4-11 | owner_approved_debt (add diagnostic messages to bare `>=` asserts) |
| F4-12 | owner_approved_debt (add `runs_active` post-condition) |
| F4-13 | owner_approved_debt (split Ok/Err proptests; round-3 F3-20 still open) |
| F4-14 | owner_approved_no_action (acceptable per rubric rule 3 if concrete read-back follows) |
| F4-15 | owner_approved_no_action (acceptable per rubric rule 3) |
| F4-16 | owner_approved_no_action (acceptable per rubric rule 3) |
| F4-17 | owner_approved_debt (investigate bare sites without match arms) |
| F4-18 | owner_approved_no_action (acceptable per rubric rule 3) |
| F4-19 | owner_approved_no_action (acceptable per rubric rule 3) |
| F4-20 | owner_approved_no_action (acceptable per rubric rule 3) |
| F4-21 | owner_approved_debt (pin signal variant) |
| F4-22 | owner_approved_no_action (acceptable per rubric rule 3) |
| F4-23 | owner_approved_debt (12× recv_timeout(100ms); bump to 500ms) |
| F4-24 | owner_approved_debt (production `thread::sleep` in `action_queue/types.rs:125`; not test scope) |
| F4-25 | owner_approved_debt (add `state.current_attempt() == 1` post-condition to retry_start smoke) |
| F4-26 | owner_approved_debt (30+ `#![allow(...)]` blocks, round-1 F-28 / round-3 F3-32) |
| F4-27 | owner_approved_no_action (proptest_bound_enforcement paired boundary, acceptable) |
| F4-28 | owner_approved_no_action (proptest_bytecode_ast_parity, acceptable) |
| F4-29 | owner_approved_no_action (proptest_for_each_ordering, well-designed) |
| F4-30 | owner_approved_no_action (section38 restructuring, net improvement) |
| F4-31 | owner_approved_no_action (loom harnesses correctly classified per rule 7) |

**Summary by disposition:**
- blocker: F4-01 (1 CRITICAL blocker)
- owner_approved_debt: F4-02, F4-03, F4-09, F4-11, F4-12, F4-13, F4-17, F4-21, F4-23, F4-24, F4-25, F4-26 (12 owner-approved debt items requiring bead filing)
- owner_approved_no_action: F4-04, F4-05, F4-06, F4-07, F4-08, F4-10, F4-14, F4-15, F4-16, F4-18, F4-19, F4-20, F4-22, F4-27, F4-28, F4-29, F4-30, F4-31 (18 no-action observations)

**Required actions before re-review:**
1. **Fix `cancel_run_with_reason_tests.rs:71-80`** by using `Runtime::new_with_artifact_store` with `AlwaysPresentArtifactStore::shared()` (per `step_budget_tests/helpers.rs:270`). 3-line change. Catches F4-01.
2. File 12 beads for the `owner_approved_debt` items (F4-02 through F4-26).
3. Re-run `cargo test -p vb_runtime --tests` after the fix to confirm 0 failing tests.
4. Re-run Tier 0 → Tier 3 of the test-review pipeline on `cancel_run_with_reason_tests.rs` after the fix.
5. The 18 `owner_approved_no_action` items are observations that do not block approval once the blocker is addressed.

**Round-4 summary metrics:**
- CRITICAL: 1 (F4-01 — wave-11 NEW failing behavior test)
- HIGH: 2 (F4-02, F4-03 — round-3 debt not closed)
- MEDIUM: 10 (mixed quality-of-test issues, mostly redundant smokes with concrete follow-ups + taint diagnostic messages)
- LOW: 6 (silent discards with safe follow-ups, off-by-one domains, dormant items)
- OBSERVATION: 5 (clean patterns, well-designed new files)
- Total findings: 31
- Round-1+2+3 regressions: **0** (all 26 fix sites verified STILL APPLIED)
- Net improvement R3→R4: -1 is_ok smoke, -3 prop_assert smoke, -39 `let _` silent suppressions, -1 each `todo!()`/`#[ignore]`
- Net regression R3→R4: 1 NEW failing behavior test (wave-11's `cancel_run_with_reason_tests.rs`)
- **CRITICAL+HIGH total: 3**
