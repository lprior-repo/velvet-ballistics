# Wave 2 Agent 02 — Runtime / Action / Durability / Shard Bug Sweep (read-only scout)

**Scope:** 18 bug IDs from `/tmp/wave2-chunk-02.txt`
**Mode:** Read-only explore. No source modified. No beads created.
**Date:** 2026-06-24
**Note on scope:** This chunk covers runtime primitives (`together.rs`, `collect.rs`),
shard lifecycle/transitions, budget traversal, replay error routing, recovery taint,
shard config validation, value-store panic suppression, and kani harness parse errors.
Verdict reflects actual code state in `/home/lewis/src/velvet-ballistics`, not
isolated worktrees.

Bead status snapshot at session start:
- 13 CLOSED (including duplicates)
- 1 IN_PROGRESS (vb-4uert, vb-5dfez)
- 1 OPEN (vb-4pwif — duplicate tracking bead, parent vb-zfyh5 already CLOSED)
- All dupe-of-closed parents already CLOSED: vb-4bz9b→vb-tpbgl, vb-5fxrk→vb-zfyh5, vb-5mnsf→vb-6zr4c, vb-6c4qe→vb-ch8og.

---

## Evidence table

| bug-id | pri | files-touched | test-file | targeted-cmd | result | verdict | evidence |
|--------|----:|---------------|-----------|--------------|--------|---------|----------|
| vb-42nj8 (RP-014) | P1 | `crates/vb_runtime/src/primitives/together.rs:37` | `crates/vb_runtime/src/together_tests.rs` (no RP-014 regression test) | `cargo test -p vb_runtime --lib together` (46 pass) | fail | NOT-PATCHED | `together_start` at line 37 still mutates `run.add_parallel_in_flight(count)?` BEFORE fallible work at lines 38-40 (`require_output`, `store.insert_list`, `run.write_slot`). No regression test exercises counter rollback when 38-40 fail. `together_start_returns_error_when_output_missing` (together_tests.rs:116) only asserts error variant; never inspects counter. Patch exists only in isolated worktrees, not merged. |
| vb-4bq3r (RP-003) | P2 | `crates/vb_runtime/src/primitives/together.rs:34` | `crates/vb_runtime/src/together_tests.rs` (no RP-003 regression test) | `cargo test -p vb_runtime --lib together` (46 pass) | fail | NOT-PATCHED | `together_start` at line 34 still uses `current.saturating_add(count) > max`. When `current + count` overflows u16 (current=10, count=u16::MAX), saturating_add yields u16::MAX; if `max == u16::MAX` the comparison is false and the code proceeds to `add_parallel_in_flight` which returns `InternalInvariantViolation { reason: "parallel_in_flight overflow" }` (frame.rs:203) — wrong variant. Proper `checked_add` fix exists only in isolated worktree `vb-4bq3r/primitives/together.rs:43`, not in main. |
| vb-4bz9b (RP-015) | P2 | `crates/vb_runtime/src/primitives/collect.rs:537-571` (via vb-tpbgl dup) | `crates/vb_runtime/src/primitives/collect/tests.rs:3058` | `cargo test -p vb_runtime --lib collect_next_rejects_empty` (1 pass) | pass | PATCHED | Bead CLOSED as duplicate of `vb-tpbgl`. Real fix: empty-page path at collect.rs:537-571 (`accept_empty_collect_page`) now rejects non-current empty pages via `require_current_page` (line 562) returning `EngineError::CollectPageOrderViolation { kind: NonCurrentPage }`. Test at collect/tests.rs:3058 explicitly verifies rejection. |
| vb-4jmw5 (FINDING-006) | P1 | (test file removed) `crates/vb_runtime/src/shard/transitions.rs:149-189` (await_timer) | `crates/vb_runtime/src/shard/tests/chunk_029.rs:357` | `cargo test -p vb_runtime --lib runtime_ask_timer_append_failure_does_not_register_pending_timer` (1 pass) | pass | PATCHED | Original test file `chunk_002_p1_bug_hunt_fixes.rs` no longer exists; production fix absorbed. `await_timer` at transitions.rs:149-189 journals BEFORE registering pending_timer (line 168-177); on append failure (line 174-176) it rolls back via `run_state_insert` and returns the journal error, never registering a timer. `runtime_ask_timer_append_failure_does_not_register_pending_timer` (chunk_029.rs:357) passes — confirms pending_timer is NOT registered after append failure, journal has no `AskScheduled` event. |
| vb-4lpa6 (RS-008) | P2 | (field removed) | (no test) | `rg "coalesce_window_ticks" crates/vb_runtime/src` → 0 hits | n/a | PATCHED | `ShardConfig` struct (types.rs:709-720) no longer has `coalesce_window_ticks`/`snapshot_interval_steps`/`max_terminal_runs`/`terminal_runs_ttl_ticks` fields. The off-by-one site `dispatch.rs:25-65` was removed. Zero `coalesce` matches in entire `crates/vb_runtime/src` tree. Validation gap closed by field removal. |
| vb-4pwif (RA-014 dup) | P3 | `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:142-156` (stale path; fix is elsewhere) | `crates/vb_runtime/src/shard/types.rs:1784-1927` (introspection_poison_regression_tests) | `cargo test -p vb_runtime --lib introspection_poison_regression_tests` (6/6 pass) | pass | PATCHED | Bead's cited path `shard/impl_parts/chunk_001.rs:142-156` is `reserve_index_map_slot`/`reserve_index_set_slot` helpers — no mutex involved; bead description mismatch. Real RA-014 fix is at `shard/types.rs:378` `IntrospectionRegistry::lock_or_recover` using `poisoned.into_inner()` (standard Rust recovery pattern). 6 regression tests pass. |
| vb-4sn2l (await_timer durable journal) | P0 | `crates/vb_runtime/src/shard/transitions.rs:149-189` (await_timer), `transitions.rs:114-146` (await_action) | `crates/vb_runtime/src/shard/tests/chunk_029.rs:357` | `cargo test -p vb_runtime --lib runtime_ask_timer_append_failure_does_not_register_pending_timer` (1 pass) | pass | PATCHED | `Shard::append_journal_event_durable` (originally added per bead) was removed in subsequent wave-17 cleanup; durability became intrinsic because the coalesce buffer was deleted entirely. `append_journal_event` (chunk_001.rs:189-194) now goes straight to `journal.append_sequenced` with no buffering path. `await_timer` journals synchronously before pending_timer registration; append failure rolls back state. Test passes — no double-ask hazard. |
| vb-4uert (CB-001) | P2 | `crates/vb_core/src/budget.rs:1422-1425` (visit_node_for_total_steps) | `crates/vb_core/src/budget/tests.rs:3896` (count_total_steps_overflow_returns_step_count_overflow) | `cargo test -p vb_core --lib step_count` (15 pass) | fail | NOT-PATCHED | Line 1424 still returns `BudgetTraversalError::StepOutOfBounds { step: current }` when `total.checked_add(1)` is None (u64 step-count overflow). Should return `BudgetTraversalError::StepCountOverflow { actual: u64::MAX }`. Existing test `count_total_steps_overflow_returns_step_count_overflow` exercises a DIFFERENT overflow path (loop-multiplication at ForEachStart `limit=u32::MAX`, line 1461-1572); the per-visit single-step overflow path remains buggy. No regression test for the single-step `checked_add` overflow. Bead remains IN_PROGRESS. |
| vb-4y4dt (RS-006) | P2 | `crates/vb_runtime/src/shard/transitions.rs:87-112` (finish_run), `transitions.rs:194-209` (fail_run_state) | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs:269-298` | `cargo test -p vb_runtime --lib finish_run` (1 pass) | pass | PATCHED | `finish_run` (transitions.rs:94-102) journals `RunFinished` BEFORE mutating state; on append failure (line 100) restores state via `run_state_insert` and returns the journal error. Same pattern in `fail_run_state` (line 196-199). Verified by commit `71c912c98` "fix(vb_runtime): RS-006 finish_run/fail_run_state journal-before-state-removal". Test `finish_run_appends_run_finished_event_and_inserts_terminal_run` (chunk_005.rs:269) passes — `RunFinished` event in journal, `terminal_runs` populated. |
| vb-55qnv (engine_to_replay_err) | P4 | `crates/vb_core/src/replay/mod.rs:219-227` (new_replay_frame) | (no specific test) | `cargo test -p vb_core --lib replay` | n/a | PATCHED | `new_replay_frame` (mod.rs:219) now uses `.map_err(engine_to_replay_err)` (line 226), routing `RunFrame::new`'s `InvalidProgramCounter { step }` through the `engine_to_replay_err` translator (mod.rs:141) so the step index is preserved as `ReplayError::InvalidProgramCounter { step }` instead of being squashed to `ReplayError::Internal { reason: "failed to create run frame" }`. Bead CLOSED. |
| vb-574zr (SR-013) | P2 | `crates/vb_storage/src/recovery/replay/summary.rs:746-752` | `crates/vb_storage/src/recovery/tests.rs:2372` (hydrate_run_frame_from_events_accepts_legacy_frame_extra_without_taint_sidecar) | `cargo test -p vb_storage --lib` (passes) | fail | NOT-PATCHED | **`legacy_slot_taint` at summary.rs:748 STILL returns `Taint::Clean` for `SlotValue::Bool(false)`** — the asymmetric Bool heuristic is unchanged. Pattern is `Bool(false) => Clean`, `Bool(true)/Null => DerivedFromSecret`, `_ => Secret`. Bead close_reason claims "Addressed in wave 8" but the actual code does the opposite. The only test that writes `Bool(false)` (recovery/tests.rs:2372) only asserts `result.is_ok()` — never inspects the returned taint — so the leak is invisible to the test suite. **Wave 1 already flagged this NOT-PATCHED; not merged since.** |
| vb-5dfez (kani parse error) | P2 | `crates/vb_runtime/src/verification/kani/kani_ask_answer_lifecycle.rs:117-121` | (kani-only) | `cargo check -p vb_runtime --lib` → clean | pass | PATCHED | File is `#[cfg(kani)]`-gated (line 17). No `kani::,` parse-error token exists at lines 117-121; lines now contain a clean `match append_result { Ok(()) => ... Err(_) => { kani::assert!(...); kani::cover!(...); } }`. Commit `7b8365940` "chore: cargo fmt sweep + repair kani::assert botches" and `02f755f1e` "fix(vb_runtime/kani): resolve all kani harness duplicates" repaired the broken expression. Bead remains IN_PROGRESS but the parse error is gone — the `cargo check` and `cargo build` are clean. The "blocks cargo fmt" claim was inaccurate since the file is kani-gated. |
| vb-5fxrk (RA-014 dup) | P3 | (duplicate of vb-zfyh5) `crates/vb_runtime/src/shard/types.rs:378` | `crates/vb_runtime/src/shard/types.rs:1827-1926` | `cargo test -p vb_runtime --lib introspection_poison_regression_tests` (6/6 pass) | pass | PATCHED | Duplicate of parent `vb-zfyh5` (CLOSED). Fix is `IntrospectionRegistry::lock_or_recover` (types.rs:378) using `poisoned.into_inner()`. All register/unregister paths use it. 6 regression tests pass. |
| vb-5j3we (QA-W0-12 re-verify) | P0 | workspace-level | n/a (re-verification) | `cargo check --workspace --lib --all-targets` → clean; `cargo test -p vb_runtime --lib together_start` (13 pass) | pass | PATCHED | Workspace compiles cleanly across all 12 crates (vb_core, vb_runtime, vb_storage, vb_validate, vb_compile, vb_expr, vb_yaml, vb_ipc, vb_verification, vb_cli, xtask, workspace_tests). vb-5j3we was a meta bead to re-verify after wave-1+2 fixes; the verification IS the deliverable, and it passes. |
| vb-5mnsf (CB-004 dup) | P3 | (duplicate of vb-6zr4c) `crates/vb_core/src/budget.rs:184,201,2101`; `crates/vb_core/src/workflow/mod.rs:411` | `crates/vb_core/src/budget/tests.rs:2207-2269` (depth_overflow_tests) | `cargo test -p vb_core --lib depth_overflow_tests` (3/3 pass) | pass | PATCHED | Duplicate of parent `vb-6zr4c`. Real fix: `BudgetTraversalError::DepthOverflow { depth: u16 }` variant added; `compute_child_depth` returns it instead of lossy `StepCountOverflow { actual: u64::MAX }`; `WorkflowError::DepthOverflow { depth: u16 }` added in `vb_core/src/workflow/mod.rs:411`. 3 regression tests pass. |
| vb-60lf2 (RQ-W0-15) | P1 | `crates/vb_runtime/src/shard/types.rs:709-720` (ShardConfig), `crates/vb_runtime/src/shard/impl_parts/chunk_003.rs:3-39` (impl ShardConfig::new) | `crates/vb_runtime/src/shard/tests/chunk_012.rs`, `chunk_021.rs`, `chunk_030.rs`, `impl_tests/chunk_001.rs` | `cargo test -p vb_runtime --lib shard_config` (21 pass) | pass | PATCHED | `ShardConfig::new` (chunk_003.rs:5-38) validates `command_queue_capacity`, `trace_capacity`, `step_budget_per_tick`, `max_active_runs`. Fields `coalesce_window_ticks`/`snapshot_interval_steps`/`max_terminal_runs`/`terminal_runs_ttl_ticks` were removed entirely (types.rs:709-720 only has 5 fields). 21 `shard_config_*` tests pass including boundary cases and zero-rejections. |
| vb-69w82 (HOLZ-VSTORE-01) | P0 | `crates/vb_core/src/value_store.rs:420-449` (kani_harnesses mod) | `crates/vb_core/src/value_store/tests.rs` (production tests) | `cargo test -p vb_core --lib value_store` (169 pass); `cargo check -p vb_core` clean | pass | PATCHED | `assert!(false)` calls at value_store.rs:433,435,443,444,447 are inside `#[cfg(kani)] mod kani_harnesses` (line 420). They never compile into the production binary. Bead's "production assert!" framing was inaccurate — the assertions are kani-only and act as Kani proof obligations (intentional panic under `Err(_)` arm = property violation). `cargo check -p vb_core` clean; 169 value_store tests pass. |
| vb-6c4qe (RS-025 dup) | P3 | (duplicate of vb-ch8og) `crates/vb_runtime/src/counters.rs:48-63` | `crates/vb_runtime/src/counters/tests.rs` (add_steps_*) | `cargo test -p vb_runtime --lib "counters::tests::add_steps"` (4 pass) | pass | PATCHED | Duplicate of parent `vb-ch8og`. Real fix in commit `9d27a90bf` "vb-mksxs: make ShardCounters::add_steps saturate on overflow via compare_exchange". counters.rs:48-63 now uses CAS loop with `saturating_add` and short-circuits when `current == next` (already at u64::MAX). 4 `add_steps_*` tests pass including `add_steps_saturates_at_u64_max_on_overflow`. |

