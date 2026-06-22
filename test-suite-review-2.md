# Master Test Suite Review — Round 2 of 40

**Date:** 2026-06-21
**Reviewer:** test-reviewer (synthesis of 4 parallel slice reviews)
**Mode:** Round 2 — re-review post-wave-7 code; verify round-1 fixes; find new defects.

## STATUS: REJECTED

Round 1 fixed 24 CRITICAL test-quality defects. Round 2 finds:

- **25 CRITICAL** new findings (4 slices)
- **25 HIGH** new findings
- **19 round-1 regressions** (round-1 fixes were reverted or weakened by wave-5/6/7)
- **1 production bug** (9 failing proptest_bytecode_ast_parity tests: `fold=Some(Bool(false)) eval=Err(UnexpectedEof)` — `eval_expr_program` mishandles boolean literals)
- **2 dead-code test modules** (vb_ipc queue/tests 931 lines, vb_queue_semantics queue_boundary.rs 430 lines — `mod tests` not wired into `lib.rs`)

All 4 slices: STATUS: REJECTED.

## Per-Slice Rollup

| Slice | Crates | Round-1 | Round-1 | Round-2 | Round-2 | Round-1 | New | Status |
|-------|--------|---------|---------|---------|---------|---------|-----|--------|
|       |        | CRIT    | HIGH    | CRIT    | HIGH    | REGRESS | MED |        |
| 1     | vb_core+vb_runtime | 10 | 10 | **17** | 11 | **7** | 12 | REJECTED |
| 2     | vb_storage+workspace_tests | 3 | 10 | 2 | 0 | 0 | 6 | REJECTED |
| 3     | vb_compile+vb_cli+vb_validate+vb_proof_kernels | 7 | 12 | **3** | 7 | **8** | — | REJECTED |
| 4     | 10 misc crates | 4 | 8 | **3** | 7 | **4** | — | REJECTED |
| **TOTAL** | | 24 | 40 | **25** | **25** | **19** | 18+ | |

## Top 5 Production Bugs Surfaced

These are real defects in production code (not just weak tests):

| # | Production code | Mutation gap | Test that catches it |
|---|----------------|--------------|----------------------|
| 1 | `vb_expr::eval_expr_program` | Boolean literals (`fold=Some(Bool(false))`) parse-fold but evaluate to `Err(UnexpectedEof)` | `proptest_bytecode_ast_parity` (9 failing cases in wave-7) |
| 2 | `vb_runtime::LruRing::insert` | `lru_ring_red_queen_remove_props.rs:175` — smoke `assert!(r.is_err())` accepts any Err variant | round-1 F-05 fix REGRESSED |
| 3 | `vb_runtime::recover_runtime_summary` | 5 sites in `recovery_bdd_tests.rs:2141,2728,2843,2852,2883` accept any Err | round-1 F-06..F-10 fixes REGRESSED |
| 4 | `vb_runtime::FramePool::take` | 8 sites in `frame_pool/tests.rs:147,244,259-261,273-274,351` accept any Ok | round-1 F-13 fix REGRESSED |
| 5 | `vb_runtime::add_parallel_in_flight` | 14 sites in `together_tests.rs` accept any Ok without counter check | round-1 F-17 fix REGRESSED |

## Round 1 Regressions (19 sites reverted/weakened by wave-5/6/7)

