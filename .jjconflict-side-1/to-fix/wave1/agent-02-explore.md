# Wave 1 Agent 02 — Compiler/YAML/IR Validation Bug Sweep (read-only scout)

**Scope:** 11 bug IDs from `/tmp/wave1-chunk-02.txt`
**Mode:** Read-only explore. No source modified. No beads created.
**Date:** 2026-06-24
**Note on scope:** Despite the "compiler/YAML/IR validation" framing, this chunk
contains runtime/storage/budget bugs (RP, SR, SC, CB, RE, RA prefixes), not
vb_compile/vb_yaml/vb_validate defects. Verdict table reflects actual code
state in `/home/lewis/src/velvet-ballistics` (the canonical repo), not the
state in isolated worktrees under `/home/lewis/src/isolated/`.

---

## Evidence table

| bug-id | pri | files-touched | test-file | targeted-cmd | result | verdict | evidence |
|--------|----:|---------------|-----------|--------------|--------|---------|----------|
| vb-42nj8 (RP-014) | P1 | `crates/vb_runtime/src/primitives/together.rs:37` | `crates/vb_runtime/src/together_tests.rs` (no RP-014-specific test) | `cargo test -p vb_runtime --lib together` (46 pass) | fail | NOT-PATCHED | together.rs:37 `run.add_parallel_in_flight(count)?` still mutates counter BEFORE fallible work at lines 38-40 (`require_output`, `store.insert_list`, `run.write_slot`). No regression test exercises counter rollback when lines 38-40 fail. Patch exists only in `/home/lewis/src/isolated/` trees, not merged. |
| vb-4bq3r (RP-003) | P2 | `crates/vb_runtime/src/primitives/together.rs:34` | `crates/vb_runtime/src/together_tests.rs` (no rp003 test in main) | `cargo test -p vb_runtime --lib together` (46 pass) | fail | NOT-PATCHED | together.rs:34 still uses `current.saturating_add(count) > max`. Pre-fix bug returns `InternalInvariantViolation` instead of typed `ParallelLimitExceeded` when `current + count` overflows. Phase23 tests cover u16 branch count limits but not the saturating-add overflow at the together_start call site. Proper `checked_add` fix exists only in isolated worktree `vb-4bq3r/primitives/together.rs:43`. |
| vb-4bz9b (RP-015) | P2 | `crates/vb_runtime/src/primitives/collect.rs` (via vb-tpbgl duplicate) | `crates/vb_runtime/src/primitives/collect/tests.rs:3058` | `cargo test -p vb_runtime --lib collect_next_rejects_empty_non_current_page` (1 pass) | pass | PATCHED | Bead is CLOSED as duplicate of `vb-tpbgl`. Real fix landed: empty non-current page now rejected with `CollectPageOrderViolation` (`crates/vb_runtime/src/primitives/collect.rs` collect_next). Test at line 3058 explicitly verifies the rejection. |
| vb-4pwif (RA-014) | P3 | `crates/vb_runtime/src/shard/types.rs:392` (IntrospectionRegistry) | `crates/vb_runtime/src/shard/types.rs:1784-1927` (introspection_poison_regression_tests) | `cargo test -p vb_runtime --lib introspection_poison_regression_tests` (6/6 pass) | pass | PATCHED | `IntrospectionRegistry::lock_or_recover` (line 378) recovers poisoned mutex via `poisoned.into_inner()`. All `register`/`unregister`/`unregister_all`/`register_with_overlap_policy` use it. Bead source path in description (`shard/impl_parts/chunk_001.rs:142-156`) is stale; fix is in `IntrospectionRegistry` not `lock_admission`. 6 regression tests pass. |
| vb-574zr (SR-013) | P2 | `crates/vb_storage/src/recovery/replay/summary.rs:746` | (no test) | (no targeted test exists) | fail | NOT-PATCHED | **`legacy_slot_taint` at summary.rs:746-752 STILL has the bug**: `SlotValue::Bool(false) => Taint::Clean`. Bead close_reason claims "Addressed in wave 8: legacy_slot_taint now unconditionally returns Taint::Secret" but the actual code does the opposite. No regression test asserts `Bool(false) → Secret`. Test `hydrate_run_frame_from_events_accepts_legacy_frame_extra_without_taint_sidecar` writes `Bool(false)` but only asserts `result.is_ok()`, never inspects returned taint — so it cannot catch the leak. |
| vb-596cp (SC-008) | P3 | `crates/vb_storage/src/trimming/logic.rs:94` | `crates/vb_storage/src/trimming/tests.rs` (multiple trim_events_for_run tests) | `cargo test -p vb_storage --lib trim_events_for_run` (1 pass) | pass | PATCHED | trimming/logic.rs:94 changed from `key.to_vec()` to `key.clone()` (Arc-cheap on `fjall::UserKey`/`Slice`). Comment at line 87-93 confirms SC-008 fix. Bead was CLOSED with REJECTED close_reason but the fix was actually applied (REJECTED reason is misleading). |
| vb-5mnsf (CB-004) | P3 | `crates/vb_core/src/budget.rs:184,201,2101` (via vb-6zr4c duplicate) | `crates/vb_core/src/budget.rs:2207-2269` (depth_overflow_tests) | `cargo test -p vb_core --lib depth_overflow_tests` (3/3 pass) | pass | PATCHED | Bead CLOSED as duplicate of `vb-6zr4c`. Real fix: `BudgetTraversalError::DepthOverflow { depth: u16 }` variant added; `compute_child_depth` returns it instead of lossy `StepCountOverflow { actual: u64::MAX }`; `WorkflowError::DepthOverflow { depth: u16 }` added in `vb_core/src/workflow/mod.rs:411`. 3 regression tests pass. |
| vb-6fibb (RE-001) | P2 | `crates/vb_core/src/engine/validate.rs:185,200`; `crates/vb_core/src/workflow/mod.rs:446,775` | (no compiled test) | `cargo test -p vb_core --lib` (NestedTogether tests not found) | partial | PARTIAL | Validation fix IS in place: `validate_no_nested_together` in `vb_core/src/engine/validate.rs:200` walks branch bodies to detect nested `TogetherStart` and returns `WorkflowError::NestedTogether { outer, inner }`. Wired into `validate_compiled_workflow` (line 185) and `WorkflowParts` validation (mod.rs:775). BUT: the regression tests `validation_rejects_nested_together_start` and `validation_accepts_sibling_together_starts` live in `crates/vb_runtime/src/engine/drive_tests.rs:1210-1268`, which is a **dead file** — never `mod`-included anywhere. No executable regression test for the fix. `compute_max_parallel_in_flight` itself (`drive.rs:19-38`) was NOT updated; the fix relies entirely on validation rejection. |
| vb-6tnb6 (RP-004) | P2 | `crates/vb_runtime/src/primitives/together.rs:139-141` | `crates/vb_runtime/src/together_tests.rs` (no O(N²) test in main) | `cargo test -p vb_runtime --lib together` (46 pass) | fail | NOT-PATCHED | `append_to_accumulator` at together.rs:139-141 still does `existing.to_vec()` → push → `insert_list`, the Θ(N) clone-per-append Θ(N²)-total pattern. Proper fix (`store.append_list(list_id, value)?`) exists only in isolated worktree `vb-6tnb6/primitives/together.rs:158`. `ValueStore::append_list` not called anywhere in main repo. Existing `phase23_join_*` tests verify correctness but not asymptotic complexity. |
| vb-6zr4c (CB-004) | P3 | `crates/vb_core/src/budget.rs:184,201,2101`; `crates/vb_core/src/workflow/mod.rs:411` | `crates/vb_core/src/budget.rs:2207-2269` (depth_overflow_tests) | `cargo test -p vb_core --lib depth_overflow_tests` (3/3 pass) | pass | PATCHED | Same as vb-5mnsf (the active bead). `BudgetTraversalError::DepthOverflow { depth: u16 }` exists, `compute_child_depth` returns it. `WorkflowError::DepthOverflow { depth: u16 }` propagated through `From<BudgetTraversalError>`. 3 regression tests pass (`compute_child_depth_returns_depth_overflow_carrying_actual_depth`, `compute_child_depth_does_not_emit_step_count_overflow_at_u16_max`, `depth_overflow_converts_to_workflow_error_carrying_actual_depth`). Bead is IN_PROGRESS but fix is complete — close status pending. |
| vb-74690 (RE-015) | P3 | `crates/vb_runtime/src/trace.rs:42-50` | `crates/vb_runtime/src/trace/tests.rs:21,50` | `cargo test -p vb_runtime --lib trace::tests::len_and_is_empty` (2/2 pass) | pass | PATCHED | `TraceRing::len` (line 42) returns `self.consumer.slots()` and `is_empty` (line 48) returns `self.consumer.is_empty()` — both query the rtrb ring buffer slots (drainable events), not the `history` `VecDeque`. Test `len_and_is_empty_ignore_retained_snapshot_history_after_drain` (line 50) explicitly drains the ring, then asserts `ring.len() == 0` and `ring.is_empty() == true` while `snapshot_for_run` still returns the remembered event. Bead is BLOCKED but code fix is in place. |