---

## Summary

- **bugs-checked:** 18
- **PATCHED:** 14 (vb-4bz9b, vb-4jmw5, vb-4lpa6, vb-4pwif, vb-4sn2l, vb-4y4dt, vb-55qnv, vb-5dfez, vb-5fxrk, vb-5j3we, vb-5mnsf, vb-60lf2, vb-69w82, vb-6c4qe)
- **NOT-PATCHED:** 4 (vb-42nj8, vb-4bq3r, vb-4uert, vb-574zr)
- **PARTIAL:** 0
- **UNKNOWN:** 0

Two IN_PROGRESS beads (vb-4uert, vb-5dfez): one (vb-4uert) is a real NOT-PATCHED bug;
the other (vb-5dfez) is fixed in code but bead not yet closed.

One OPEN bead (vb-4pwif) — duplicate tracking bead whose parent (vb-zfyh5) is CLOSED
and whose underlying bug is fixed. vb-4pwif can be closed.

---

## Top-3 NOT-PATCHED with file:line and brief reason

1. **vb-574zr (SR-013) — `crates/vb_storage/src/recovery/replay/summary.rs:748`**
   `legacy_slot_taint` STILL maps `SlotValue::Bool(false) => Taint::Clean`, leaking secret
   `Bool(false)` slots through legacy hydrate paths as Clean. Bead was CLOSED claiming
   a wave-8 fix at `slots/taint.rs:47-48` (file does not exist) — no such file was ever
   created and the summary.rs function was never updated. The regression test
   `hydrate_run_frame_from_events_accepts_legacy_frame_extra_without_taint_sidecar`
   (recovery/tests.rs:2372) writes `Bool(false)` but only asserts `result.is_ok()` —
   never inspects the returned taint, so the leak is invisible to CI.

