# Domain Model Review: vb-qi37.2.1 — Aggregate Resource Budget Model

## Model Shape

`AggregateResourceBudget` is a 16-dimensional resource requirement struct where each field represents a hard ceiling on a named resource dimension. The model has three related types:

- **`AggregateResourceBudget`** — workflow-level ceiling (u32/u16 fields, derived from workflow IR + `ResourceContract`).
- **`AggregateResourceCapacity`** — shard-runtime available resource (u64 fields, from `ShardConfig`).
- **`AggregateResourceUsage`** — currently reserved/used resources (u64 fields, maintained by admission/release).

### Dimension Mapping

| Budget field | Capacity field | Usage field | Unit |
|---|---|---|---|
| `max_steps_executable` | `max_steps_executable` | `max_steps_executable` | steps (u32→u64) |
| `max_action_tickets` | `max_action_tickets` | `max_action_tickets` | tickets (u32→u64) |
| `max_parallel_in_flight` | `max_parallel_in_flight` | `max_parallel_in_flight` | actions (u16→u32→u64) |
| `max_retries_per_action` | — | — | retries (u16) |
| `max_gather_pages` | `max_gather_pages` | `max_gather_pages` | pages (u32→u64) |
| `max_gather_items` | `max_gather_items` | `max_gather_items` | items (u32→u64) |
| `max_for_each_iterations` | — | — | iterations (u32) |
| `max_together_branches` | — | — | branches (u16) |
| `max_repeat_attempts` | — | — | attempts (u16) |
| `max_run_time_seconds` | — | — | seconds (u64) |
| `max_result_bytes` | `max_result_bytes` | `max_result_bytes` | bytes (u32→u64) |
| `max_total_slots_written` | `max_total_slots_written` | `max_total_slots_written` | slots (u32→u64) |
| `max_queue_depth` | `max_queue_depth` | `max_queue_depth` | entries (u32→u64) |
| `max_journal_batch_bytes` | `max_journal_batch_bytes` | `max_journal_batch_bytes` | bytes (u32→u64) |
| `max_step_budget_per_tick` | `max_step_budget_per_tick` | `max_step_budget_per_tick` | steps (u64) |
| `max_transitions_per_tick` | `max_transitions_per_tick` | `max_transitions_per_tick` | transitions (u64) |

## Arithmetic Model

All budget arithmetic is component-wise checked:

- **`try_add_budget`**: `new_dim = current_dim.checked_add(budget_dim)` → `Ok(new)` or `Err(Overflow { dim })`
- **`try_subtract_budget`**: `new_dim = current_dim.checked_sub(budget_dim)` → `Ok(new)` or `Err(Underflow { dim })`

No wrapping. No saturation. No casting that loses significant bits.

### Width Narrows

Budget fields are u32/u16 but Usage/Capacity fields are u64. Conversions use `u64::from(budget_field)`. Overflow in narrowing is detected by `validate_aggregate_budget` against `BoundednessPolicy`.

### Step Ceiling Hard Limits

`max_step_budget_per_tick` and `max_transitions_per_tick` have hard-coded limits (`HARD_MAX_STEP_BUDGET_PER_TICK = 1_000_000`, `HARD_MAX_TRANSITIONS_PER_TICK = 1_000_000`). Zero is also rejected. These are validated separately via `validate_step_ceilings`.

## Error Semantics

| Error variant | Trigger | Carries |
|---|---|---|
| `WorkflowBudget(WorkflowError)` | Invalid workflow IR | `WorkflowError` |
| `PolicyExceeded { resource, actual, limit }` | Budget > BoundednessPolicy | resource name, actual, limit |
| `CapacityExceeded { resource, requested, available }` | Requested > Available | resource name, requested, available |
| `Overflow { resource }` | `checked_add` fails | resource name only |
| `Underflow { resource }` | `checked_sub` fails | resource name only |
| `InvalidCapacity { resource }` | Zero capacity for required dim | resource name only |
| `ReservationNotFound { run }` | Release unknown RunId | RunId |
| `StepCeilingExceeded { requested, limit }` | step budget = 0 or > hard limit | requested, limit |
| `PerTickCeilingExceeded { requested, limit }` | transitions = 0 or > hard limit | requested, limit |

## Capacity Comparison

`fits_within(capacity)` is a conjunction of per-dimension `<=` checks, evaluated in a fixed dimension order. The first failing dimension returns `CapacityExceeded`. Equality (usage == capacity) is an admit.

## Review Findings

1. **Width asymmetry is intentional and safe** — Budget fields (u32/u16) are narrower than Usage/Capacity (u64). Narrowing is safe because `validate_aggregate_budget` rejects values that don't fit the target width.
2. **Checked arithmetic is correctly implemented** — `add_dim` and `sub_dim` use `checked_add`/`checked_sub` with error return, not panicking variants.
3. **Step ceiling hard limits are appropriate** — 1_000_000 as a hard upper bound for per-tick step/transitions is a reasonable operational limit.
4. **BH-BUD-06 addressed** — No `saturating_add` or `saturating_sub` in any budget arithmetic path.
5. **BH-BUD-07 addressed** — `gather_items` dimension uses the same `add_dim`/`sub_dim` pattern as all other dimensions.

## Open Questions

- Should `max_retries_per_action`, `max_for_each_iterations`, `max_together_branches`, `max_repeat_attempts`, and `max_run_time_seconds` have corresponding Capacity/Usage tracking fields, or are they budget-only dimensions?
- Should `max_step_budget_per_tick` and `max_transitions_per_tick` be validated against policy limits in `validate_aggregate_budget` or only against hard limits in `validate_step_ceilings`?