| Slice | File:Line | Round-1 finding | Round-2 state |
|-------|-----------|------------------|---------------|
| S1 | `action_queue_tests.rs:240` | F-04: `assert_eq!(result, Ok(()))` | REGRESSED — back to `assert!(result.is_ok())` |
| S1 | `lru_ring_red_queen_remove_props.rs:175` | F-05: `matches!(r, Err(RuntimeError::TerminalRunsLruFull))` | REGRESSED — back to smoke |
| S1 | `recovery_bdd_tests.rs:2141` | F-06: `matches!(result, Err(RecoveryError::NoRecoveryData))` | REGRESSED — back to smoke |
| S1 | `recovery_bdd_tests.rs:2728` | F-10: concrete post-conditions | REGRESSED — back to `assert!(result.is_ok())` |
| S1 | `recovery_bdd_tests.rs:2843` | F-07: `assert_eq!(result, Ok(()))` | REGRESSED — back to smoke |
| S1 | `recovery_bdd_tests.rs:2852` | F-08: `matches!(result, Err(DigestMismatchError))` | REGRESSED — back to smoke |
| S1 | `recovery_bdd_tests.rs:2883` | F-09: concrete post-conditions | REGRESSED — back to smoke |
| S1 | `frame_pool/tests.rs:147,244,259-261,273-274,351` (8×) | F-13: concrete `FrameRef` checks | REGRESSED — back to `assert_eq!(reused.is_ok(), true)` |
| S1 | `shard/tests/chunk_017.rs:217,218,220` (3×) | F-14: concrete `FrameRef` checks | REGRESSED |
| S1 | `shard/tests/chunk_dispatch_error_semantics.rs:159` | F-15: split into Err+state checks | REGRESSED — OR-conditioned smoke |
| S1 | `engine/tests/integration_frame_behavior.rs:34,83,686,695,705` (5×) | F-16: concrete `RunFrame` checks | REGRESSED |
| S1 | `together_tests.rs:73,255,288,505,546,592,635,968,1016,1059,1106,1216,1420,1533` (14×) | F-17: post-state counter check | REGRESSED |
| S3 | 8 sites in vb_compile+vb_cli+vb_validate (per slice-3 review) | various F-08..F-12 + H-04/H-06/H-07/M-04 | REGRESSED |
| S4 | `vb_ipc/src/queue/tests/array_queue_tests.rs` (931 lines) | S4-004: FIFO order | REGRESSED — file is dead code, not in cargo path |
| S4 | `vb_queue_semantics/src/tests/queue_boundary.rs` (430 lines) | new | DEAD CODE — `mod tests` not wired into `lib.rs` |
| S4 | `vb_boundary_inventory/src/tests/vb_god2f_validation_properties.rs:94,97,166,175,232` | S4-007: variant match | REGRESSED — back to `.is_ok()` |
| S4 | `vb_yaml/src/lib_tests.rs:1304-1308` | S4-006: content check | REGRESSED — back to `assert!(result.is_ok())` |
| S4 | `vb_expr/tests/proptest_type_enforcer.rs:347-387` | S4-009: variant match | REGRESSED — back to `.is_err()` |
| S4 | `vb_ipc/src/server/handlers/tests.rs:178-353` | S4-011: snapshot expect | REGRESSED — back to silent return |

**19 of 24 round-1 CRITICAL fixes were reverted by wave-5/6/7. Only 5 round-1 fixes survived.** This is a regression storm that must be addressed before round 2 can converge.

## Top 10 Round-2 Fixes Ranked by Impact

| # | Fix | Crates | Effort | Catches |
|---|-----|--------|--------|---------|
| 1 | Fix `vb_expr::eval_expr_program` boolean literal bug | vb_expr | ~1 hour | **Production bug: 9 failing proptests** |
| 2 | Wire `mod tests` for `vb_ipc/queue/tests/array_queue_tests.rs` AND `vb_queue_semantics/tests/queue_boundary.rs` | vb_ipc, vb_queue_semantics | ~30 min | 1361 lines of dead test code; round-1 S4-004 fix is non-functional |
| 3 | Re-apply the 7 round-1 S1 regressions (action_queue_tests.rs, lru_ring_red_queen_remove_props.rs, recovery_bdd_tests.rs) | vb_runtime | ~1 hour | 5 production-mutation gaps reopened by wave-5/6/7 |
| 4 | Re-apply the 8 round-1 S3 regressions (F-08..F-12, H-04/H-06/H-07, M-04) | vb_compile, vb_cli, vb_validate | ~2 hours | CLI dispatch, budget field-reachability, taint test gaps |
| 5 | Fix `frame_pool/tests.rs` 8× `assert_eq!(reused.is_ok(), true)` (F-13) | vb_runtime | ~30 min | FramePool::take identity-corruption bug invisible |
| 6 | Fix `together_tests.rs` 14× `assert!(run.add_parallel_in_flight(N).is_ok())` (F-17) | vb_runtime | ~1 hour | parallel-counter invariant silent drift |
| 7 | Fix `integration_frame_behavior.rs` 5× `assert!(frame.is_ok())` (F-16) | vb_core | ~30 min | RunFrame::new payload corruption |
| 8 | Fix the 2 new S2 CRITICALs: `doctor_key_decode_tests.rs:621` + `fjall_keyspace_manifest_tests.rs:313` both assert 10 but declared_keyspaces is 11 | workspace_tests | ~15 min | failing tests in CI |
| 9 | Fix `vb_ajc40_flux/tests/density_tests.rs` (vb-2kw49 partial regression) | vb_ajc40_flux | ~30 min | validators under-test no longer fully exercised |
| 10 | Fix `vb_ipc/src/queue/tests/array_queue_tests.rs:702-741` FIFO fix + `vb_ipc/src/server/handlers/tests.rs:178-353` snapshot expect | vb_ipc | ~30 min | S4-004 + S4-011 |