2. **vb-42nj8 (RP-014) — `crates/vb_runtime/src/primitives/together.rs:37`**
   `together_start` mutates `run.add_parallel_in_flight(count)?` at line 37 BEFORE the
   fallible work at lines 38-40 (`require_output`, `store.insert_list`, `run.write_slot`).
   If any of 38-40 fail, the counter is leaked. Existing tests cover happy-path and
   no-output-missing (which triggers at line 38, AFTER counter mutation) but no test
   verifies counter rollback when the mutation at line 37 is followed by a fallible
   error at line 39 (`insert_list` BudgetExceeded). Fix exists only in isolated
   worktree; not merged to main.

3. **vb-4bq3r (RP-003) — `crates/vb_runtime/src/primitives/together.rs:34`**
   `together_start` still uses `current.saturating_add(count) > max`. When `current +
   count` would overflow u16 (e.g., current=10, count=u16::MAX-5=65530), saturating_add
   clamps at u16::MAX. If `max >= u16::MAX` (or happens to be equal), the comparison
   `u16::MAX > max` may be false, then `add_parallel_in_flight(count)` (frame.rs:202-207)
   detects overflow via `checked_add` and returns `CoreError::InternalInvariantViolation`
   — wrong variant. Proper fix would be `checked_add(...).ok_or(...).and_then(|sum| if
   sum > max { Err(ParallelLimitExceeded) } else { Ok(()) })` or equivalent — exists
   only in isolated worktree `vb-4bq3r/primitives/together.rs:43`.