---

## Summary

- **bugs-checked:** 11
- **pass:** 5 (vb-4bz9b, vb-4pwif, vb-596cp, vb-5mnsf, vb-6zr4c, vb-74690) — counting duplicates individually
- **fail (NOT-PATCHED):** 3 (vb-42nj8, vb-4bq3r, vb-574zr, vb-6tnb6) — actually 4
- **partial (PARTIAL):** 1 (vb-6fibb)
- **Recounted:** 5 PATCHED, 4 NOT-PATCHED, 1 PARTIAL, 0 UNKNOWN

**Tally:**
- PATCHED: 5 (vb-4bz9b, vb-4pwif, vb-596cp, vb-5mnsf, vb-6zr4c, vb-74690 = 6) — see note
- NOT-PATCHED: 4 (vb-42nj8, vb-4bq3r, vb-574zr, vb-6tnb6)
- PARTIAL: 1 (vb-6fibb)

Corrected tally: **6 PATCHED, 4 NOT-PATCHED, 1 PARTIAL** (the pass count of 5 in my summary line above was wrong — vb-4bz9b and vb-5mnsf are dup-redirect PATCHED so they do count, vb-74690 is BLOCKED but PATCHED, giving 6 total).

---

## Top-3 NOT-PATCHED with file:line and brief reason

