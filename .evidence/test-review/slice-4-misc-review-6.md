# Test Review — Slice 4 Round 6 (Misc Crates: vb_expr, vb_ipc, vb_yaml, vb_queue_semantics, vb_boundary_inventory, vb_benchmark, vb_test_util, vb_doc, vb_ajc40_flux, vb_verification)

## STATUS: REJECTED

Round 6 finds **3 NEW CRITICAL defects** (3 newly-failing behavior tests that are correctly catching real production regressions) and confirms **zero regressions** of all round-1+2+3+4+5 fixes. The 8 short-circuit tests (`eval_tests.rs:680, 693, 706, 719, 732, 739, 746, 753`), `vb_ajc40_flux` validator imports (`density_tests.rs:18-27`), `MemoryIngress` disconnect (`tests.rs:445-451`), FIFO run_id order assertion (`array_queue_tests.rs:756-760`), `queue_boundary` wiring (19 tests), `bap_*` bytecode parity (12 tests), and the `tests.rs:2019-2022` variant check all remain in place. The 3 NEW CRITICAL defects are: (1) `vb_ipc::server::handlers::tests::handle_cancel_run_accepts_reason_and_routes_to_runtime` (L355) fails because the production `RunCancelled` journal event is not being recorded with the IPC-supplied reason; (2) `vb_ipc::server::handlers::tests::handle_cancel_run_without_reason_records_no_reason_on_journal` (L409) fails because the `RunCancelled` event is not being recorded at all for the no-reason path; (3) `vb_benchmark::tests::batched_atomicity_tests::coalescing_ratio_at_least_three` (L422) fails because the non-batching shard (window=1) is incorrectly using batch appends (batch_a=100, expected 0). All three tests are well-designed behavior tests that catch real production bugs introduced in wave-5 (commit `da55addc7`). Test counts: vb_expr 904 passed, vb_ipc 652 passed + 2 failed = 654 total, vb_queue_semantics 325 passed, vb_yaml 306 passed, vb_boundary_inventory 191 passed, vb_benchmark 33+1 passed + 1 failed = 35 total, vb_test_util 50 passed, vb_doc 83 passed, vb_ajc40_flux 52 passed, vb_verification 0. `vb_verification` STILL has 0 behavior tests after **6 cycles**. The `let _ = ` pattern in `vb_yaml` (2 sites), `vb_boundary_inventory` (0), `vb_queue_semantics` (0), `vb_ajc40_flux` (0), `vb_test_util` (0), `vb_doc` (0) are unchanged from round 5 — only the 2 documented acceptable sites remain. `vb_ipc` has no new `crossbeam_channel` uses (the 6 crossbeam references in `vb_ipc` are all doc comments + the production `crossbeam_queue::ArrayQueue` import at `ingress.rs:8`).

### Round 1+2+3+4+5 Fix Verification Table

| Round Bead | Finding | File:Line (Expected) | Round-6 Status | Evidence |
|------------|---------|----------------------|----------------|----------|
| vb-0to5y | S4-001/S4-021 (Section 46 short-circuit tests) | `crates/vb_expr/src/eval_tests.rs:680, 693, 706, 719, 732, 739, 746, 753` (8 tests) | **STILL APPLIED** | `cargo test -p vb_expr --lib -- eval_binary_op_and_evaluates_right` runs **1 passed, 884 filtered out**. All 8 short-circuit tests present at expected line numbers. |
| vb-2kw49 | S4-002 (vb_ajc40_flux local validator copies) | Imports from `vb_core::workflow::compiled_slug` | **STILL APPLIED** | `vb_ajc40_flux/tests/density_tests.rs:18-27` imports `validate_compiled_slug_count`, `validate_compiled_slug_summary` from `vb_core::workflow::compiled_slug`. 6 `assert!(false, ...)` sites unchanged (S4-R5-001 carryover). |
| vb-8r7cp | S4-003/S4-018 (forbidden `crossbeam_channel` in `vb_ipc/src/tests.rs:443-455`) | `MemoryIngress::bounded(...)` + `disconnect_sender()` + `assert_eq!(try_recv(), Err(IpcError::Disconnected))` | **STILL APPLIED** | `crates/vb_ipc/src/tests.rs:445-451` uses `MemoryIngress::bounded(QueueCapacity::new(std::num::NonZeroUsize::MIN))` + `disconnect_sender()` + `assert_eq!(try_recv(), Err(IpcError::Disconnected))`. No `crossbeam_channel::bounded` import in code. **6 crossbeam references in vb_ipc: all are `crossbeam_queue::ArrayQueue` (production, Section 50 compliant) or doc comments.** |
| vb-few2x + S4-R2-001 + S4-R3-001 | S4-004 (FIFO order) + S4-R2-001 (queue/tests/array_queue_tests.rs wired) + S4-R3-001 (mod queue; in lib.rs) | `while let Ok(Some(frame))` + `prop_assert_eq!(received run_id order, submitted order)` + file compiled | **STILL APPLIED (FUNCTIONAL)** | `cargo test -p vb_ipc --lib -- array_queue_tests::` runs **33 passed, 621 filtered out**. `fifo_order_invariant_for_submit_recv_cycle` runs and passes with strong order assertion at `array_queue_tests.rs:756-760`. The 3-cycle regression remains closed. |
| S4-R2-002 | `vb_queue_semantics/src/tests/queue_boundary.rs` wired | `#[cfg(test)] mod tests;` in `lib.rs`; `mod queue_boundary;` in `src/tests/mod.rs` | **STILL APPLIED** | `cargo test -p vb_queue_semantics --lib -- queue_boundary::` runs **19 passed, 183 filtered out**. |
| S4-R2-003 | Production bug in `eval_expr_program` for boolean literals | All 12 `proptest_bytecode_ast_parity` tests pass | **STILL APPLIED** | `cargo test -p vb_expr --lib -- bap_bool_literal_parity` runs **1 passed, 884 filtered out**. The 12 parity tests remain green. |
| S4-R3-014 | Type error at `array_queue_tests.rs:774` (now L785) | `while let Ok(Some(_frame))` | **STILL APPLIED** | `crates/vb_ipc/src/array_queue_tests.rs:785`: `while let Ok(Some(_frame)) = ingress.try_recv() { /* drain */ }` (correct). |
| S4-R2-007 | `prop_assert_err!(result)` at `tests.rs:2011` without variant check | Variant match must follow | **STILL APPLIED (round-5 misclassification correction)** | `crates/vb_ipc/src/tests.rs:2017-2023`: `prop_assert_err!(result); if let Err(e) = result { prop_assert!(matches!(e, IpcError::InvalidMagic { .. }), "expected InvalidMagic, got {e:?}"); }` — variant check IS present. |