(Honorable mention: vb-4uert — `crates/vb_core/src/budget.rs:1424` still returns
`StepOutOfBounds` for the per-visit `total.checked_add(1)` overflow path. Existing
`count_total_steps_overflow_returns_step_count_overflow` test passes but exercises
a different code path — loop-multiplication overflow, not single-step increment
overflow. Bead is IN_PROGRESS.)

---

## Notes on test infrastructure gaps

- **`legacy_slot_taint` has no meaningful regression test.** The single test that
  exercises `Bool(false)` legacy hydrate (recovery/tests.rs:2372) only checks
  `result.is_ok()`. Adding an assertion on the returned taint would catch the bug.
- **`parallel_in_flight` counter leak in `together_start` has no rollback test.**
  `together_start_returns_error_when_output_missing` (together_tests.rs:116) triggers
  the error at line 38 AFTER counter mutation but never inspects counter state
  afterward. A test that asserts `run.parallel_in_flight() == 0` after the error
  would catch the leak.
- **`together_start` saturating-add overflow path has no test.** Phase23 tests cover
  u16 branch count limits (count itself) but not the `current + count` overflow at
  the parallel_in_flight comparison site. A test with `current = 10`, `count =
  u16::MAX - 5`, `max = u16::MAX` would expose the `InternalInvariantViolation`
  variant mismatch.
- **`visit_node_for_total_steps` single-step overflow has no test.** Existing
  `count_total_steps_overflow_returns_step_count_overflow` exercises the loop-
  multiplication overflow path. A test that exercises `total.checked_add(1)`
  overflow at line 1422-1424 would catch the wrong-variant return.

## Output file

Written to: `/home/lewis/src/velvet-ballistics/to-fix/wave2/agent-02-explore.md`
