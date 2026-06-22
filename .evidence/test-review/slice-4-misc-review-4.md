# Test Review — Slice 4 Round 4 (Misc Crates: vb_expr, vb_ipc, vb_yaml, vb_queue_semantics, vb_boundary_inventory, vb_benchmark, vb_test_util, vb_doc, vb_ajc40_flux, vb_verification)

## STATUS: REJECTED

Round 4 confirms the round-3 CRITICAL fix (S4-R3-001) is **STILL APPLIED**: `crates/vb_ipc/src/lib.rs:98` now contains `#[cfg(test)] mod array_queue_tests;` and the `pub mod queue;` declaration at `lib.rs:29` references the doc-only `queue/mod.rs`. The test binary `target/debug/deps/vb_ipc-160bf0e4d3a7740d` now contains 33 `array_queue_tests::*` entries (was 0 in round 3); `target/debug/deps/vb_ipc-160bf0e4d3a7740d array_queue_tests::` runs 33/33 passing. The lib test count rose from 621 to 654 (+33, exactly the `array_queue_tests` count). The round-1 FIFO fix (vb-few2x) is now functional: `fifo_order_invariant_for_submit_recv_cycle` at `array_queue_tests.rs:730-761` asserts both `submitted.len() == received.len()` AND `received.iter().map(|f| f.run_id().as_u64()) == submitted.iter().map(...)` — the exact order check the round-1 reviewer demanded. **Zero round-1+2+3 regressions detected**; all 6 prior fix beads and all 22 round-3 findings remain in their expected disposition. However, a **NEW LOW finding** surfaces: a stale 944-line duplicate `array_queue_tests.rs` lives at `crates/vb_ipc/src/queue/tests/array_queue_tests.rs` (orphan, not compiled; differs from the active `src/array_queue_tests.rs` by the `arb_capacity` refactor and 32-line whitespace). The slice remains REJECTED on the same 13+ MEDIUM/LOW carryover items from rounds 1–3 that have not been closed.

### Round 1+2+3 Fix Verification Table

| Round Bead | Finding | File:Line (Expected) | Round-4 Status | Evidence |
|------------|---------|----------------------|----------------|----------|
| vb-0to5y | Round 1 S4-001/S4-021 (Section 46 short-circuit tests) | `crates/vb_expr/src/eval_tests.rs:669-757` (8 tests) | **STILL APPLIED** | `eval_tests.rs:680, 693, 706, 719, 732, 739, 746, 753` — 8 short-circuit tests `eval_binary_op_{and,or}_{evaluates_right_even_when_left_is_{false,true},rejects_two_non_boolean_operands,accepts_two_boolean_operands,returns_{false,true}_when_left_{true,false}_right_{false,true}}`. Each uses `let Err(ExprError::TypeMismatch { expected, found }) = result else { return Err(...) };` to force the right-operand `TypeMismatch` to be observed regardless of left. Orphan file `crates/vb_expr/src/eval/tests/and_or_short_circuit_tests.rs` does not exist. |
| vb-2kw49 | Round 1 S4-002 (vb_ajc40_flux local validator copies) | Imports from `vb_core::workflow::compiled_slug` | **STILL APPLIED** | `vb_ajc40_flux/tests/density_tests.rs:24-27` imports `validate_compiled_slug_count`, `validate_compiled_slug_summary` from `vb_core::workflow::compiled_slug`. Local copies deleted. Header at L17-21 explicitly documents the S4-002 fix. 52 tests pass under `--features positive`. |
| vb-8r7cp | Round 1 S4-003/S4-018 (forbidden `crossbeam_channel` in `vb_ipc/src/tests.rs:443-455`) | `MemoryIngress::bounded(...)` + `disconnect_sender()` + `assert_eq!(try_recv(), Err(IpcError::Disconnected))` | **STILL APPLIED** | `crates/vb_ipc/src/tests.rs:447-449` reads: `let mut ingress = MemoryIngress::bounded(QueueCapacity::new(std::num::NonZeroUsize::MIN)); ingress.disconnect_sender(); assert_eq!(ingress.try_recv(), Err(IpcError::Disconnected));`. No `crossbeam_channel` import in code. The only `crossbeam` references in vb_ipc are in DOC COMMENTS only: `array_queue_tests.rs:2`, `ingress.rs:80`, `queue/mod.rs:1-6`. The `crossbeam_queue::ArrayQueue` import at `ingress.rs:8` is the production type (Section 50 compliant). |
| vb-few2x | Round 1 S4-004 (FIFO order assertion) + Round 2 S4-R2-001 (queue/tests/array_queue_tests.rs wired into Cargo) + Round 3 S4-R3-001 (mod queue; in lib.rs) | `while let Ok(Some(frame))` + `prop_assert_eq!(received run_id order, submitted order)` + file compiled by Cargo | **STILL APPLIED (FINALLY FUNCTIONAL)** | `crates/vb_ipc/src/array_queue_tests.rs:730-761` (active file at `src/array_queue_tests.rs`, wired by `crates/vb_ipc/src/lib.rs:98` `mod array_queue_tests;`): `while let Ok(Some(frame)) = ingress.try_recv() { received.push(frame); }` and `prop_assert_eq!(received.iter().map(|f| f.run_id().as_u64()).collect::<Vec<_>>(), submitted.iter().map(|i| u64::try_from(*i).unwrap()).collect::<Vec<_>>(), "received run_ids must match submitted run_ids in submission order (FIFO)")`. `target/debug/deps/vb_ipc-160bf0e4d3a7740d --list` shows 33 `array_queue_tests::*` entries (incl. `fifo_order_invariant_for_submit_recv_cycle`, `memory_ingress_try_recv_returns_items_in_fifo_order`). `array_queue_tests::` filter runs 33/33 passing. **The 3-round-old defect is now fully closed.** |
| S4-R2-002 | Round 2 S4-R2-002 (queue_boundary.rs wired into lib.rs) | `#[cfg(test)] mod tests;` in `vb_queue_semantics/src/lib.rs`; `mod queue_boundary;` in `src/tests/mod.rs` | **STILL APPLIED** | `crates/vb_queue_semantics/src/lib.rs:21` has `#[cfg(test)] mod tests;`. `src/tests/mod.rs:8` has `mod queue_boundary;`. Test binary contains 19 `tests::queue_boundary::*` entries (verified by `--list`). 5 lib tests pass for the `rpo_queue_003_*` family (boundary-tier proptest regression suite at `lib.rs:21-22` of the integration test file). |
| S4-R2-003 | Round 2 S4-R2-003 (production bug in `eval_expr_program` for boolean literals) | All 12 `proptest_bytecode_ast_parity` tests pass | **STILL APPLIED** | `target/debug/deps/vb_expr-ec3cc0ab4a18a082 proptest_bytecode_ast_parity` reports **12 passed, 0 failed, 873 filtered out**. The previous `fold=Some(Bool(false)) eval=Err(UnexpectedEof)` failure is gone. Production fix is in place. |

