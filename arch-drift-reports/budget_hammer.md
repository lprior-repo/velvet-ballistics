# Architectural Drift Report: `vb_core/src/budget.rs`

## Line Count Violation

| Metric | Value |
|--------|-------|
| **Current lines** | 2716 |
| **Limit** | 300 |
| **Over ratio** | **9.1x** — CATASTROPHIC |
| **Status** | `REFACTOR REQUIRED` |

---

## Budget Functions Requiring Extraction

### Domain Types (8 structs + 3 enums — cohesive)

| Struct/Enum | Lines | Responsibility | Suggested Module |
|---|---|---|---|
| `WholeWorkflowBudget` | 10–166 | Computed budget result from IR walk | `budget/compute.rs` |
| `BudgetTraversalError` | 170–189 | Narrow DFS traversal error (Kani-safe subset) | `budget/traversal.rs` |
| `BoundednessPolicy` | 324–441 | Policy limits + validation | `budget/policy.rs` |
| `BudgetError` | 516–552 | Validation failure taxonomy | `budget/policy.rs` |
| `AggregateResourceBudget` | 554–758 | Per-run resource reservation | `budget/resource.rs` |
| `AggregateResourceCapacity` | 583–604 | Shard capacity ceiling | `budget/resource.rs` |
| `AggregateResourceUsage` | 607–1092 | Usage snapshot + accounting ops | `budget/resource.rs` |
| `AggregateReservation` | 631–635 | Run-bound reservation | `budget/resource.rs` |
| `AggregateBudgetError` | 638–714 | Aggregate accounting failures | `budget/resource.rs` |
| `SmallLinearMetrics` | 234–322 | Fast-path metrics for ≤2-node linear workflows | `budget/compute.rs` |

### Pure Calculation Functions (40+ free functions — BLOAT)

| Function | Lines | Responsibility |
|---|---|---|
| `compute_small_linear_budget` | 191–232 | Fast path for trivial linear workflows |
| `small_linear_domain` | 241–253 | Predicates whether workflow qualifies for fast path |
| `small_linear_node` | 255–267 | Node-kind gate for fast path |
| `small_linear_next` | 269–273 | Next pointer validation |
| `small_linear_metrics` | 276–290 | Metrics accumulation for fast path |
| `small_linear_node_metrics` | 292–312 | Per-node metrics extraction |
| `validate_extended_budget` | 444–461 | Extended policy dimension validation |
| `validate_payload_budget` | 463–492 | Payload dimension validation |
| `validate_u32_budget` | 494–503 | u32 dimension validator |
| `validate_u64_budget` | 506–513 | u64 dimension validator |
| `validate_aggregate_budget` | 1094–1193 | Aggregate budget policy check |
| `validate_step_ceilings` | 1197–1232 | Step/tick ceiling validation |
| `add_dim` | 1234–1242 | Checked dimension addition |
| `sub_dim` | 1244–1252 | Checked dimension subtraction |
| `check_capacity` | 1254–1268 | Capacity fit check |
| `check_policy` | 1270–1284 | Policy limit check |
| `count_total_steps` | 1304–1344 | DFS total-step walk |
| `find_node_position` | 1346–1370 | Node index lookup |
| `node_at_position` | 1372–1381 | Bounds-checked node access |
| `visit_node_for_total_steps` | 1383–1531 | Per-node step counting |
| `add_conditional_max_steps` | 1533–1549 | Conditional branch max step counting |
| `add_conditional_slot_max_steps` | 1551–1567 | Slot branch max step counting (**DUPLICATE of above**) |
| `checked_step_add` | 1569–1573 | Overflow-safe step addition |
| `count_path_steps` | 1576–1599 | Path step counting |
| `push_path_successors` | 1601–1629 | Successor stack push |
| `iterative_branch_depth` | 1634–1698 | Iterative (non-recursive) branch depth |
| `iterative_slot_branch_depth` | 1701–1707 | Slot branch depth (delegates to above) |
| `push_longest_expr_branch` | 1709–1735 | Longest expr branch finder |
| `push_longest_slot_branch` | 1737–1759 | Longest slot branch finder (**DUPLICATE of above**) |
| `push_selected_branch` | 1761–1765 | Branch push helper |
| `count_and_push_loop_body` | 1768–1796 | Loop body iteration multiplication |
| `push_done_continuation` | 1798–1815 | Loop exit continuation |
| `count_body_region_nodes` | 1817–1846 | Body region node counting |
| `visit_body_region_node` | 1848–1964 | Body region DFS visitor |
| `count_nested_for_region` | 1966–1993 | Nested loop counting |
| `push_successor_targets` | 1995–2037 | Polymorphic successor push |
| `node_kind_has_no_successors` | 2039–2062 | No-succeedor predicate |
| `push_expr_choose_successors` | 2064–2076 | Choose successor push |
| `push_slot_choose_successors` | 2078–2090 | ChooseSlot successor push |
| `push_loop_successors` | 2092–2096 | Loop successor push |
| `push_repeat_check_successors` | 2098–2101 | RepeatCheck successor push |
| `push_together_start_successors` | 2103–2109 | TogetherStart successor push |
| `push_together_branch_successors` | 2111–2115 | TogetherBranch successor push |
| `push_error_handler_successors` | 2117–2121 | ErrorHandler successor push |
| `branch_count_to_u16` | 2123–2130 | Branch count conversion |
| `usize_to_u64_saturating` | 2132–2134 | usize→u64 with saturation |
| `bounded_tracking_vec` | 2136–2138 | Capacity-bounded tracking vec |
| `tracked_steps_contain` | 2140–2142 | Path cycle detection |
| `insert_tracked_step` | 2144–2157 | Path step insertion |
| `remove_tracked_step` | 2159–2163 | Path step removal |
| `insert_tracked_jump_edge` | 2165–2178 | Jump edge tracking |
| `compute_fanout_and_depth` | 2180–2270 | DFS fanout/depth computation |
| `compute_child_depth` | 2272–2299 | Per-node depth calculation |
| `update_fanout` | 2301–2327 | Fanout update logic |
| `update_workflow_metrics` | 2329–2384 | Workflow metric aggregation |

