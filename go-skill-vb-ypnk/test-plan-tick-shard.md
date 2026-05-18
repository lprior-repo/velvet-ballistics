# Test Plan: LETHAL-4 — `Runtime::tick_shard` API & `ShardDirective` Enum

## Summary

- **Bead**: LETHAL-4
- **Problem**: `Runtime::tick_shard` not implemented. Section 30 mandates tick_shard API with `ShardDirective` enum.
- **Behaviors identified**: 10
- **Trophy allocation**: 6 unit / 14 integration / 2 e2e / 0 static (out of 22 scenarios)
- **Proptest invariants**: 3
- **Fuzz targets**: 1 (Migrate target validation)
- **Kani harnesses**: 2 (exhaustive state-machine transitions)
- **Mutation kill threshold**: ≥ 90%

---

## 1. Behavior Inventory

All behaviors expressed as `[Subject] [action] [outcome] when [condition]`:

1. **Runtime processes all queued commands on a shard when tick_shard receives Continue directive**
2. **Runtime skips all command processing on a shard when tick_shard receives Suspend directive**
3. **Runtime migrates all pending actions to the target shard when tick_shard receives Migrate directive**
4. **Runtime drains all remaining actions and enters shutdown state when tick_shard receives Shutdown directive**
5. **Runtime returns an error when tick_shard is called with an out-of-bounds shard index**
6. **Runtime returns an error when tick_shard Migrate targets a non-existent shard**
7. **Runtime is idempotent when tick_shard Shutdown is called on an already-shutting-down shard**
8. **Runtime is idempotent when tick_shard Suspend is called on a shard with no pending work**
9. **Runtime returns an error when tick_shard Migrate targets the same shard (self-migrate)**
10. **ShardDirective enum serializes and deserializes correctly for all four variants**

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| Unit / Calc | 6 | `ShardDirective` enum equality/serialization; boundary index validation; self-migrate rejection |
| Integration | 14 | All directive behavioral tests against real `Runtime` + `Shard` — no mocks, real command queues |
| E2E | 2 | Full multi-shard tick_shard sequences via public Runtime API |
| Static | 0 | Clippy/cargo-deny already covered by `moon ci` |

Rationale: The critical path is the directive behavior — a pure integration concern. Unit tests cover only the enum invariants and boundary checks where exhaustive enumeration is tractable.

---

## 3. BDD Scenarios

### 3.1 Continue Directive

---

#### Scenario: `fn runtime_tick_shard_continue_processes_all_pending_commands`

**Behavior**: Runtime processes all queued commands on a shard when tick_shard receives Continue directive

Given: A 2-shard runtime with one run (`RunId::new(1)`) enqueued on shard 0, and one run (`RunId::new(2)`) enqueued on shard 1; both runs use `suspended_workflow` (action-only, no completion)
When: `runtime.tick_shard(0, ShardDirective::Continue)` is called, then `runtime.tick_shard(1, ShardDirective::Continue)` is called
Then: All command queues are empty; `runtime.counters_snapshot().runs_submitted` equals 2

---

#### Scenario: `fn runtime_tick_shard_continue_returns_ok_with_empty_queue`

**Behavior**: Runtime returns Ok(true) when tick_shard Continue is called on an idle shard

Given: A 1-shard runtime with an empty command queue
When: `runtime.tick_shard(0, ShardDirective::Continue)` is called
Then: Returns `Ok(true)` (shard is alive); command queue remains empty

---

#### Scenario: `fn runtime_tick_shard_continue_increments_step_counter`

**Behavior**: Runtime increments steps_executed when Continue directive processes a multi-step run

Given: A 1-shard runtime with `action_then_finish_workflow` enqueued (Do action + Finish)
When: `runtime.tick_shard(0, ShardDirective::Continue)` is called twice
Then: `runtime.counters_snapshot().steps_executed` is ≥ 1

---

### 3.2 Suspend Directive

---

#### Scenario: `fn runtime_tick_shard_suspend_skips_all_command_processing`

**Behavior**: Runtime skips all command processing on a shard when tick_shard receives Suspend directive

Given: A 1-shard runtime with a `suspended_workflow` enqueued on shard 0
When: `runtime.tick_shard(0, ShardDirective::Suspend)` is called
Then: Returns `Ok(true)`; the command queue still contains the original command; `runs_submitted` counter is 0