**Round 1+2+3 regression count: 0.** (S4-R3-001 is now closed; no new regressions introduced.)

### Round 1–3 Findings Status (carryover)

| Round ID | Disposition | Round-4 Status | Evidence |
|----------|-------------|----------------|----------|
| (R1) S4-005 (90+ `assert_ok!` macro uses in `vb_ipc/src/tests.rs`) | owner_approved_debt | **STILL PRESENT** | `tests.rs:89` `assert_ok!` invocations + `frame/tests.rs:36` = 125 total in vb_ipc. Per-call-site audit still not done; many sites have content checks, but L1402-L1828 bare-as-sole-assertion sites remain. Plus `tests.rs:2011` `prop_assert_err!(result)` still has no variant check. |
| (R1) S4-006 (`vb_yaml/src/lib_tests.rs:1306` smoke `is_ok()`) | owner_approved_debt | **STILL PRESENT** | `lib_tests.rs:1304-1308`: `assert!(result.is_ok(), "load_fixture_source should accept valid workflow, got {result:?}")`. No content verification. |
| (R1) S4-007 (5 `is_ok()` in `vb_boundary_inventory/.../vb_god2f_validation_properties.rs`) | owner_approved_debt | **STILL PRESENT** | `vb_god2f_validation_properties.rs:94, 97, 166, 175, 232` — 5 `prop_assert!(...is_ok())` / `assert!(...is_ok())` checks with no variant field verification. |
| (R1) S4-008 (`assert_io_ok` helper in `vb_boundary_inventory/src/tests/api_tests.rs:25-27`) | owner_approved_debt | **STILL PRESENT** | `api_tests.rs:25-27`: `fn assert_io_ok(result, context) { assert!(result.is_ok(), "{context}: {result:?}"); }`. Used 14+ times (L73, 77, 96, 100, 120, 124, 148, 152, 173, 177, 195, 231, 235, 938) plus `assert!(dir.is_ok(), "tempdir succeeds: {dir:?}")` at L942. |
| (R1) S4-009 (16 `is_err()` in `vb_expr/tests/proptest_type_enforcer.rs`) | owner_approved_debt | **STILL PRESENT** | `proptest_type_enforcer.rs:372-389`: 16 `prop_assert!(...is_err())` calls — partition invariant at L347-351 IS a stronger check, but the per-enforcer `is_err()` doesn't verify the exact variant. |
| (R1) S4-011 (`vb_ipc/src/server/handlers/tests.rs` `Err(_) => return` pattern) | owner_approved_debt | **STILL PRESENT** | 3 sites at L180-190, L246-256, L456-465 with the exact pattern `assert!(snapshot.is_ok(), ...); let events = match snapshot { Ok(events) => events, Err(_) => return };`. The multi-line pattern means a flat `rg` for `Err(_) => return` returns 0 matches (lines wrap), but the pattern is unchanged from rounds 1–3. 0 `assert!(result.is_err())` redundant sites in `error_tests.rs:158, 169` also unchanged. |
| (R1) S4-012 (`ast.is_err()` / `tokens.is_err()` in `parser/miri_tests.rs`, `lexer/miri_tests.rs`) | owner_approved_debt | **STILL PRESENT** | `parser/miri_tests.rs:130, 162, 180, 205, 221` (5 `ast.is_err()`); `lexer/miri_tests.rs:134, 149, 181` (3 `tokens.is_err()`). Mutation: `parse_expr(&[])` returns `Err(ExpressionTooLong)` instead of `Err(UnexpectedToken)` → all tests pass. |
| (R1) S4-013 (`vb_expr/src/bytecode/tests.rs:416-426` smoke `max_stack > 0`) | owner_approved_debt | **STILL PRESENT** | `bytecode/tests.rs:424`: `assert!(max_stack > 0, "max_stack should be positive")`. Mutation: `check_expr_stack_bound` returns `999` → test passes. |
| (R1) S4-014 (`vb_boundary_inventory/src/tests/error_tests.rs:98-118` smoke `!debug_str.is_empty()`) | owner_approved_debt | **STILL PRESENT** | `error_tests.rs:116`: `assert!(!debug_str.is_empty())`. Derived `Debug` is guaranteed non-empty. |
| (R1) S4-015 (18 `assert!(false, …)` in vb_boundary_inventory) | owner_approved_debt | **STILL PRESENT** | `validation_tests.rs:35, 55, 75, 82, 91, 98, 173, 180, 202, 232, 240, 293, 312` (13 sites) + `parser_tests.rs:22, 43, 48, 71, 104, 325, 583, 599, 604` (9 sites) + `property_tests.rs:105, 119, 126, 210, 217, 225, 233` (7 `prop_assert!(false)` sites). Total 29 forbidden `assert!(false)` / `prop_assert!(false)` sites. |
| (R1) S4-016 (`fail_assert!` macro in 6 vb_yaml files) | owner_approved_debt | **STILL PRESENT** | 6 files: `lib_tests.rs:17`, `source_map_tests.rs:10`, `profile_tests.rs:11`, `profile_tests_adversarial.rs:6`, `profile_error_variants_tests.rs:4`, `events_tests.rs:24` = **72 invocations total** (unchanged from round 2). |
| (R1) S4-020 (`vb_verification` has 0 behavior tests) | owner_approved_debt | **STILL UNRESOLVED** | `crates/vb_verification/src/lib.rs` is still 114 lines: only `#[cfg(kani)] mod kani_harnesses { ... }` (3 `#[kani::proof]` fns) + `#[cfg(not(kani))] mod not_kani { }`. **No `tests/` directory exists. `cargo test -p vb_verification --lib` reports 0 passed.** Round 4 made no progress. |
| (R2) S4-R2-003 | fixed_with_evidence | **STILL APPLIED** | 12 `proptest_bytecode_ast_parity` tests pass; production bug fixed. |
| (R2) S4-R2-002 | fixed_with_evidence | **STILL APPLIED** | 19 `queue_boundary::*` tests in test binary. |
| (R3) S4-R3-001 | blocker → **fixed_with_evidence** | **STILL APPLIED (NOW FULLY FUNCTIONAL)** | 33 `array_queue_tests::*` tests in test binary; `fifo_order_invariant_for_submit_recv_cycle` runs. The 3-round regression is closed. |
| (R3) S4-R3-002 (6 `assert!(false, …)` in `vb_ajc40_flux`) | owner_approved_debt | **STILL PRESENT** | `vb_ajc40_flux/tests/density_tests.rs:202, 211, 220, 237, 246, 255` — 6 forbidden sites unchanged. |
| (R3) S4-R3-003 (16 `unwrap_or_default()` in `vb_yaml/src/source_map_tests.rs`) | owner_approved_debt | **STILL PRESENT** | 16 `unwrap_or_default()` calls at L240, 257, 342, 360, 377, 394, 411, 431, 463, 483, 497, 514, 564, 580, 641, 665. |
| (R3) S4-R3-004 (72 `fail_assert!` in 6 vb_yaml files) | owner_approved_debt | **STILL PRESENT** | Same as (R1) S4-016: 72 invocations across 6 files. |
| (R3) S4-R3-005 (125 `assert_ok!` in vb_ipc) | owner_approved_debt | **STILL PRESENT** | 89 in `tests.rs` + 36 in `frame/tests.rs` = 125. |
| (R3) S4-R3-006 (7 `Err(_) => return` in `handlers/tests.rs`) | owner_approved_debt | **STILL PRESENT** | 3 multi-line `assert!(snapshot.is_ok(), ...); match snapshot { Ok(events) => events, Err(_) => return };` patterns at L180, L246, L456. |
| (R3) S4-R3-007 (24 `assert!(false, …)` + 8 `is_ok()` in vb_boundary_inventory) | owner_approved_debt | **STILL PRESENT** | 18 `assert!(false, …)` + 8 `is_ok()` (5 in `vb_god2f_validation_properties.rs` + 1 in `api_tests.rs` `assert_io_ok` helper + 1 in `error_tests.rs:158` + 1 in `error_tests.rs:169`) = 29+8 = 37 weak-assertion sites. |
| (R3) S4-R3-008 (30+ `unreachable!()` in `vb_queue_semantics/src/transitions/tests.rs`) | owner_approved_debt | **STILL PRESENT** | 54 `unreachable!()` occurrences (count rose from "30+" in round 3 to 54 in round 4 — same sites but more). |
| (R3) S4-R3-009 (5 `let _ =` + 8 `Ok(()) => {}` + 2 `is_err()` + 48 `panic!()` in vb_benchmark) | owner_approved_debt | **STILL PRESENT** | 8 `let _ = shard` (5 in tests + 3 in benches); 8 `Ok(()) => {}` (4 in `benchmark_tests.rs` + 4 in `batched_atomicity_tests.rs`); 2 `is_err()` in `benches/batched_atomicity.rs:374, 377`; 48 `panic!()` (32 in `benchmark_tests.rs` + 16 in `batched_atomicity_tests.rs`). |
| (R3) S4-R3-010 (12 `panic!()` in `vb_test_util/tests/density_tests.rs`) | owner_approved_debt | **STILL PRESENT** | 12 `panic!()` sites at L110, 142, 154, 165, 181, 185, 197, 201, 213, 224, 233, 297. |
| (R3) S4-R3-011 (`vb_yaml/src/lib_tests.rs:1306` smoke `is_ok()`) | owner_approved_debt | **STILL PRESENT** | Same as (R1) S4-006. |
| (R3) S4-R3-012 (`ast.is_err()` / `tokens.is_err()` in vb_expr miri_tests) | owner_approved_debt | **STILL PRESENT** | Same as (R1) S4-012: 5 + 3 = 8 sites. |
| (R3) S4-R3-013 (5 `prop_assert!().is_err()` in `eval_tests.rs:3421-3425`) | owner_approved_no_action | **STILL PRESENT (acceptable per round-3 disposition)** | 5 `prop_assert!(expect_*(v).is_err())` calls. Partition invariant at L3444 catches wrong-variant mutation. |
| (R3) S4-R3-014 (type error at `array_queue_tests.rs:774`) | blocker → **fixed_with_evidence** | **STILL APPLIED (FIXED in active file; the OLD file also has the fix)** | The active `src/array_queue_tests.rs:784` reads `while let Ok(Some(_frame)) = ingress.try_recv() { /* drain */ }` (correct). The orphan at `src/queue/tests/array_queue_tests.rs:784` also has the fix (`while let Ok(Some(_frame))`). The S4-R3-014 type error is no longer present in either file. |
| (R3) S4-R3-015 (redundant `is_err()` after `matches!()` in `error_tests.rs:158, 169`) | owner_approved_debt | **STILL PRESENT** | 2 redundant `assert!(result.is_err())` lines. |
| (R3) S4-R3-016 (`Err(_) => return Ok(())` in `vb_yaml/proptest_yaml_event_classification.rs:188, 225`) | owner_approved_debt | **STILL PRESENT** | 2 `Err(_) => return Ok(())` sites. The line numbers L188 and L225 are confirmed in the file. |
| (R3) S4-R3-017 (smoke `max_stack > 0` in `vb_expr/bytecode/tests.rs:416-426`) | owner_approved_debt | **STILL PRESENT** | Same as (R1) S4-013. |
| (R3) S4-R3-018 (smoke `!debug_str.is_empty()` in `error_tests.rs:98-118`) | owner_approved_debt | **STILL PRESENT** | Same as (R1) S4-014. |
| (R3) S4-R3-019 (vb_verification 0 behavior tests) | owner_approved_debt | **STILL UNRESOLVED** | Same as (R1) S4-020. **Round 4 added zero behavior tests.** |
| (R3) S4-R3-020 (`is_err()` in `vb_benchmark/benches/batched_atomicity.rs:374, 377`) | owner_approved_debt | **STILL PRESENT** | 2 banned `is_err()` sites. |
| (R3) S4-R3-021 (`thread_local!` in `vb_ipc/src/server/helpers/mod.rs:147`) | owner_approved_no_action | **STILL PRESENT (acceptable)** | Test-helper `thread_local!`; local to test process. |
| (R3) S4-R3-022 (`let _ = eval_expr_program(...)` in `bap_reference_does_not_fold`, `bap_helper_does_not_fold`) | owner_approved_no_action | **STILL PRESENT (acceptable)** | 2 `let _ = eval_expr_program(...)` sites in `proptest_bytecode_ast_parity.rs:342, 417`; documented "no-panic" test per docstring. |