---

## Primitive Obsession Map

**Every budget dimension is a raw primitive.** `WholeWorkflowBudget` contains **18 raw numeric fields** using `u64`, `u32`, `u16` — none wrapped in newtypes.

| Field | Raw Type | Semantic Newtype |
|---|---|---|
| `max_total_steps` | `u64` | `StepCount` |
| `max_total_slots` | `u64` | `SlotCount` |
| `max_fanout` | `u16` | `FanoutFactor` |
| `max_nesting_depth` | `u16` | `NestingDepth` |
| `max_steps_executable` | `u32` | `ExecutableSteps` |
| `max_action_tickets` | `u32` | `ActionCount` |
| `max_parallel_in_flight` | `u16` | `ParallelSlots` |
| `max_retries_per_action` | `u16` | `RetryBudget` |
| `max_gather_pages` | `u32` | `GatherPages` |
| `max_gather_items` | `u32` | `GatherItemCount` |
| `max_for_each_iterations` | `u32` | `IterationCount` |
| `max_together_branches` | `u16` | `BranchCount` |
| `max_repeat_attempts` | `u16` | `AttemptCount` |
| `max_run_time_seconds` | `u64` | `DurationSeconds` |
| `max_result_bytes` | `u32` | `ResultBytes` |
| `max_total_slots_written` | `u32` | `SlotWriteCount` |
| `max_timer_entries` | `u32` | `TimerEntryCount` |
| `max_trace_events` | `u64` | `TraceEventCount` |
| `max_journal_batch_bytes` | `u32` | `JournalBytes` |
| `max_queue_depth` | `u32` | `QueueDepth` |
| `max_ipc_payload_bytes` | `u32` | `IpcBytes` |
| `max_blob_bytes` | `u64` | `BlobBytes` |
| `max_input_bytes` | `u32` | `InputBytes` |

**The same primitive obsession repeats in:**
- `BoundednessPolicy` (16 raw limit fields)
- `AggregateResourceBudget` (18 raw fields)
- `AggregateResourceCapacity` (18 raw fields)
- `AggregateResourceUsage` (18 raw fields)

**Total raw primitive fields across 4 major structs: 70+ untyped numbers.**

---

## Parse Don't Validate Violations

### Violation 1: `BoundednessPolicy::validate` — Raw Comparison Without Parsing

```rust
pub fn validate(&self, budget: &WholeWorkflowBudget) -> Result<(), BudgetError> {
    if budget.max_total_steps > self.max_total_steps {
        return Err(BudgetError::TotalStepsExceeded { ... });
    }
    // ... 9 more identical comparisons
}
```

**Problem:** Values are compared as raw `u64`/`u32`/`u16` with no domain parsing. A `StepCount` newtype would enforce invariants at construction, not at the 11th comparison.

### Violation 2: `validate_u32_budget` / `validate_u64_budget` — Untyped Dimension Dispatch

```rust
fn validate_u32_budget(kind: &'static str, actual: u32, limit: u32) -> Result<(), BudgetError> {
    if actual <= limit { return Ok(()); }
    match kind {
        "journal" => Err(BudgetError::JournalBatchBytesExceeded { actual, limit }),
        "queue"   => Err(BudgetError::QueueDepthExceeded { actual, limit }),
        "ipc"     => Err(BudgetError::IpcPayloadBytesExceeded { actual, limit }),
        _         => Err(BudgetError::InputBytesExceeded { actual, limit }),  // WRONG variant on catch-all!
    }
}
```