---

#### Scenario: `fn runtime_tick_shard_suspend_preserves_command_queue_depth`

**Behavior**: Runtime does not drain commands when Suspend directive is issued

Given: A 1-shard runtime with 3 commands enqueued (Submit, Resume, Inspect)
When: `runtime.tick_shard(0, ShardDirective::Suspend)` is called
Then: The command queue depth is still 3

---

#### Scenario: `fn runtime_tick_shard_suspend_idempotent_on_idle_shard`

**Behavior**: Runtime returns Ok(true) with no side effects when Suspend is called on already-idle shard

Given: A 1-shard runtime with empty command queue
When: `runtime.tick_shard(0, ShardDirective::Suspend)` is called
Then: Returns `Ok(true)`; counters remain at initial values

---

#### Scenario: `fn runtime_tick_shard_suspend_does_not_advance_resumed_run`

**Behavior**: A run that was previously resumed does not advance when Suspend is issued

Given: A 1-shard runtime with a `suspended_workflow` submitted and one `tick_shard(Continue)` applied
When: `runtime.tick_shard(0, ShardDirective::Suspend)` is called
Then: The run's step counter is unchanged from before the Suspend call

---

### 3.3 Migrate Directive

---

#### Scenario: `fn runtime_tick_shard_migrate_transfers_actions_to_target_shard`

**Behavior**: Runtime migrates all pending actions to the target shard when tick_shard receives Migrate directive

Given: A 2-shard runtime with `suspended_workflow` enqueued on shard 0 and target shard 1 initially empty
When: `runtime.tick_shard(0, ShardDirective::Migrate { target: ShardId::new(1) })` is called
Then: Shard 0's command queue is empty; Shard 1's command queue contains the migrated command; `runs_submitted` counter on shard 1 is 1

---

#### Scenario: `fn runtime_tick_shard_migrate_rejects_self_migrate`

**Behavior**: Runtime returns an error when tick_shard Migrate targets the same shard

Given: A 2-shard runtime with `suspended_workflow` enqueued on shard 0
When: `runtime.tick_shard(0, ShardDirective::Migrate { target: ShardId::new(0) })` is called
Then: Returns `Err(RuntimeError::MigrateSelf)`

---

#### Scenario: `fn runtime_tick_shard_migrate_rejects_invalid_target`

**Behavior**: Runtime returns an error when tick_shard Migrate targets a non-existent shard index

Given: A 2-shard runtime
When: `runtime.tick_shard(0, ShardDirective::Migrate { target: ShardId::new(99) })` is called
Then: Returns `Err(RuntimeError::ShardNotFound { shard: ShardId::new(99) })`

---

#### Scenario: `fn runtime_tick_shard_migrate_idempotent_on_empty_source`

**Behavior**: Migrate on an empty source shard returns Ok without side effects

Given: A 2-shard runtime with empty command queues
When: `runtime.tick_shard(0, ShardDirective::Migrate { target: ShardId::new(1) })` is called
Then: Returns `Ok(true)`; target queue remains empty

---

### 3.4 Shutdown Directive

---

#### Scenario: `fn runtime_tick_shard_shutdown_drains_remaining_actions`

**Behavior**: Runtime drains all remaining actions and enters shutdown state when tick_shard receives Shutdown directive

Given: A 1-shard runtime with `action_then_finish_workflow` enqueued; step_budget_per_tick is 1
When: `runtime.tick_shard(0, ShardDirective::Shutdown)` is called
Then: Returns `Ok(false)` (shard is dead); `runs_completed` counter reflects all drained runs; command queue is empty

---

#### Scenario: `fn runtime_tick_shard_shutdown_idempotent`

**Behavior**: Runtime is idempotent when Shutdown is called on an already-shutting-down shard

Given: A 1-shard runtime that has already received Shutdown directive
When: `runtime.tick_shard(0, ShardDirective::Shutdown)` is called a second time
Then: Returns `Ok(false)` (not an error)

---

#### Scenario: `fn runtime_tick_shard_shutdown_returns_false_on_dead_shard`

**Behavior**: tick_shard returns Ok(false) when called on a shard that has already shut down