### New Findings Table (Round 4)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix |
|----|-----|-----------|--------|------------------------------|------------------|
| S4-R4-001 | LOW | `crates/vb_ipc/src/queue/tests/array_queue_tests.rs` (entire file, 944 lines) | **STALE ORPHAN DUPLICATE.** The active `array_queue_tests.rs` lives at `crates/vb_ipc/src/array_queue_tests.rs` (wired in via `crates/vb_ipc/src/lib.rs:98` `mod array_queue_tests;`). The 944-line `crates/vb_ipc/src/queue/tests/array_queue_tests.rs` is the old copy and is now ORPHANED: `crates/vb_ipc/src/queue/mod.rs` is a 12-line doc-comment stub (no `mod tests;` declaration) referenced by `pub mod queue;` at `lib.rs:29`. The orphan file differs from the active file in 33 lines (notably: the `arb_capacity` strategy was refactored from `any::<NonZeroUsize>().prop_filter(...).prop_map(QueueCapacity::new)` to `(1usize..=1024).prop_map(...)`, plus whitespace in `arb_ingress_frame`). The orphan is **not compiled** (no path attribute or `mod` declaration references it), but it occupies 944 lines of the repository. | N/A — orphan, not a mutation gap. Risk is repository hygiene / future confusion: a future maintainer might edit the orphan and assume it's the active test file. | `git rm crates/vb_ipc/src/queue/tests/array_queue_tests.rs` (or move the active file to `src/queue/tests/array_queue_tests.rs` and wire it via `pub mod queue;` properly). The 12-line doc-only `queue/mod.rs` should either (a) declare `mod tests;` to properly house the file, or (b) be deleted entirely along with `pub mod queue;` at `lib.rs:29` (since `queue/` contains no production code). **Effort: 5 minutes, mechanical.** |
| S4-R4-002 | LOW | `crates/vb_ipc/src/queue/mod.rs` (entire file, 12 lines) | **EMPTY STUB MODULE.** The `pub mod queue;` declaration at `lib.rs:29` references a `queue/mod.rs` that contains ONLY doc comments — no `pub mod tests;`, no `mod array_queue_tests;`, no production code. The comment at `queue/mod.rs:11-12` describes a wiring plan (`#[path = "queue/tests/array_queue_tests.rs"] mod array_queue_tests;`) that was never implemented (the actual wiring happens at `lib.rs:98`, not here). | N/A — empty stub, not a mutation gap. | Either (a) delete `pub mod queue;` at `lib.rs:29` and the empty `queue/` directory entirely, or (b) move the active `src/array_queue_tests.rs` into `src/queue/tests/array_queue_tests.rs` and add the proper `mod tests;` declaration in `queue/mod.rs` so the comment matches reality. **Effort: 5 minutes.** |