**Problems:**
- `kind: &'static str` is string-typed dispatch — should be enum variant
- `_` catch-all maps to `InputBytesExceeded` — could silently mis-route

### Violation 3: `AggregateResourceBudget::from_whole_workflow_budget` — Blind Field Copy

```rust
pub fn from_whole_workflow_budget(
    budget: WholeWorkflowBudget,
    contract: ResourceContract,
) -> Result<Self, AggregateBudgetError> {
    Ok(Self {
        max_steps_executable: budget.max_steps_executable,
        // ... 17 more blind copies
        max_step_budget_per_tick: contract.max_step_budget_per_tick,
        max_transitions_per_tick: contract.max_transitions_per_tick,
    })
}
```

**Problem:** No validation at construction — raw values flow through unchanged. If `max_step_budget_per_tick` is 0, it passes through to `validate_step_ceilings` which only then catches it.

### Violation 4: `count_total_steps` — Direct `u64::MAX` Injection on Error

```rust
BudgetError::TotalStepsExceeded { actual: u64::MAX, limit: u64::MAX }
```

**Problem:** On error paths, actual/limit are set to `u64::MAX` as a sentinel. No newtype distinction between "error sentinel" and "valid maximum value."

---

## Recommended Split

```
crates/vb_core/src/budget/
├── lib.rs                 # Re-exports
├── compute.rs            # ~160 lines: WholeWorkflowBudget, SmallLinearMetrics, compute entry
├── traversal.rs          # ~60 lines: BudgetTraversalError, cycle detection helpers
├── policy.rs             # ~200 lines: BoundednessPolicy, BudgetError, validation logic
├── resource.rs           # ~450 lines: AggregateResourceBudget/Capacity/Usage/Reservation + ops
├── traversal_fns.rs     # ~550 lines: ALL graph-walk functions (count, fanout, depth, successors)
└── kani.rs               # ~320 lines: Kani harnesses (isolated behind #[cfg(kani)])
```

**Line count after split (estimated):**
- `compute.rs`: 166 (types) + 50 (helpers) = **216 lines** ✓
- `traversal.rs`: 60 lines ✓
- `policy.rs`: 180 lines ✓
- `resource.rs`: 450 lines — **STILL OVER** — needs further split:
  - `reservation.rs`: AggregateReservation + error
  - `accounting.rs`: AggregateResourceUsage + try_add/try_sub/fits_within
  - `aggregate.rs`: AggregateResourceBudget + from_workflow
  - `capacity.rs`: AggregateResourceCapacity
- `traversal_fns.rs`: 550 lines — **STILL OVER** — needs further split:
  - `step_count.rs`: count_total_steps, count_path_steps, visit_node_for_total_steps
  - `loop_count.rs`: count_body_region_nodes, count_and_push_loop_body, nested region counting
  - `fanout_depth.rs`: compute_fanout_and_depth, update_fanout, compute_child_depth, update_workflow_metrics
  - `successors.rs`: All push_*_successors functions
  - `branch_helpers.rs`: iterative_branch_depth, push_longest_*_branch

**Final target: 6–8 files × <300 lines each.**

---

## Other Violations

### Code Duplication
- `add_conditional_max_steps` (1553–1549) and `add_conditional_slot_max_steps` (1551–1567) are **identical logic** operating on different branch types — should be unified via generics or trait.
- `push_longest_expr_branch` and `push_longest_slot_branch` are near-duplicates.
- `iterative_slot_branch_depth` is a one-line delegation to `iterative_branch_depth`.

### Kani Harnesses Inline
- `#[cfg(kani)] mod kani_harnesses` (lines 2386–2710) is **324 lines of verification code embedded in production source**. Should be `budget/kani.rs` or in `verification/kani/` workspace.

### Test Modules Stub
- `#[cfg(test)] mod tests;` and `#[cfg(test)] mod vb_qi37_2_4_state8_tests;` are empty placeholder module declarations — dead weight until populated.

---

## Severity Assessment

| Violation | Severity | Effort to Fix |
|---|---|---|
| 9.1x line overage | **CRITICAL** | High — full module rewrite |
| Primitive obsession (70+ raw fields) | **HIGH** | Medium — newtype wrappers + From impls |
| Parse don't validate failures | **HIGH** | Medium — add constructors/validators |
| Code duplication (branch helpers) | **MEDIUM** | Low — unify with generics |
| Kani harnesses inline | **LOW** | Low — move to separate file |
| Empty test stubs | **LOW** | Trivial — remove or populate |

---

## Status

```
STATUS: REFACTOR REQUIRED
VIOLATIONS: 2716 lines | 70+ primitive obsession | 4 Parse-don't-validate failures | 3 duplicate functions
MOON GATE REQUIRED: YES (structural change)
```