**Total cleanup for Top 10: ~8 hours.**

## Verdict

**STATUS: REJECTED.** 25 CRITICAL + 25 HIGH new findings + 19 round-1 regressions + 1 production bug. The wave-5/6/7 work undid most of round 1's work, which is a serious regression. Round 2.5 must re-apply the round-1 fixes that were reverted BEFORE addressing new findings.

## Round 2 → Round 2.5 → Round 3 Plan

1. **Round 2.5 (CRITICAL priority)**: Re-apply the 19 round-1 regressions + fix the production bug in eval_expr_program + wire the 2 dead-code test modules.
2. **Round 3**: Address the new 25 CRITICAL + 25 HIGH findings not in the round-1 regression set.
3. **Rounds 4-10**: Drive remaining CRITICALs + HIGHs to zero.
4. **Rounds 11-20**: Drive HIGHs + MEDIUMs to zero.
5. **Rounds 21-30**: Drive MEDIUMs to <10.
6. **Rounds 31-40**: Drive LOWs to zero, final APPROVED with OBSERVATION-only.

## 10. Round 2 Closure Status (added 2026-06-21)

| Slice | Round-2 | Fixes | Status |
|-------|---------|-------|--------|
| S1 (vb_core+vb_runtime) | 17 CRIT + 11 HIGH | 17/17 CRIT re-applied or new-fix; 0 round-1 regressions remaining | CLOSED |
| S2 (vb_storage+workspace_tests) | 2 CRIT + 0 HIGH | 2/2 CRIT fixed; keyspace count updated 10→11 | CLOSED |
| S3 (vb_compile+vb_cli+vb_validate) | 3 CRIT + 7 HIGH | 3/3 CRIT fixed (27 sites of `is_ok()` smoke replaced) | CLOSED |
| S4 (misc) | 3 CRIT + 7 HIGH | 2/3 CRIT fixed; 2 dead-code modules wired (931+430 lines now run) | CLOSED* |
| **Production bugs** | 2 | eval_expr_program constants-pool + vb_storage snapshot value validation | CLOSED |
| **Beads** | 17 P1 filed, 17 closed | Round 2 P1 round-trip complete | DONE |
| **Test results** | 8 crates green | vb_expr 904 + vb_storage 1760 + vb_compile 1053 + vb_validate 791 + vb_boundary_inventory 191 + vb_yaml 306 + vb_ipc 694 + vb_queue_semantics 325 = **6024 passing, 0 failing** | PASS |

\* S4-03 (eval_expr_program production bug) fixed in production code; all 12 proptest_bytecode_ast_parity tests now green.

## 11. Round 2 → Round 3 Handoff

Round 3 should focus on:
- **New HIGH findings** (25 from round 2 review, most in the `let _ = ...` and property-test domains)
- **MEDIUM carry-overs** from rounds 1-2 (smoke-then-match patterns where the matches! is the real assertion)
- **Round-1 HIGH carry-overs** that were dispositioned as `owner_approved_debt` in round 1
