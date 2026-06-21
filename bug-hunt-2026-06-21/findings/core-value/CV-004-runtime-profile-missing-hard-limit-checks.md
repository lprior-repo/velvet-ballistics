# CV-004: `RuntimeLimitsProfile::new` skips validation for 7 fields including `trace_ring_capacity`

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/policy/contract.rs:153-272`
- **Confidence**: confirmed

## Description

The smart constructor validates 12 fields against `MAX_*` constants but skips 7 fields entirely. The skipped fields only pass through the `nz_*` (non-zero) check at the bottom — they are not bounded by their corresponding hard limits.

Validated: `active_runs`, `ready_queue_depth`, `ipc_frame_bytes`, `action_input_bytes`, `action_output_bytes`, `step_output_bytes`, `result_bytes`, `journal_writer_queue_capacity`, `together_branch_count`, `collect_items`, `repeat_attempts`, `retry_attempts`.

**Not validated against hard limits**:
- `trace_ring_capacity` (should be vs `MAX_TRACE_RING_CAPACITY = 1_048_576`)
- `for_each_item_count` (no published hard limit, but boundedness policy reads it)
- `collect_pages`
- `collect_time_seconds`
- `repeat_time_seconds`
- `max_wait_duration_seconds`
- `ask_timeout_seconds`

## Evidence

`contract.rs:153-242` — the `if config.X == 0 || config.X > MAX_X` block only covers 12 fields. There is no `if config.trace_ring_capacity > MAX_TRACE_RING_CAPACITY` check.

A caller can construct:

```rust
RuntimeLimitsProfile::new(ProfileName::Relaxed, RuntimeLimitsConfig {
    trace_ring_capacity: usize::MAX,
    ...
})
```

The constructor returns `Ok(...)`. The resulting profile then drives `BoundednessPolicy::from_profile` (contract.rs:480-517) which sets `absolute_max_trace_events = usize_to_u64(profile.trace_ring_capacity.get())` — that saturates to `u64::MAX`. The resulting policy permits unlimited trace events.

## Adversarial Check

The three canonical profile factories (`strict()`, `journaled()`, `relaxed()`) hand-pick literals that are within bounds (contract.rs:374-455), so they bypass the issue via `from_validated_config`. The bug surfaces for any consumer that calls `RuntimeLimitsProfile::new` directly with caller-supplied values — which is the public, intended use of the smart constructor (per its doc "validates all fields against hard limits"). The doc-contract is false for 7 of the 19 fields.

## Suggested Fix

Add validation blocks for the missing fields, paralleling the existing pattern:

```rust
if config.trace_ring_capacity == 0 || config.trace_ring_capacity > MAX_TRACE_RING_CAPACITY {
    return Err(ProfileValidationError::ExceedsHardLimit {
        field: "trace_ring_capacity",
        value: usize_to_u64(config.trace_ring_capacity),
        limit: usize_to_u64(MAX_TRACE_RING_CAPACITY),
    });
}
```

Decide on published hard limits for the time/iteration fields (`for_each_item_count`, `collect_pages`, `collect_time_seconds`, `repeat_time_seconds`, `max_wait_duration_seconds`, `ask_timeout_seconds`) — either add `MAX_*` constants to `limits.rs` or document these as user-discretion fields that bypass hard-limit checking. Either way the doc-comment on `RuntimeLimitsProfile::new` must be corrected.