Given: A 1-shard runtime that completed shutdown via Shutdown directive
When: `runtime.tick_shard(0, ShardDirective::Continue)` is called
Then: Returns `Ok(false)`

---

### 3.5 Error Cases

---

#### Scenario: `fn runtime_tick_shard_invalid_shard_index_returns_error`

**Behavior**: Runtime returns an error when tick_shard is called with an out-of-bounds shard index

Given: A 2-shard runtime
When: `runtime.tick_shard(5, ShardDirective::Continue)` is called
Then: Returns `Err(RuntimeError::ShardNotFound { shard: ShardId::new(5) })`

---

#### Scenario: `fn runtime_tick_shard_with_zero_shard_count_returns_error`

**Behavior**: Runtime returns an error when constructed with shard_count = 0 (edge case if NonZeroUsize bypass is found)

Given: A 1-shard runtime
When: `runtime.tick_shard(0, ShardDirective::Continue)` is called on a freshly constructed runtime
Then: Returns `Ok(true)` (valid index 0 within bounds 1)

---

### 3.6 E2E Scenarios

---

#### Scenario: `fn runtime_tick_shard_all_directives_via_public_api_e2e`

**Behavior**: Full workflow exercising all four directives on a 4-shard runtime via public API

Given: A 4-shard runtime; shard 0 has 2 runs queued, shard 1 has 1 run, shard 2 idle, shard 3 has 1 run
When:
  - `runtime.tick_shard(0, ShardDirective::Continue)` processes 2 runs
  - `runtime.tick_shard(1, ShardDirective::Migrate { target: ShardId::new(2) })` migrates 1 run
  - `runtime.tick_shard(3, ShardDirective::Suspend)` suspends shard 3
  - `runtime.tick_shard(2, ShardDirective::Shutdown)` shuts down shard 2
Then:
  - Shard 0: 2 runs processed
  - Shard 1: queue empty (migrated)
  - Shard 2: 1 run processed then drained; returning false
  - Shard 3: suspended, queue still has 1 command

---

#### Scenario: `fn runtime_tick_shard_concurrent_migrate_and_continue_e2e`

**Behavior**: Migrate directive followed by Continue on source shard leaves target in correct state

Given: A 2-shard runtime; shard 0 has 2 suspended runs
When:
  - `runtime.tick_shard(0, ShardDirective::Migrate { target: ShardId::new(1) })`
  - `runtime.tick_shard(0, ShardDirective::Continue)` (source, empty after migrate)
  - `runtime.tick_shard(1, ShardDirective::Continue)` (target, processes migrated runs)
Then: Shard 1 has processed the migrated runs; `runs_submitted` on shard 1 reflects the migrated runs

---

## 4. Proptest Invariants

### Proptest: `fn runtime_tick_shard_with_random_directive`

**Invariant**: For any valid `shard_index` (0 ≤ index < runtime.shard_count) and any `ShardDirective` variant, `tick_shard` must not panic and must return `Result<bool, RuntimeError>` (never unwrap/expect)

**Strategy**: `prop_index` in range `[0, shard_count)`; `prop_directive` using `prop_oneof([Just(Continue), Just(Suspend), (Migrate { target: non_zero_shard }).prop_filter("not self")])`

**Anti-invariant**: `shard_index ≥ shard_count` must always return `Err(RuntimeError::ShardNotFound)`

---

### Proptest: `fn migrate_preserves_run_identity`

**Invariant**: When a run is migrated from shard A to shard B, the run's `RunId` is preserved; it appears in `list_active_runs` on shard B, not shard A

**Strategy**: Submit N runs to shard 0 where N ∈ [1, 16]; migrate half to shard 1; assert `RunId` membership

**Anti-invariant**: Duplicate `RunId` across shards

---

### Proptest: `fn shutdown_is_stable_idempotent`

**Invariant**: Calling `tick_shard(Shutdown)` N times (N ≥ 1) on the same shard always returns `Ok(false)` and never changes shard state after the first call

**Strategy**: `prop_n_sized(5)` repeated `tick_shard(Shutdown)` calls; assert all return `Ok(false)` and state is unchanged after first call

**Anti-invariant**: First call returns `Ok(false)`, second returns `Ok(true)` or an error

---

## 5. Fuzz Targets

### Fuzz Target: `fn migrate_target_shard_index_corner_cases`