### Pattern Census (Round 4 counts)

| Crate | `assert_ok!` macro | `is_ok()` direct | `is_err()` direct | `unwrap_or_default()` | `assert!(false, …)` | `panic!()` | `unreachable!()` | `unwrap_or_else(panic)` | `fail_assert!` | `prop_assert!(false, …)` | `let _ = ` (problematic) |
|-------|---------------------|-------------------|---------------------|------------------------|----------------------|------------|-------------------|--------------------------|-----------------|---------------------------|----------------------------|
| vb_expr | 0 | ~25 (parser/lexer miri_tests + 5 in eval_tests.rs:3421-3425) | ~15 (proptest_type_enforcer.rs + 5 in eval_tests.rs) | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 (eval/property_tests bap, "no-panic" docstring tests) |
| vb_ipc | 89 (tests.rs) + 36 (frame/tests.rs) = 125 | 5 (handlers/tests.rs:182, 248, 458) | 0 (in `is_err()` form); 2 banned `if drain.is_err()` in benches | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 (frame_types.rs:140 + kani_flag_validation.rs:883 — production code) |
| vb_yaml | 0 | 1 (lib_tests.rs:1306) | 0 | 16 (source_map_tests.rs) | 0 | 0 | 0 | 0 | 72 (6 files) | 0 | 2 (events_tests.rs:583 + proptest_yaml_event_classification.rs:255 — both acceptable) |
| vb_queue_semantics | 0 | 0 | 0 | 0 | 0 | 0 | 54 (transitions/tests.rs) | 0 | 0 | 0 | 0 |
| vb_boundary_inventory | 0 | 8 (api_tests.rs:25-27 + L942 + vb_god2f_validation_properties.rs:94/97/166/175/232) | 2 (error_tests.rs:158, 169) | 0 | 22 (validation_tests.rs:13 + parser_tests.rs:9) | 0 | 0 | 0 | 0 | 7 (property_tests.rs:105/119/126/210/217/225/233) | 0 |
| vb_benchmark | 0 | 0 | 2 (benches/batched_atomicity.rs:374, 377) | 0 | 0 | 48 (benchmark_tests.rs:32 + batched_atomicity_tests.rs:16) | 0 | 0 | 0 | 0 | 8 (let _ = shard.enqueue) |
| vb_test_util | 0 | 0 | 0 | 0 | 0 | 12 (density_tests.rs) | 0 | 0 | 0 | 0 | 0 |
| vb_doc | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| vb_ajc40_flux | 0 | 0 | 0 | 0 | 6 (density_tests.rs:202/211/220/237/246/255) | 0 | 0 | 0 | 0 | 0 | 0 |
| vb_verification | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| **Slice 4 Total** | **125** | **~39** | **~19** | **16** | **28** | **60** | **54** | **0** | **72** | **7** | **14** (8 problematic in vb_benchmark + 2 acceptable in vb_yaml + 2 acceptable in vb_expr + 2 in vb_ipc production) |