**Round 1+2+3+4+5 regression count: 0.** All 8 prior fix beads remain in their expected disposition. No defects introduced into previously-fixed code paths. **However, 3 NEW CRITICAL defects are caught by behavior tests that were newly added in wave-5 (commit `da55addc7`)** — these tests pass the rubric and are correctly designed, but they expose production bugs.

### Round 1–5 Findings Status (carryover)

| Round ID | Disposition | Round-6 Status | Evidence |
|----------|-------------|----------------|----------|
| (R1) S4-005 (125 `assert_ok!` macro uses in vb_ipc) | owner_approved_debt | **STILL PRESENT** | 89 in `tests.rs` + 36 in `frame/tests.rs` = 125 (unchanged). |
| (R1) S4-006 / (R3) S4-R3-011 (`vb_yaml/src/lib_tests.rs:1306` smoke `is_ok()`) | owner_approved_debt | **STILL PARTIALLY FIXED** | Round-5 content check at L1309-1314 still in place; smoke `is_ok()` at L1305-1308 remains. |
| (R1) S4-007 (5 `is_ok()` in `vb_boundary_inventory/.../vb_god2f_validation_properties.rs`) | owner_approved_debt | **STILL PRESENT** | L94, 97, 166, 175, 232 (unchanged). |
| (R1) S4-008 (`assert_io_ok` helper in `vb_boundary_inventory/src/tests/api_tests.rs:25-27`) | owner_approved_debt | **STILL PRESENT** | L25-27 helper + 13 call sites + L942 `assert!(dir.is_ok(), ...)`. |
| (R1) S4-009 (16 `is_err()` in `vb_expr/tests/proptest_type_enforcer.rs`) | owner_approved_debt | **STILL PRESENT** | 16 `prop_assert!(...is_err())` calls (unchanged). |
| (R1) S4-011 (`vb_ipc/src/server/handlers/tests.rs` `Err(_) => return`) | owner_approved_debt | **STILL PRESENT** | 7 `Err(_) => return` sites (L189, L255, L334, L346, L389, L401, L465) + 3 multi-line `assert!(snapshot.is_ok(), ...); match snapshot { Ok(events) => events, Err(_) => return };` patterns at L180-190, L246-256, L456-465. |
| (R1) S4-012 (`ast.is_err()` / `tokens.is_err()` in miri_tests) | owner_approved_debt | **STILL PRESENT** | `parser/miri_tests.rs:192, 219, 221` (3 `ast.is_err()`); `lexer/miri_tests.rs:121, 123, 131, 158, 179, 181` (6 `tokens.is_ok()` / `tokens.is_err()`). |
| (R1) S4-013 (smoke `max_stack > 0` in `bytecode/tests.rs:416-426`) | owner_approved_debt | **STILL PRESENT** | L424 `assert!(max_stack > 0, ...)` (unchanged). |
| (R1) S4-014 (smoke `!debug_str.is_empty()` in `error_tests.rs:98-118`) | owner_approved_debt | **STILL PRESENT** | L116 (unchanged). |
| (R1) S4-015 (77 `assert!(false, …)` in vb_boundary_inventory) | owner_approved_debt | **STILL PRESENT (census: 77)** | 43 in `api_tests.rs` + 13 in `validation_tests.rs` + 9 in `parser_tests.rs` + 9 in `property_tests.rs` (round-5 census confirmed). |
| (R1) S4-016 (`fail_assert!` macro in 6 vb_yaml files) | owner_approved_debt | **STILL PRESENT (census: 72)** | Round-6 grep: `events_tests.rs:24` + `lib_tests.rs:17` + `profile_tests.rs:11` + `source_map_tests.rs:10` + `profile_tests_adversarial.rs:6` + `profile_error_variants_tests.rs:4` = **72 invocations** (down from round-5's 90; some `fail_assert!` macro sites may have been removed in wave-5 or earlier rounds — this is not a regression but a count correction). |
| (R1) S4-020 (`vb_verification` has 0 behavior tests) | owner_approved_debt | **STILL UNRESOLVED (6 cycles)** | `cargo test -p vb_verification --tests --lib` reports **0 passed**. No `#[test]` attribute anywhere in `crates/vb_verification/`. Only `#[cfg(kani)] mod kani_harnesses` (3 proofs) + `#[cfg(not(kani))] mod not_kani`. **6 cycles unresolved — the most persistent finding in the slice.** |
| (R2) S4-R2-002 | fixed_with_evidence | **STILL APPLIED** | 19 `queue_boundary::*` tests run. |
| (R2) S4-R2-003 | fixed_with_evidence | **STILL APPLIED** | 12 `bap_*` tests pass. |
| (R3) S4-R3-001 | fixed_with_evidence | **STILL APPLIED** | 33 `array_queue_tests::*` tests run; FIFO order check functional. |
| (R3) S4-R3-002 (6 `assert!(false, …)` in `vb_ajc40_flux`) | owner_approved_debt | **STILL PRESENT** | L202, 211, 220, 237, 246, 255 (unchanged). |
| (R3) S4-R3-003 (16 `unwrap_or_default()` in `vb_yaml/src/source_map_tests.rs`) | owner_approved_debt | **STILL PRESENT** | 16 sites (unchanged). |
| (R3) S4-R3-004 (72 `fail_assert!` in 6 vb_yaml files) | owner_approved_debt | **STILL PRESENT (census corrected: 72 vs. round-5 90)** | Sites confirmed; count may have dropped from 90 to 72 in wave-5 edits. |
| (R3) S4-R3-005 (125 `assert_ok!` in vb_ipc) | owner_approved_debt | **STILL PRESENT** | 89 + 36 = 125 (unchanged). |
| (R3) S4-R3-006 (7 `Err(_) => return` in `handlers/tests.rs`) | owner_approved_debt | **STILL PRESENT** | 7 sites unchanged. |
| (R3) S4-R3-007 (24 `assert!(false, …)` + 8 `is_ok()` in vb_boundary_inventory) | owner_approved_debt | **STILL PRESENT (census: 77 + 8 + 5 = 90)** | See S4-R5-003. |
| (R3) S4-R3-008 (45 `unreachable!()` in `vb_queue_semantics/transitions/tests.rs`) | owner_approved_debt | **STILL PRESENT** | 45 sites (unchanged from round 5). |
| (R3) S4-R3-009 (8 `let _ =` + 8 `Ok(()) => {}` + 2 `is_err()` + 48 `panic!()` in vb_benchmark) | owner_approved_debt | **STILL PRESENT** | 5 `let _ = shard` in tests + 3 in benches; 8 `Ok(()) => {}`; 2 `is_err()`; 48 `panic!()` (unchanged). |
| (R3) S4-R3-010 (12 `panic!()` in `vb_test_util/tests/density_tests.rs`) | owner_approved_debt | **STILL PRESENT** | 12 sites unchanged. |
| (R3) S4-R3-012 (`ast.is_err()` in miri_tests) | owner_approved_debt | **STILL PRESENT** | 9 sites in lexer/parser miri_tests. |
| (R3) S4-R3-013 (5 `prop_assert!().is_err()` in `eval_tests.rs:3421-3425`) | owner_approved_no_action | **STILL PRESENT (acceptable)** | 5 sites. Partition invariant at L3444 catches wrong-variant mutation. |
| (R3) S4-R3-015 (redundant `is_err()` after `matches!()`) | owner_approved_debt | **STILL PRESENT** | 2 sites. |
| (R3) S4-R3-016 (`Err(_) => return Ok(())` in `proptest_yaml_event_classification.rs:188, 225`) | owner_approved_debt | **STILL PRESENT** | 2 sites unchanged. |
| (R3) S4-R3-017 (smoke `max_stack > 0`) | owner_approved_debt | **STILL PRESENT** | Same as S4-013. |
| (R3) S4-R3-018 (smoke `!debug_str.is_empty()`) | owner_approved_debt | **STILL PRESENT** | Same as S4-014. |
| (R3) S4-R3-019 (vb_verification 0 behavior tests) | owner_approved_debt | **STILL UNRESOLVED (6 cycles)** | Same as S4-020. |
| (R3) S4-R3-020 (`is_err()` in `benches/batched_atomicity.rs:374, 377`) | owner_approved_debt | **STILL PRESENT** | 2 sites unchanged. |
| (R4) S4-R4-001 (stale orphan `crates/vb_ipc/src/queue/tests/array_queue_tests.rs`, 944 lines) | owner_approved_debt | **STILL PRESENT (unchanged)** | Same as round 4. |
| (R4) S4-R4-002 (empty stub `crates/vb_ipc/src/queue/mod.rs`, 12 lines) | owner_approved_debt | **STILL PRESENT (unchanged)** | Same as round 4. |
| (R5) S4-R5-001 (23 `assert!(false, ...)` in vb_yaml) | owner_approved_debt | **STILL PRESENT** | 18 in `source_map_tests.rs` + 5 in `proptest_yaml_profile_enforcement.rs` (unchanged). |
| (R5) S4-R5-002 (2 `is_ok()` in `vb_queue_semantics/capacity/tests.rs:74, 79`) | owner_approved_debt | **STILL PRESENT** | 2 sites unchanged. |
| (R5) S4-R5-003 (77 `assert!(false, ...)` in vb_boundary_inventory) | owner_approved_debt | **STILL PRESENT (77 sites confirmed)** | See S4-R5-003. |
| (R5) S4-R5-004 (15 `panic!()` in `vb_ipc/src/array_queue_tests.rs`) | owner_approved_debt | **STILL PRESENT** | 15 sites unchanged. |
| (R5) S4-R5-006 (S4-R3-011 partial fix in `lib_tests.rs:1305-1314`) | owner_approved_debt | **STILL PARTIALLY FIXED** | Content check at L1309-1314 still in place. |
| (R5) S4-R5-008 (stale orphan + empty stub) | owner_approved_debt | **STILL PRESENT (unchanged)** | Same as round 4. |

### New Findings Table (Round 6)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix |
|----|-----|-----------|--------|------------------------------|------------------|
| S4-R6-001 | CRITICAL | `crates/vb_ipc/src/server/handlers/tests.rs:355` (`handle_cancel_run_accepts_reason_and_routes_to_runtime`) | **NEW FAILING TEST — REAL PRODUCTION BUG.** Test added in wave-5 (commit `da55addc7` 2026-06-21 12:43:53). The test submits a workflow, calls `tick_all()`, then sends `IpcPayload::CancelRun { run_id: 3201, reason: Some("user requested abort") }` via `handle_cancel_run`. After a second `tick_all()`, the test expects `events.iter().any(\|e\| matches!(e, RuntimeJournalEvent::RunCancelled { run, reason: Some(r) } if *run == run_id && r == reason))` to be `true`. The assertion fails: `"durable RunCancelled event must carry the IPC-supplied reason"`. Production code at `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:159-188` does `self.append_journal_event(RuntimeJournalEvent::RunCancelled { run, reason })?` — but the journal contains no matching event. Either the `RunCancelled` event is being suppressed (run already terminal at cancel time), the reason field is being dropped, OR the event is being recorded with a different field shape. | Delete the test and the production bug ships. Keep the test and the production bug is caught — forcing the implementation to ensure the durable `RunCancelled` event is recorded with the IPC-supplied reason (or to fail loudly when the run is already terminal). | **Investigate `Runtime::cancel_run_with_reason` → `ShardCommand::Cancel → dispatch_cancel → handle_cancel` at `chunk_002.rs:159-188`.** The likely cause: after the first `tick_all()` at L325, the run may have advanced to the `Ask` step (suspended), but the cancel command is enqueued, then `tick_all()` at L341 processes it. `handle_cancel` at L169 does `if let Some(state) = self.run_state_remove(run)` — if the run is still in `run_state`, the event should be recorded. The bug must be in the event path or the reason propagation. **Do NOT delete the test.** |
| S4-R6-002 | CRITICAL | `crates/vb_ipc/src/server/handlers/tests.rs:409` (`handle_cancel_run_without_reason_records_no_reason_on_journal`) | **NEW FAILING TEST — REAL PRODUCTION BUG (companion to S4-R6-001).** Same test pattern but with `reason: None`. Test fails: `"durable RunCancelled event must record None when caller omits reason"`. Either the `RunCancelled` event is not being recorded at all (run already terminal) OR the reason field is not `None` for the no-reason case. | Same as S4-R6-001. | Same as S4-R6-001 — these two tests together pin the reason-propagation contract. **Do NOT delete the test.** |
| S4-R6-003 | CRITICAL | `crates/vb_benchmark/tests/batched_atomicity_tests.rs:422` (`coalescing_ratio_at_least_three`) | **NEW FAILING TEST — REAL PRODUCTION REGRESSION.** Test fails: `"non-batching shard (window=1) must not use batch appends: batch_a=100"`. The test sets up two shards — `shard_a` with `window=1` (non-batching) and `shard_b` with `window=10` (batching) — and submits 100 commands to each. After draining, `shard_a.batch_count()` is 100 (expected 0) and `shard_a.append_count()` matches `shard_b`. The non-batching shard is using batch appends when it should not, indicating the production code's coalescing window logic is broken. | This is a real performance regression: a non-batching shard using batch appends defeats the I/O reduction invariant. The mutation (delete the window check) goes undetected if the test is removed. | **Investigate the production coalescing window logic in `vb_runtime`** — likely in `shard/impl_parts/` or `shard/lifecycle/`. The fix must ensure `window=1` shards use individual appends, not batch appends. The test at L422-431 is correctly designed and catches the regression. **Do NOT delete the test.** |
| S4-R6-004 | CRITICAL | `crates/vb_verification/src/lib.rs` (entire file, 114 lines) | **STILL UNRESOLVED AFTER 6 CYCLES.** `vb_verification` has 0 behavior tests. `cargo test -p vb_verification --tests --lib` reports 0 passed. The crate contains ONLY `#[cfg(kani)] mod kani_harnesses { ... }` (3 `#[kani::proof]` fns) and `#[cfg(not(kani))] mod not_kani { }`. A regression in `hydrate_run_frame` return-value correctness is invisible to the test suite. | Mutate `hydrate_run_frame` to return `Ok(BadFrame)` for valid input. Zero behavior tests fail. Kani harnesses only verify panic-freedom. The positive-case harness `hydrate_run_frame_postcond_ok` discards the result with `let _ = ...`. | Add `crates/vb_verification/tests/hydrate_run_frame_behavior.rs` with 5 cases: (1) empty events + matching run_id → `Err(EmptyEvents)`; (2) non-matching run_id → `Err(RunIdMismatch)`; (3) RunAccepted present + matching run_id → `Ok(frame)` (assert frame.run_id == expected); (4) snapshot seq > tail seq → `Err(SeqOutOfOrder)`; (5) missing RunAccepted → `Err(MissingRunAccepted)`. Each test must assert the EXACT variant and key fields. **Effort: 1 hour. The most persistent finding in the slice (6 cycles unresolved).** |
| S4-R6-005 | LOW | `crates/vb_ipc/src/server/handlers/tests.rs:305-413` (2 new tests for `handle_cancel_run_*`) | **SILENT `Err(_) => return` PATTERN IN THE NEW FAILING TESTS.** The newly-added (and now-failing) tests at L343-347 and L398-402 use the same `Err(_) => return` silent-pass pattern that S4-011 + S4-R3-006 have flagged across 7 other sites in this file. When the test reaches the `assert!(matched, ...)` line at L355/L409, the `snap` is `Ok(events)` (no silent return); the assertion fires on the `matched` boolean. So in this case the pattern does NOT hide the failure — the `Err(_) => return` is unreachable when the assertion fires. However, if a future regression causes `journal.snapshot()` to return `Err(...)`, the test would silently `return` and be marked PASSED, hiding a different production bug. | Mutate `VolatileRuntimeJournal::snapshot` to return `Err(JournalPoisoned)`. The `Err(_) => return` arm fires, the test silently returns, and the regression is hidden. | Replace the `match snap { Ok(events) => events, Err(_) => return }` pattern with `let events = snap.expect("journal snapshot must succeed for valid run state");` at both L343-347 and L398-402. **Effort: 2 minutes, mechanical. Closes S4-011 / S4-R3-006 for the 2 new sites.** |
| S4-R6-006 | LOW | `crates/vb_ipc/src/array_queue_tests.rs:15` sites (15 `panic!()` sites) | **ROUND-5 CENSUS FINDING — STILL PRESENT.** `array_queue_tests.rs` has 15 `panic!()` sites (L44, L51, L280, L609, L703, L735, L793, L860 etc.). All are `panic!()`-equivalent forbidden patterns per AGENTS.md "No `panic!`" rule. The mutation IS caught (panic = test failure), but the forbidden macro usage persists. | Mutate `IngressFrame::new` to return `Err(PayloadTooLarge)` for valid input. The `panic!()` in `unwrap_or_else` fires — test fails. Mutation IS caught. Defect is the forbidden macro, not a mutation gap. | Replace each `panic!("...", ...)` with `.expect("...")` (allowed in tests per AGENTS.md) for the `unwrap_or_else` closures. For the bare `panic!()` in match-else arms, use fallible test signature + `?`. **Effort: 30 minutes, mechanical.** |
| S4-R6-007 | LOW | `crates/vb_ipc/src/queue/tests/array_queue_tests.rs` (944 lines, stale orphan) + `crates/vb_ipc/src/queue/mod.rs` (12 lines, empty stub) | **ROUND-4+5 CARRYOVER.** Stale orphan duplicate and empty stub unchanged from round 4. | N/A — orphan/stub, not a mutation gap. | `git rm crates/vb_ipc/src/queue/tests/array_queue_tests.rs` and delete the empty `queue/` directory + `pub mod queue;` at `lib.rs:29`. **Effort: 5 minutes.** |

### Pattern Census (Round 6 counts)

| Crate | `assert_ok!` macro | `is_ok()` direct | `is_err()` direct | `unwrap_or_default()` | `assert!(false, …)` / `prop_assert!(false, …)` | `panic!()` | `unreachable!()` | `fail_assert!` | `Err(_) => return` | `let _ = ` (problematic) |
|-------|---------------------|-------------------|---------------------|------------------------|----------------------|------------|-------------------|-----------------|---------------------|----------------------------|
| vb_expr | 0 | ~30 (parser/lexer miri_tests: 3 + 6; eval_tests.rs:3421-3425: 5; proptest_type_enforcer.rs: 16) | ~15 (proptest_type_enforcer.rs: 16) | 0 | 0 | 10 (eval_tests.rs) | 0 | 0 | 0 | 2 (eval/property_tests bap, "no-panic" docstring tests) |
| vb_ipc | 125 (tests.rs:89 + frame/tests.rs:36) | 5 (handlers/tests.rs:182, 248, 458) | 0 in `is_err()` form; 2 in `if drain.is_err()` benches | 0 | 0 | 15 (array_queue_tests.rs) + 0 elsewhere | 0 | 0 | 7 (handlers/tests.rs:189, 255, 334, 346, 389, 401, 465) | 0 |
| vb_yaml | 0 | 1 (lib_tests.rs:1306) | 0 | 16 (source_map_tests.rs) | 23 (source_map_tests.rs:18 + proptest_yaml_profile_enforcement.rs:5) | 0 | 0 | 72 (6 files; round-6 corrected from round-5's 90) | 2 (proptest_yaml_event_classification.rs:188, 225) | 0 (only 2 acceptable sites in property_tests) |
| vb_queue_semantics | 0 | 2 (capacity/tests.rs:74, 79) | 0 | 0 | 0 | 0 | 45 (transitions/tests.rs) | 0 | 0 | 0 |
| vb_boundary_inventory | 0 | 8 (api_tests.rs:25-27 + L942 + vb_god2f_validation_properties.rs:94/97/166/175/232) | 2 (error_tests.rs:158, 169) | 0 | 77 (api_tests.rs:43 + validation_tests.rs:13 + parser_tests.rs:9 + property_tests.rs:9 + comments) | 0 | 0 | 0 | 0 | 0 |
| vb_benchmark | 0 | 0 | 2 (benches/batched_atomicity.rs:374, 377) | 0 | 0 | 48 (benchmark_tests.rs:32 + batched_atomicity_tests.rs:16) | 0 | 0 | 0 | 8 (5 tests + 3 benches) |
| vb_test_util | 0 | 0 | 0 | 0 | 0 | 12 (density_tests.rs) | 0 | 0 | 0 | 0 |
| vb_doc | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| vb_ajc40_flux | 0 | 0 | 0 | 0 | 6 (density_tests.rs:202/211/220/237/246/255) | 0 | 0 | 0 | 0 | 0 |
| vb_verification | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| **Slice 4 Total** | **125** | **~46** | **~19** | **16** | **106** | **85** | **45** | **72** | **9** | **10** |

Notes:
- The 3 NEW CRITICAL findings (S4-R6-001, S4-R6-002, S4-R6-003) are **production-bug-exposing tests**, NOT test-quality defects. The tests are correctly designed; the production code has bugs.
- `fail_assert!` count corrected from 90 (round 5) to 72 (round 6). Likely wave-5 edits removed 18 `fail_assert!` invocations.
- `vb_verification` still has ZERO behavior tests after 6 cycles.
- `vb_ipc` has no new `crossbeam_channel` uses. The 6 `crossbeam` references in `vb_ipc` are: 1 production `use crossbeam_queue::ArrayQueue;` at `ingress.rs:8` (Section 50 compliant) + 5 doc-comment references (not executable).
- `let _ = ` audit confirms only 2 acceptable sites in `vb_yaml` (`events_tests.rs:583` and `proptest_yaml_event_classification.rs:255` — both documented as "no-panic" tests) + 8 problematic in `vb_benchmark` (5 tests + 3 benches) + 2 in `vb_ipc` production (`frame_types.rs:140`, `kani_flag_validation.rs:883` — not test surface) + 2 in `vb_expr` kani harnesses (acceptable). No new `let _ = ` silent-error patterns introduced.

### Mutation Gaps (top 5 most dangerous mutations that would NOT be caught)

1. **`hydrate_run_frame` returns `Ok(BadFrame)` for valid input** (production in `vb_storage`, called from `crates/vb_verification/src/lib.rs`). **STILL THE HIGHEST-SEVERITY MUTATION GAP** for the 6th consecutive cycle. `vb_verification` has 0 behavior tests. Kani harnesses only verify panic-freedom. The positive-case harness `hydrate_run_frame_postcond_ok` discards the result with `let _ = hydrate_run_frame(...)`. `cargo test -p vb_verification --lib` reports 0 passed. (See S4-R6-004.)

2. **`Runtime::cancel_run_with_reason` fails to record the `RunCancelled` event with the IPC-supplied reason** (production in `vb_runtime`, called from `crates/vb_ipc/src/server/handlers/runs.rs:125`). **NEW MUTATION GAP surfaced by round 6.** The two newly-failing tests (S4-R6-001, S4-R6-002) are correctly designed and would catch this bug — but if the test surface were smaller (e.g., if the tests were removed because they're failing), the bug would ship. The two tests at L355 and L409 are the only thing catching this regression.

3. **`Runtime` coalescing window logic is broken — `window=1` shards use batch appends** (production in `vb_runtime`, called from `crates/vb_benchmark/tests/batched_atomicity_tests.rs:422`). **NEW MUTATION GAP surfaced by round 6.** The test at L422-431 (`assert_eq!(batch_a, 0, "non-batching shard (window=1) must not use batch appends: batch_a={batch_a}")`) is the only thing catching this performance regression. If the test were removed or weakened, a regression that defeats the I/O reduction invariant would ship silently.

4. **`build_semantic_source_map` returns `Err(YamlError::EmptySource)` for valid YAML** (production `crates/vb_yaml/src/source_map_build.rs`). 16 tests in `vb_yaml/src/source_map_tests.rs` use `unwrap_or_default()` (S4-R3-003), silently swallowing the error. The 72 `fail_assert!` invocations in 6 vb_yaml files (S4-R3-004) would also become no-ops if `assertion_failed` were mutated to return `true`. Plus the 23 `assert!(false, ...)` sites in vb_yaml (S4-R5-001) silently pass on fixture failures.

5. **`VolatileRuntimeJournal::snapshot` returns `Err(JournalPoisoned)` for valid run state** (production in `vb_runtime`, called from `crates/vb_ipc/src/server/handlers/tests.rs:180, 246, 456` + 2 new sites at L343, L398). The multi-line pattern `assert!(snapshot.is_ok(), ...); match snapshot { Ok(events) => events, Err(_) => return };` at 3+2=5 sites silently returns PASSED when snapshot fails. The 2 new sites (L343, L398) inherit the S4-R3-006 defect.

### Top 5 Fixes (ranked by impact-per-effort)

1. **Fix the production bug in `Runtime::cancel_run_with_reason` → `handle_cancel`** at `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:159-188`. The two failing tests (S4-R6-001, S4-R6-002) pin the reason-propagation contract. Likely cause: the run is being moved to terminal state by the first `tick_all()` BEFORE the cancel command is processed, so `handle_cancel` at L169 finds the run not in `run_state` and skips the journal append. Verify by reading the test workflow (`ask_then_finish_workflow` — SetConst → Ask → AskResume → Finish) and the runtime lifecycle; the first `tick_all()` at L325 should suspend on Ask, then the cancel should record the event. If the run is being marked terminal prematurely, the fix is in the tick logic; if the cancel event is being suppressed because the run is not in `run_state`, the fix is to check `terminal_runs_contains` and either skip or log. **Effort: 1-2 hours. Closes S4-R6-001 + S4-R6-002 CRITICAL.** Do NOT weaken or delete the tests.

2. **Fix the production regression in the coalescing window logic** at `crates/vb_runtime/src/shard/impl_parts/dispatch.rs` or `crates/vb_runtime/src/shard/lifecycle/`. The test at `batched_atomicity_tests.rs:422` expects `window=1` shards to have `batch_count() == 0`, but the actual value is 100. The window check is being bypassed. **Effort: 30 minutes. Closes S4-R6-003 CRITICAL.** Do NOT weaken or delete the test.

3. **Add `crates/vb_verification/tests/hydrate_run_frame_behavior.rs`** with 5 cases: (a) empty events + matching run_id → `Err(EmptyEvents)`; (b) non-matching run_id in snapshot → `Err(RunIdMismatch)`; (c) RunAccepted event present + matching run_id → `Ok(frame)` (assert frame.run_id == expected); (d) snapshot seq > tail seq → `Err(SeqOutOfOrder)`; (e) missing RunAccepted → `Err(MissingRunAccepted)`. Each test must assert the EXACT variant and key fields. **Effort: 1 hour. Closes the 6-cycle-old S4-R3-019 (was S4-020). The only CRITICAL-level coverage gap remaining in the slice.**

4. **Replace `assert!(snapshot.is_ok(), ...); let events = match snap { Ok(events) => events, Err(_) => return };` pattern with `let events = snap.expect("journal snapshot must succeed for valid run state");`** at the 2 new sites `vb_ipc/src/server/handlers/tests.rs:343-347, 398-402` (S4-R6-005). Same for the 7 carryover sites (S4-R3-006). **Effort: 10 minutes, mechanical.**

5. **Replace 6 `assert!(false, "...")` panic-equivalent sites in `vb_ajc40_flux/tests/density_tests.rs`** (L202, L211, L220, L237, L246, L255) with `expect(...)` and fallible test signatures. **Effort: 15 minutes, mechanical. Completes the round-1 fix vb-2kw49. Closes S4-R3-002.**

### Verdict Line

STATUS: REJECTED

### Disposition

| Finding ID | Disposition |
|-----------|-------------|
| S4-R6-001 | blocker (CRITICAL — NEW failing test in wave-5; test is correctly designed and catches a real production bug in `Runtime::cancel_run_with_reason` → `handle_cancel` reason propagation; fix the production code) |
| S4-R6-002 | blocker (CRITICAL — NEW failing test in wave-5; companion to S4-R6-001; same production bug) |
| S4-R6-003 | blocker (CRITICAL — NEW failing test in wave-5; test catches a real production regression in coalescing window logic — `window=1` shards using batch appends; fix the production code) |
| S4-R6-004 | blocker (CRITICAL — `vb_verification` still has 0 behavior tests after **6 cycles**; the most persistent finding in the slice) |
| S4-R6-005 | owner_approved_debt (2 NEW sites in `handlers/tests.rs:343-347, 398-402` inherit the S4-R3-006 silent-pass pattern; replace with `.expect(...)`) |
| S4-R6-006 | owner_approved_debt (round-5 census finding: 15 `panic!()` sites in `array_queue_tests.rs` still present; forbidden macro usage but mutation IS caught) |
| S4-R6-007 | owner_approved_debt (round-4+5 carryover: stale orphan + empty stub unchanged) |
| (Round 1) vb-0to5y | fixed_with_evidence (8 short-circuit tests in `eval_tests.rs:680-753`; orphan file removed) |
| (Round 1) vb-2kw49 | partial (imports correct, 6 `assert!(false, …)` sites remain) |
| (Round 1) vb-8r7cp | fixed_with_evidence (`MemoryIngress` at `tests.rs:445-451`; no `crossbeam_channel` in code) |
| (Round 1) vb-few2x | fixed_with_evidence (FIFO run_id order check applied at `array_queue_tests.rs:756-760`; 33 tests run) |
| (Round 1) S4-005 | NOT FIXED (125 `assert_ok!` macro invocations in vb_ipc) |
| (Round 1) S4-006 / (R3) S4-R3-011 | PARTIALLY FIXED in round 5 (content check at L1309-1314) |
| (Round 1) S4-007 | NOT FIXED (5 `is_ok()` in vb_boundary_inventory) |
| (Round 1) S4-008 | NOT FIXED (`assert_io_ok` helper) |
| (Round 1) S4-009 | STILL PRESENT (16 `is_err()` in proptest_type_enforcer) |
| (Round 1) S4-011 | NOT FIXED (7 `Err(_) => return` in handlers/tests.rs; + 2 new sites S4-R6-005) |
| (Round 1) S4-012 | NOT FIXED (`ast.is_err()` in miri_tests) |
| (Round 1) S4-013 | NOT FIXED (smoke `max_stack > 0`) |
| (Round 1) S4-014 | NOT FIXED (smoke `!debug_str.is_empty()`) |
| (Round 1) S4-015 | NOT FIXED (77 `assert!(false, …)`) |
| (Round 1) S4-016 | NOT FIXED (72 `fail_assert!`; corrected from round-5's 90) |
| (Round 1) S4-020 | NOT FIXED (`vb_verification` 0 behavior tests; S4-R6-004 — **6 cycles unresolved**) |
| (Round 2) S4-R2-001 | fixed_with_evidence (file is no longer dead code; 33 tests run) |
| (Round 2) S4-R2-002 | fixed_with_evidence (queue_boundary.rs wired; 19 boundary tests run) |
| (Round 2) S4-R2-003 | fixed_with_evidence (12 `bap_*` tests pass; production bug fixed) |
| (Round 2) S4-R2-007 | fixed_with_evidence (round-5 misclassification correction: variant check IS present at `tests.rs:2019-2022`) |
| (Round 2) S4-R2-004 | NOT FIXED (6 `assert!(false, …)`; S4-R3-002) |
| (Round 2) S4-R2-005 | NOT FIXED (16 `unwrap_or_default()`) |
| (Round 2) S4-R2-006 | NOT FIXED (72 `fail_assert!`) |
| (Round 2) S4-R2-008 | NOT FIXED (7 `Err(_) => return`) |
| (Round 2) S4-R2-009 | NOT FIXED (8 `let _ =` and `Ok(()) => {}` in vb_benchmark) |
| (Round 2) S4-R2-010 | NOT FIXED (77 `assert!(false, …)`) |
| (Round 2) S4-R2-011 | NOT FIXED (redundant `is_err()`) |
| (Round 2) S4-R2-012 | NOT FIXED (72 `panic!()` across vb_benchmark + vb_test_util + array_queue_tests) |
| (Round 2) S4-R2-014 | NOT FIXED (`vb_verification` 0 behavior tests; S4-R6-004) |
| (Round 2) S4-R2-015 | NOT FIXED (`ast.is_err()`) |
| (Round 2) S4-R2-016 | NOT FIXED (45 `unreachable!()`) |
| (Round 2) S4-R2-017 | NOT FIXED (`Err(_) => return Ok(())`) |
| (Round 2) S4-R2-018 | NOT FIXED (`is_err()` in benches) |
| (Round 2) S4-R2-019 | NOT FIXED (smoke `!debug_str.is_empty()`) |
| (Round 2) S4-R2-020 | NOT FIXED (smoke `max_stack > 0`) |
| (Round 3) S4-R3-001 | fixed_with_evidence (CRITICAL closed: 33 `array_queue_tests::*` tests run; FIFO order check functional) |
| (Round 3) S4-R3-014 | fixed_with_evidence (type error resolved: `while let Ok(Some(_frame))`) |
| (Round 4) S4-R4-001 | owner_approved_debt (stale orphan at `src/queue/tests/array_queue_tests.rs`) |
| (Round 4) S4-R4-002 | owner_approved_debt (empty stub `queue/mod.rs`) |
| (Round 5) S4-R5-001 | owner_approved_debt (23 `assert!(false, ...)` in vb_yaml) |
| (Round 5) S4-R5-002 | owner_approved_debt (2 `is_ok()` in vb_queue_semantics) |
| (Round 5) S4-R5-003 | owner_approved_debt (77 `assert!(false, ...)` in vb_boundary_inventory) |
| (Round 5) S4-R5-004 | owner_approved_debt (15 `panic!()` in array_queue_tests.rs; S4-R6-006) |
| (Round 5) S4-R5-005 | fixed_with_evidence (S4-R2-007 resolved: variant check IS present) |
| (Round 5) S4-R5-006 | owner_approved_debt (S4-R3-011 partial fix) |
| (Round 5) S4-R5-007 | owner_approved_debt → S4-R6-004 (vb_verification 0 behavior tests; **6 cycles**) |
| (Round 5) S4-R5-008 | owner_approved_debt (S4-R4-001 + S4-R4-002 carryover; S4-R6-007) |