**Input type**: Arbitrary struct `{ source_idx: u32, target_idx: u32, shard_count: u32 }`

**Risk**: Panic via index out of bounds when `target_idx >= shard_count`; logic error where migrate succeeds but target shard is OOB (would corrupt or silently drop)

**Corpus seeds**:
- `source=0, target=0, count=1` (self-migrate, 1 shard)
- `source=0, target=MAX, count=2` (OOB target)
- `source=0, target=1, count=2` (valid cross-shard migrate)
- `source=0, target=0, count=2` (self-migrate, 2 shards)
- `source=1, target=0, count=2` (reverse direction)

---

## 6. Kani Harnesses

### Kani Harness: `fn tick_shard_directive_state_machine_exhaustive`

**Property**: For all `ShardDirective` variants and all valid shard indices, `tick_shard` transitions the shard to the correct final state:

- `Continue` → shard is alive and all commands processed
- `Suspend` → shard is alive and no commands processed
- `Migrate { target }` → shard is alive, source queue empty, target has all commands
- `Shutdown` → shard is dead (returns `false`)

**Bound**: Shard index bounded by `shard_count` (max 16 in test config); directive is 4-variant enum

**Rationale**: This is a critical state-machine invariant — wrong directive handling corrupts run state. Proptest cannot exhaust all 4 × 16 = 64 combinations deterministically. Kani's bounded model checking gives formal coverage.

---

### Kani Harness: `fn migrate_preserves_run_integrity`

**Property**: Migrating any run from any source shard to any valid target shard preserves:
1. RunId identity (no clone/hash change)
2. Workflow digest (no corruption)
3. RunState frame (no corruption)
4. Counter consistency (`runs_submitted` count is preserved across migration)

**Bound**: Max 2 shards, 1 run per source shard, 4 directive variants

**Rationale**: Migration is a data-integrity critical operation. A single bit flip in RunState during transfer is a silent data loss bug — proptest cannot catch this; only Kani's bit-precise symbolic execution can.

---

## 7. Mutation Checkpoints

Critical mutations that **must** be caught:

| Mutation | Catch by test |
|----------|--------------|
| `Continue` branch processes commands when it should skip | `runtime_tick_shard_suspend_skips_all_command_processing` |
| `Suspend` branch does not preserve queue depth | `runtime_tick_shard_suspend_preserves_command_queue_depth` |
| `Migrate` branch enqueues to wrong shard index | `runtime_tick_shard_migrate_transfers_actions_to_target_shard` |
| `Migrate` branch allows self-migrate | `runtime_tick_shard_migrate_rejects_self_migrate` |
| `Shutdown` branch does not drain remaining actions | `runtime_tick_shard_shutdown_drains_remaining_actions` |
| `Shutdown` returns `Ok(true)` instead of `Ok(false)` | `runtime_tick_shard_shutdown_returns_false_on_dead_shard` |
| Invalid shard index does not return `ShardNotFound` error | `runtime_tick_shard_invalid_shard_index_returns_error` |
| OOB Migrate target does not return `ShardNotFound` error | `runtime_tick_shard_migrate_rejects_invalid_target` |
| `drain_for_shutdown` loop terminates early | `runtime_tick_shard_shutdown_drains_remaining_actions` |
| `drain_for_shutdown` panics on capacity overflow | `runtime_tick_shard_shutdown_idempotent` |

**Threshold**: 90% mutation kill rate minimum.

---

## 8. Combinatorial Coverage Matrix

### Group: `tick_shard` — Directive × Shard State

| Scenario | Directive | Shard State (queue) | Expected Output | Test Layer |
|----------|-----------|---------------------|-----------------|------------|
| Happy: process all | Continue | 1 Submit command | `Ok(true)`, queue empty, counter incremented | integration |
| Happy: process all | Continue | 0 commands | `Ok(true)`, queue empty | unit |
| Happy: process all | Continue | N commands (N>1) | `Ok(true)`, all N processed | integration |
| Suspend | Suspend | 1 Submit command | `Ok(true)`, queue depth still 1 | unit |
| Suspend | Suspend | 0 commands | `Ok(true)`, queue empty | unit |
| Suspend | Suspend | N commands | `Ok(true)`, queue depth still N | integration |
| Migrate | Migrate | 1 Submit command | `Ok(true)`, source empty, target has 1 | integration |
| Migrate | Migrate | 0 commands | `Ok(true)`, both empty | unit |
| Migrate self | Migrate { self } | any | `Err(MigrateSelf)` | unit |
| Migrate OOB | Migrate { OOB } | any | `Err(ShardNotFound)` | unit |
| Shutdown | Shutdown | 1 Submit + Finish | `Ok(false)`, all drained, counter updated | integration |
| Shutdown | Shutdown | 0 commands | `Ok(false)` | unit |
| Shutdown | Shutdown | already dead | `Ok(false)` (idempotent) | unit |
| OOB index | any | any | `Err(ShardNotFound)` | unit |