Notes:
- `vb_ipc/src/array_queue_tests.rs` (the new active file, 942 lines) is **CLEAN** of all banned patterns: 0 `is_ok()` / `is_err()` / `Some(_)` / `unwrap_or_default()` / `let _ = ` sites, 0 `assert!(false, …)` sites, 0 `fail_assert!` invocations. All 33 tests use exact variant assertions (e.g., `assert_eq!(result, Err(IpcError::PayloadTooLarge { actual: ..., limit: ... }))`) or `prop_assert_eq!` on collected values. The test file is a model of strong-assertion BDD style and closes the round-1 FIFO defect.
- The orphan `vb_ipc/src/queue/tests/array_queue_tests.rs` is also clean of banned patterns (verified the fix at L784: `while let Ok(Some(_frame))`).
- `vb_queue_semantics` `unreachable!()` count rose from "30+" (round 3 estimate) to **54** (round 4 count). This is the same defect, not a regression — the round-3 count was a rough estimate.
- `vb_ajc40_flux` is NOT in the workspace (`Cargo.toml` `exclude = ["...", "crates/vb_ajc40_flux"]`); it must be tested via direct `cd` + `cargo test --tests --features positive` (works, 52 tests pass).
- `vb_verification` has ZERO behavior tests (still 0 after 4 review cycles).

### Mutation Gaps (top 5 most dangerous mutations that would NOT be caught)