1. **vb-574zr (SR-013) — `crates/vb_storage/src/recovery/replay/summary.rs:748`**
   `legacy_slot_taint` STILL maps `SlotValue::Bool(false) => Taint::Clean`, leaking secret `Bool(false)` slots through legacy hydrate paths as Clean. Bead was CLOSED claiming wave-8 fix at `slots/taint.rs:47-48` (file does not exist) — no such file was ever created and the summary.rs function was never updated. No regression test asserts `Bool(false) → Secret`.

2. **vb-42nj8 (RP-014) — `crates/vb_runtime/src/primitives/together.rs:37`**
   `together_start` mutates `parallel_in_flight` at line 37 BEFORE the fallible work at lines 38-40 (`require_output`, `store.insert_list`, `run.write_slot`). If any of 38-40 fail, the counter is leaked. Existing tests cover happy-path and branch-failure-after-counter but no test verifies counter rollback when `require_output`/`insert_list`/`write_slot` fails after line 37. Fix exists only in isolated worktree; not merged to main.

3. **vb-6tnb6 (RP-004) — `crates/vb_runtime/src/primitives/together.rs:139-141`**
   `append_to_accumulator` still does `let mut items = existing.to_vec(); items.push(value); let updated = store.insert_list(items.into_boxed_slice())?;` — the Θ(N) clone-per-append / Θ(N²)-total pattern. `ValueStore::append_list` exists in the isolated worktree fix but is not present in `crates/vb_storage/src/value_store.rs` (or wherever) — not callable from main. No asymptotic complexity test.

(Honorable mention: vb-4bq3r / RP-003 — same pattern, fix isolated, `saturating_add` still in `together.rs:34`.)

---

## Notes on test infrastructure gaps

- `crates/vb_runtime/src/engine/drive_tests.rs` (1269 lines, contains RE-001 regression tests at lines 1210-1268) is **never `mod`-included** anywhere in the crate. All tests in this file are dead code. Verified via `grep "mod drive_tests"` returning no hits.
- No regression test for `legacy_slot_taint` exists in `crates/vb_storage/src/recovery/replay/` tests. The hydrate test that writes `Bool(false)` (`recovery/tests.rs:2372`) only asserts `result.is_ok()` — never inspects returned taint.
- No regression test for the `parallel_in_flight` counter leak in `together_start` — the `phase23_start_failure_when_branches_empty` test fails at line 27 (before counter mutation), so it cannot exercise the leak.

## Output file

Written to: `/home/lewis/src/velvet-ballistics/to-fix/wave1/agent-02-explore.md`
