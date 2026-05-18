# Test Plan Review: LETHAL-4 `Runtime::tick_shard` & `ShardDirective`

## Mode 1: Plan Inquisition
## VERDICT: REJECTED

---

## Axis 1 — Contract Parity: **PASS** (with caveats)

All 10 behaviors have BDD scenarios. Error variant assertions are exact (`Err(RuntimeError::ShardNotFound { shard: ShardId::new(5) })`, `Err(RuntimeError::MigrateSelf)`). No `is_ok()`/`is_err()` as sole assertions.

**Caveat (not LETHAL — Open Questions acknowledge it):** `RuntimeError::ShardNotFound` and `RuntimeError::MigrateSelf` do **not** exist in `crates/vb_runtime/src/error/mod.rs`. The current `RuntimeError` enum has `RunNotFound`, `ShutdownInProgress`, etc. — these two variants must be added. Similarly, `ShardId` does not exist in `crates/vb_core/src/ids/mod.rs`. The plan correctly lists these as new types in "Files Under Test" and "Open Questions #1 and #6" — but they must be resolved before implementation.

---

## Axis 2 — Assertion Sharpness: **FAIL**

### MAJOR — `runtime_tick_shard_continue_increments_step_counter` (line 78)
```
Then: `runtime.counters_snapshot().steps_executed` is ≥ 1
```
**Problem**: `≥ 1` is not an exact value. The scenario uses `action_then_finish_workflow` (Do action + Finish) with **two** `tick_shard(Continue)` calls. If the workflow has 2 steps, the counter should be exactly `2`. If the implementation returns `1`, the test passes incorrectly. If it returns `3`, the test also passes. This is not a sharp assertion.

**Required fix**: Assert exact step count (`== 2`) or provide a deterministic workflow where exact count is knowable.

### MAJOR — `runtime_tick_shard_continue_returns_ok_with_empty_queue` (line 68)
```
Then: Returns `Ok(true)` (shard is alive); command queue remains empty
```
**Problem**: Does not assert `runs_submitted == 0`. The counter could increment (erroneously) on an empty shard and this test would not catch it. The observable effect is that queue remains empty, but the counter invariant is unstated.

**Required fix**: Add `runtime.counters_snapshot().runs_submitted == 0` to the Then clause.

### MAJOR — `runtime_tick_shard_suspend_idempotent_on_idle_shard` (line 112)
```
Then: Returns `Ok(true)`; counters remain at initial values
```
**Problem**: "initial values" is vague. Does not explicitly state that `runs_submitted == 0` and `steps_executed == 0`. A reader cannot verify counter invariants without knowing what "initial values" means.

**Required fix**: State the explicit counter values.

---

## Axis 3 — Trophy Allocation: **PASS**

- 6 unit / 14 integration / 2 e2e is a reasonable ratio for a state-machine API.
- 3 proptest invariants defined: `runtime_tick_shard_with_random_directive`, `migrate_preserves_run_identity`, `shutdown_is_stable_idempotent`.
- 1 fuzz target for migrate target corner cases.
- 2 Kani harnesses for critical state-machine invariants.
- `tick_shard` returns `Result<bool, RuntimeError>` — non-trivial input space; proptest is appropriate.

No LETHAL violations here.

---

## Axis 4 — Boundary Completeness: **MINOR**

All major boundaries are named in the combinatorial coverage matrix:

| Boundary | Status |
|----------|--------|
| Min (0 commands) | ✅ Covered |
| Max (N commands) | ✅ Covered |
| Empty queue | ✅ Covered |
| OOB index | ✅ Covered |
| Self-migrate | ✅ Covered |
| Already-dead shard | ✅ Covered |

**MINOR gaps (not MAJOR — ≤2 per function):**
- **Shutdown with queued commands** — explicitly covered by `runtime_tick_shard_shutdown_drains_remaining_actions` (integration), but the unit/combinatorial matrix marks "1 Submit + Finish" as integration-only. A unit-level shutdown-with-commands boundary test would close this gap.
- **Negative shard index** — not explicitly named. If `ShardId` uses `u32` internally, negative indices are impossible by construction. If `isize` is used, this is a missing boundary.
- **`Continue` on source shard after `Migrate`** — Open Question #5 documents this as unknown. Not a missing boundary — this is an acknowledged open question.

---

## Axis 5 — Mutation Survivability: **MAJOR**

Apply the four mental mutations to each scenario:

| Mutation | Catching test | Status |
|----------|--------------|--------|
| Continue processes when should skip | `runtime_tick_shard_suspend_skips_all_command_processing` | ✅ |
| Suspend doesn't preserve queue depth | `runtime_tick_shard_suspend_preserves_command_queue_depth` | ✅ |
| Migrate enqueues to wrong shard | `runtime_tick_shard_migrate_transfers_actions_to_target_shard` | ✅ |
| Migrate allows self-migrate | `runtime_tick_shard_migrate_rejects_self_migrate` | ✅ |
| Shutdown doesn't drain | `runtime_tick_shard_shutdown_drains_remaining_actions` | ✅ |
| Shutdown returns `Ok(true)` on already-dead shard | `runtime_tick_shard_shutdown_returns_false_on_dead_shard` | ⚠️ **GAP** |
| Invalid index doesn't return ShardNotFound | `runtime_tick_shard_invalid_shard_index_returns_error` | ✅ |
| OOB Migrate target doesn't return ShardNotFound | `runtime_tick_shard_migrate_rejects_invalid_target` | ✅ |
| drain_for_shutdown terminates early | `runtime_tick_shard_shutdown_drains_remaining_actions` | ✅ |
| drain_for_shutdown panics on overflow | `runtime_tick_shard_shutdown_idempotent` | ✅ |

**CRITICAL GAP**: The test `runtime_tick_shard_shutdown_returns_false_on_dead_shard` verifies that calling `Continue` on a dead shard returns `Ok(false)`. However, if the implementation **incorrectly** returns `Ok(true)` (the opposite of correct), **no test fails**. The test only proves the correct behavior; it does not falsify the incorrect behavior.

**Mutation**: `Shutdown` returns `Ok(true)` instead of `Ok(false)` → not caught.
**Required**: Add `runtime_tick_shard_shutdown_returns_false_on_dead_shard` with assertion that `Ok(true)` would be WRONG, or add a separate negative test that `Continue` on dead shard does NOT return `Ok(true)`.

---

## Axis 6 — Evidence Plan Audit: **PASS**

- All scenarios have explicit `Given` blocks stating preconditions.
- Proptest strategies are bounded and reproducible (`prop_oneof`, `prop_filter`, `prop_n_sized(5)`).
- Fuzz corpus has explicit seed values.
- Kani harnesses specify explicit bounds (max 16 shards, 4-variant enum).
- Helper functions (`suspended_workflow`, `action_then_finish_workflow`) are named but not defined in the plan. Since this is Mode 1 (no implementation), the reproducibility of these helpers cannot be verified yet — flag as OPEN.

---

## Summary of Findings

### LETHAL FINDINGS (0)
None. The plan has no bare `is_ok()`/`is_err()` assertions and no pure functions with zero proptest coverage.

### MAJOR FINDINGS (3)
1. **`runtime_tick_shard_continue_increments_step_counter` (line 78)**: `≥ 1` is not an exact assertion — must assert `== 2` given exactly-2-step workflow with 2 tick calls.
2. **`runtime_tick_shard_continue_returns_ok_with_empty_queue` (line 68)**: Missing `runs_submitted == 0` counter invariant — queue-empty does not imply counter unchanged.
3. **Mutation gap — Continue on dead shard returning `Ok(true)`**: Test only proves `Ok(false)` is returned; does not falsify `Ok(true)`. Requires negative assertion or paired test.

### MINOR FINDINGS (2)
1. **`runtime_tick_shard_suspend_idempotent_on_idle_shard` (line 112)**: "counters remain at initial values" is vague — state `runs_submitted == 0 && steps_executed == 0`.
2. **ShardDirective enum unit test**: The "ShardDirective Enum" section of the combinatorial matrix shows ✅ for serialization roundtrip and equality, but no specific unit test is named for these. Integration scenarios cover it but a dedicated unit test for `ShardDirective::{Continue, Suspend, Migrate, Shutdown}` equality and serialization would close the loop.

### OPEN ITEMS (not LETHAL — pre-implementation)
1. `RuntimeError::ShardNotFound` variant must be added to `vb_runtime/src/error/mod.rs`.
2. `RuntimeError::MigrateSelf` variant must be added to `vb_runtime/src/error/mod.rs`.
3. `ShardId` type must be created in `vb_core/src/ids/mod.rs` (or decide `usize` is acceptable).
4. Open Questions #1–#6 must be resolved before implementation begins.

---

## MANDATE

Before resubmission, the following must be resolved:

1. **[MAJOR]** Change `runtime_tick_shard_continue_increments_step_counter` Then clause from `≥ 1` to exact expected value `== 2`, or use a workflow where exact count is deterministic and stated.
2. **[MAJOR]** Add `runs_submitted == 0` assertion to `runtime_tick_shard_continue_returns_ok_with_empty_queue` Then clause.
3. **[MAJOR]** Add explicit negative mutation test or modify `runtime_tick_shard_shutdown_returns_false_on_dead_shard` to assert that `Ok(true)` would be incorrect (e.g., `assert_ne!(result, Ok(true))`).
4. **[MINOR]** Replace "counters remain at initial values" with explicit `runs_submitted == 0 && steps_executed == 0` in `runtime_tick_shard_suspend_idempotent_on_idle_shard`.
5. **[MINOR]** Add named unit test for `ShardDirective` enum equality and serialization (e.g., `fn shard_directive_equality_and_debug`).
6. **[OPEN]** Resolve Open Questions #1–#6 (ShardId type, Migrate semantics, backpressure, Suspend+active run, Continue after Migrate, error naming) before implementation.

After fixes: resubmit for full re-review from Axis 1.