1. **`MemoryIngress::try_recv` returns frames in reverse submission order** — **CLOSED in round 4.** The proptest at `crates/vb_ipc/src/array_queue_tests.rs:730-761` checks `received.iter().map(|f| f.run_id().as_u64()) == submitted.iter().map(...)` AND is now in the test binary (33 tests run). A real FIFO bug WOULD be caught. This was the round-1 mutation gap S4-004 / S4-R2-001 / S4-R3-001 — closed after 3 review cycles.

2. **`hydrate_run_frame` returns `Ok(BadFrame)` for valid input** (production in `vb_storage`, called from `crates/vb_verification/src/lib.rs`). Still the highest-severity mutation gap in the slice. `vb_verification` has 0 behavior tests in 4 consecutive review cycles. Kani harnesses only verify panic-freedom; the positive-case harness `hydrate_run_frame_postcond_ok` discards the result with `let _ = …`. `cargo test -p vb_verification --lib` reports 0 passed. (See S4-R3-019.)

3. **`build_semantic_source_map` returns `Err(YamlError::EmptySource)` for valid YAML** (production `crates/vb_yaml/src/source_map_build.rs`). 16 tests in `vb_yaml/src/source_map_tests.rs` use `unwrap_or_default()`, silently swallowing the error (S4-R3-003). The 72 `fail_assert!` invocations in 6 vb_yaml files (S4-R3-004) would also become no-ops if `assertion_failed` were mutated to return `true`. Both patterns ship silently.

4. **`VolatileRuntimeJournal::snapshot` returns `Err(JournalPoisoned)` for valid run state** (production in `vb_runtime`, called from `crates/vb_ipc/src/server/handlers/tests.rs:180, 246, 456`). The multi-line pattern `assert!(snapshot.is_ok(), ...); match snapshot { Ok(events) => events, Err(_) => return };` at 3 sites silently returns PASSED when snapshot fails (S4-R3-006). Net effect: 3 tests go green even when the runtime journal is broken.

5. **`load_fixture_source` returns `Ok(empty_workflow)` for valid YAML** (production `crates/vb_yaml/src/lib.rs`). The test at `vb_yaml/src/lib_tests.rs:1304-1308` uses `assert!(result.is_ok(), "load_fixture_source should accept valid workflow, got {result:?}")` — banned `is_ok()` shape with no content verification (S4-R3-011). A regression that silently drops workflow content goes undetected.

### Top 5 Fixes (ranked by impact-per-effort)

1. **Add `crates/vb_verification/tests/hydrate_run_frame_behavior.rs`** with 5 cases: (a) empty events + matching run_id → `Err(EmptyEvents)`; (b) non-matching run_id in snapshot → `Err(RunIdMismatch)`; (c) RunAccepted event present + matching run_id → `Ok(frame)` (assert frame.run_id == expected); (d) snapshot seq > tail seq → `Err(SeqOutOfOrder)`; (e) missing RunAccepted → `Err(MissingRunAccepted)`. Each test must assert the EXACT variant and key fields via `let Err(StorageError::Variant) = result else { panic!("expected Variant, got {result:?}") };`. **Effort: 1 hour. Closes the 4-cycle-old S4-R3-019 (was S4-020) and is the only CRITICAL-level coverage gap remaining in the slice.**

2. **Replace `assert!(false, ...)` (6 sites in `vb_ajc40_flux/tests/density_tests.rs:202, 211, 220, 237, 246, 255`) with `.expect(…)`**. Change `fn checked_pair_sum_zero_zero()` to return `Result<(), TestError>` and use `?` propagation. Each `match result { Some(v) => assert_eq!(v, 0), None => assert!(false, ...) }` becomes `match result { Some(v) => { prop_assert_eq!(v, 0); Ok(()) }, None => Err(TestError::Overflow("0 + 0 must not overflow")) }`. **Effort: 15 minutes, mechanical. Completes the round-1 fix vb-2kw49. Closes S4-R3-002.**

3. **Replace 16 `unwrap_or_default()` in `vb_yaml/src/source_map_tests.rs` (L240, 257, 342, 360, 377, 394, 411, 431, 463, 483, 497, 514, 564, 580, 641, 665) with `.expect("source map for valid YAML must succeed: {e:?}")`**. The 3 `build_source_map(yaml).unwrap_or_default()` sites at L564, L580, L665 follow the same pattern. **Effort: 10 minutes, mechanical. Closes S4-R3-003.**

4. **Delete the `assertion_failed` helper + `fail_assert!` macro from 6 vb_yaml test files** (`events_tests.rs`, `lib_tests.rs`, `profile_tests.rs`, `profile_tests_adversarial.rs`, `profile_error_variants_tests.rs`, `source_map_tests.rs`). Replace each `fail_assert!(…)` with `panic!(…)` (72 invocations). The `assertion_failed` function adds no value over `panic!`. **Effort: 20 minutes, mechanical. Closes S4-R3-004 + S4-016 (3-cycle-old).**

5. **Replace 3 `assert!(snapshot.is_ok(), ...); let events = match snapshot { Ok(events) => events, Err(_) => return };` patterns in `vb_ipc/src/server/handlers/tests.rs:180, 246, 456`** with `let events = journal.snapshot().expect("journal snapshot must succeed for valid run state");`. Same for the 4 bare `Err(_) => return` sites at L334, 346, 389, 401. **Effort: 10 minutes. Closes S4-R3-006 + S4-011 (3-cycle-old).**