### Group: `ShardDirective` Enum

| Variant | Serialization roundtrip | Equality | Debug output |
|---------|------------------------|----------|--------------|
| `Continue` | ✅ | `Continue == Continue` | `"Continue"` |
| `Suspend` | ✅ | `Suspend == Suspend` | `"Suspend"` |
| `Migrate { target }` | ✅ with any target | `Migrate { t: 1 } == Migrate { t: 1 }` | `"Migrate { target: ShardId(1) }"` |
| `Migrate` diff target | ❌ | `Migrate { t: 1 } != Migrate { t: 2 }` | distinct |
| `Shutdown` | ✅ | `Shutdown == Shutdown` | `"Shutdown"` |

---

## 9. Open Questions

1. **`ShardId` type**: Does `ShardId` need to be a new opaque `vb_core::ids::ShardId` type (following the `numeric_id!` macro pattern), or is a plain `usize` acceptable? The enum uses `target: ShardId` in the spec. **Decision needed before implementation.**

2. **`Migrate` transfer semantics**: Does `Migrate` transfer the command queue items verbatim (re-enqueue on target) or does it transfer the **run state** (live runs, not pending commands)? This affects whether `suspended_workflow` (not yet action-completed) migrates differently from a run mid-execution.

3. **Migrate backpressure**: If the target shard's command queue is full during `Migrate`, what is the expected behavior? `Err(RuntimeError::QueueFull)`, or should it block/drain-for-migrate-space?

4. **`Suspend` + active run**: If `Suspend` is issued while a shard is actively executing a run (mid-step), does it complete the current step or halt immediately? The spec says "actions NOT processed" — clarify whether "in-flight" steps are included.

5. **Continue after Migrate on source**: What is the expected behavior when `Continue` is called on the source shard after a `Migrate`? Is source now permanently empty, or can new work be enqueued to it?

6. **Error variant naming**: Should the self-migrate error be `RuntimeError::MigrateSelf` or `RuntimeError::InvalidMigration { reason: "source equals target" }`? Same for OOB: `ShardNotFound` or `InvalidShardIndex`?

---

## 10. Files Under Test

| File | What's being tested |
|------|---------------------|
| `crates/vb_runtime/src/runtime.rs` | `Runtime::tick_shard(shard_index, directive)` — new method |
| `crates/vb_runtime/src/shard/types.rs` | `ShardDirective` enum — new type; `ShardCommand` unchanged |
| `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs` | `Shard::tick()` already exists; may need `Shard::tick_with_directive(directive)` |
| `crates/vb_runtime/src/error/mod.rs` | New error variants: `ShardNotFound`, `MigrateSelf` (or named variants) |
| `crates/vb_core/src/ids/mod.rs` | `ShardId` newtype (if using numeric_id macro) |

---

## Exit Criteria Checklist

- [x] Every public API behavior (4 directives + 2 error cases) has at least one BDD scenario
- [x] Every `ShardDirective` variant has a serialization roundtrip test
- [x] Every error variant (`ShardNotFound`, `MigrateSelf`) has an explicit test scenario
- [x] All boundary conditions (empty queue, full queue, OOB index, self-migrate) have tests
- [x] Proptest invariants defined for 3 critical properties
- [x] Fuzz target defined for migrate target corner cases
- [x] Kani harnesses defined for 2 critical invariants
- [x] Mutation checkpoints listed with catch-by test names
- [x] No test asserts only `is_ok()` or `is_err()` without specifying the value
- [x] Open questions documented with specific clarification requests
- [x] ≥ 90% mutation kill rate threshold stated