### Verdict Line

STATUS: REJECTED

### Disposition

| Finding ID | Disposition |
|-----------|-------------|
| S4-R4-001 | owner_approved_debt (stale orphan duplicate at `crates/vb_ipc/src/queue/tests/array_queue_tests.rs`; 944 lines of dead code, not a mutation gap, but should be `git rm`'d for repository hygiene) |
| S4-R4-002 | owner_approved_debt (empty stub `crates/vb_ipc/src/queue/mod.rs`; 12 lines of doc comments, no production code, no `mod` declarations; either delete the entire `queue/` directory + `pub mod queue;` at `lib.rs:29` or move the active `array_queue_tests.rs` into `queue/tests/` and wire it via `mod tests;` in `queue/mod.rs`) |
| S4-R3-001 | **fixed_with_evidence** (round 3's CRITICAL blocker is now resolved: 33 `array_queue_tests::*` tests run; the round-1 FIFO fix is functional) |
| S4-R3-002 | owner_approved_debt (round-1 vb-2kw49 partial — 6 `assert!(false, …)` sites in `vb_ajc40_flux/tests/density_tests.rs`; same as round 3) |
| S4-R3-003 | owner_approved_debt (round-2 S4-R2-005 not addressed — 16 `unwrap_or_default()` in `vb_yaml/src/source_map_tests.rs`; same as round 3) |
| S4-R3-004 | owner_approved_debt (round-1 S4-016 + round-2 S4-R2-006 not addressed — 72 `fail_assert!` macro invocations in 6 vb_yaml files; same as round 3) |
| S4-R3-005 | owner_approved_debt (round-1 S4-005 + round-2 S4-R2-007 not addressed — 125 `assert_ok!` invocations in vb_ipc, ~20 bare-as-sole-assertion; same as round 3) |
| S4-R3-006 | owner_approved_debt (round-1 S4-011 + round-2 S4-R2-008 not addressed — 3 `Err(_) => return` + `assert!(snapshot.is_ok(), …)` sites in `vb_ipc/src/server/handlers/tests.rs`; same as round 3) |
| S4-R3-007 | owner_approved_debt (round-1 S4-015 + round-2 S4-R2-010 not addressed — 22 `assert!(false, …)` + 8 `is_ok()` + 7 `prop_assert!(false, …)` in vb_boundary_inventory; same as round 3) |
| S4-R3-008 | owner_approved_debt (round-2 S4-R2-016 not addressed — 54 `unreachable!()` in `vb_queue_semantics/src/transitions/tests.rs`; same as round 3, count rose from "30+" to 54) |
| S4-R3-009 | owner_approved_debt (round-2 S4-R2-009 not addressed — 8 `let _ =` + 8 `Ok(()) => {}` + 2 `is_err()` + 48 `panic!()` in vb_benchmark; same as round 3) |
| S4-R3-010 | owner_approved_debt (round-2 S4-R2-012 not addressed — 12 `panic!()` in `vb_test_util/tests/density_tests.rs`; same as round 3) |
| S4-R3-011 | owner_approved_debt (round-1 S4-006 + round-2 still present; `vb_yaml/src/lib_tests.rs:1306` smoke `is_ok()`; same as round 3) |
| S4-R3-012 | owner_approved_debt (round-1 S4-012 + round-2 S4-R2-015 not addressed; `ast.is_err()` / `tokens.is_err()` in vb_expr miri_tests; same as round 3) |
| S4-R3-013 | owner_approved_no_action (5 `prop_assert!().is_err()` in `eval_tests.rs:3421-3425` are redundant safety net; partition invariant at L3444 catches the wrong-variant mutation; same as round 3) |
| S4-R3-014 | **fixed_with_evidence** (round 3's blocker is closed: `while let Ok(Some(_frame))` is the correct pattern in both the active and orphan `array_queue_tests.rs` files) |
| S4-R3-015 | owner_approved_debt (round-2 S4-R2-011 not addressed; redundant `is_err()` after `matches!()` at `error_tests.rs:158, 169`; same as round 3) |
| S4-R3-016 | owner_approved_debt (round-2 S4-R2-017 not addressed; `Err(_) => return Ok(())` in `vb_yaml/src/property_tests/proptest_yaml_event_classification.rs:188, 225`; same as round 3) |
| S4-R3-017 | owner_approved_debt (round-1 S4-013 + round-2 S4-R2-020 not addressed; smoke `max_stack > 0` in `vb_expr/src/bytecode/tests.rs:416-426`; same as round 3) |
| S4-R3-018 | owner_approved_debt (round-1 S4-014 + round-2 S4-R2-019 not addressed; smoke `!debug_str.is_empty()` in `vb_boundary_inventory/src/tests/error_tests.rs:98-118`; same as round 3) |
| S4-R3-019 | owner_approved_debt (round-1 S4-020 + round-2 S4-R2-014 + round-3 still present; `vb_verification` has 0 behavior tests after 4 review cycles; same as round 3) |
| S4-R3-020 | owner_approved_debt (round-2 S4-R2-018 not addressed; `is_err()` in `vb_benchmark/benches/batched_atomicity.rs:374, 377`; same as round 3) |
| S4-R3-021 | owner_approved_no_action (test-helper `thread_local!` is local to test process; not a test-surface defect; same as round 3) |
| S4-R3-022 | owner_approved_no_action (`let _ = eval_expr_program(...)` in `bap_reference_does_not_fold` / `bap_helper_does_not_fold` is documented "no-panic" test per docstring; same as round 3) |
| (Round 1) vb-0to5y | fixed_with_evidence (8 short-circuit tests in `eval_tests.rs:669-757`; orphan file removed) |
| (Round 1) vb-2kw49 | partial (imports correct, but 6 `assert!(false, …)` sites remain; S4-R3-002) |
| (Round 1) vb-8r7cp | fixed_with_evidence (`MemoryIngress` at `vb_ipc/src/tests.rs:447`; no `crossbeam_channel` in code, only in doc comments) |
| (Round 1) vb-few2x | **fixed_with_evidence** (FIFO run_id order check applied at `array_queue_tests.rs:745-761`; functional because the file is now in the test binary; 33 tests run; closes S4-R3-001) |
| (Round 1) S4-005 | NOT FIXED (125 `assert_ok!` macro invocations in vb_ipc; S4-R3-005) |
| (Round 1) S4-006 | NOT FIXED (`vb_yaml/src/lib_tests.rs:1306`; S4-R3-011) |
| (Round 1) S4-007 | NOT FIXED (5 `is_ok()` in `vb_boundary_inventory/.../vb_god2f_validation_properties.rs`; folded into S4-R3-007) |
| (Round 1) S4-008 | NOT FIXED (`assert_io_ok` helper; folded into S4-R3-007) |
| (Round 1) S4-009 | STILL PRESENT (16 `is_err()` in `vb_expr/tests/proptest_type_enforcer.rs`; deferred in round 1) |
| (Round 1) S4-011 | NOT FIXED (3 `Err(_) => return` in `vb_ipc/src/server/handlers/tests.rs`; S4-R3-006) |
| (Round 1) S4-012 | NOT FIXED (`ast.is_err()` in miri_tests; S4-R3-012) |
| (Round 1) S4-013 | NOT FIXED (smoke `max_stack > 0`; S4-R3-017) |
| (Round 1) S4-014 | NOT FIXED (smoke `!debug_str.is_empty()`; S4-R3-018) |
| (Round 1) S4-015 | NOT FIXED (29 `assert!(false, …)`; S4-R3-007) |
| (Round 1) S4-016 | NOT FIXED (72 `fail_assert!`; S4-R3-004) |
| (Round 1) S4-020 | NOT FIXED (vb_verification 0 behavior tests; S4-R3-019) |
| (Round 2) S4-R2-001 | **fixed_with_evidence** (file is no longer dead code; 33 tests run; S4-R3-001 closed) |
| (Round 2) S4-R2-002 | fixed_with_evidence (queue_boundary.rs wired; 19 boundary tests run) |
| (Round 2) S4-R2-003 | fixed_with_evidence (12 `proptest_bytecode_ast_parity` tests pass; production bug fixed) |
| (Round 2) S4-R2-004 | NOT FIXED (6 `assert!(false, …)`; S4-R3-002) |
| (Round 2) S4-R2-005 | NOT FIXED (16 `unwrap_or_default()`; S4-R3-003) |
| (Round 2) S4-R2-006 | NOT FIXED (72 `fail_assert!`; S4-R3-004) |
| (Round 2) S4-R2-007 | NOT FIXED (125 `assert_ok!`; S4-R3-005) |
| (Round 2) S4-R2-008 | NOT FIXED (3 `Err(_) => return`; S4-R3-006) |
| (Round 2) S4-R2-009 | NOT FIXED (silent `let _ =` and `Ok(()) => {}` in vb_benchmark; S4-R3-009) |
| (Round 2) S4-R2-010 | NOT FIXED (29 `assert!(false, …)` in vb_boundary_inventory; S4-R3-007) |
| (Round 2) S4-R2-011 | NOT FIXED (redundant `is_err()`; S4-R3-015) |
| (Round 2) S4-R2-012 | NOT FIXED (60 `panic!()`; S4-R3-009 + S4-R3-010) |
| (Round 2) S4-R2-014 | NOT FIXED (vb_verification 0 behavior tests; S4-R3-019) |
| (Round 2) S4-R2-015 | NOT FIXED (`ast.is_err()`; S4-R3-012) |
| (Round 2) S4-R2-016 | NOT FIXED (54 `unreachable!()`; S4-R3-008) |
| (Round 2) S4-R2-017 | NOT FIXED (`Err(_) => return Ok(())`; S4-R3-016) |
| (Round 2) S4-R2-018 | NOT FIXED (`is_err()` in benches; S4-R3-020) |
| (Round 2) S4-R2-019 | NOT FIXED (smoke `!debug_str.is_empty()`; S4-R3-018) |
| (Round 2) S4-R2-020 | NOT FIXED (smoke `max_stack > 0`; S4-R3-017) |
| (Round 3) S4-R3-001 | **fixed_with_evidence** (CRITICAL closed: `mod array_queue_tests;` at `lib.rs:98` wires 33 tests; FIFO order check is now functional; previously a 3-cycle-old regression, now resolved) |
| (Round 3) S4-R3-014 | **fixed_with_evidence** (type error at `array_queue_tests.rs:774` resolved: `while let Ok(Some(_frame))` is the correct pattern in both the active and orphan files) |
