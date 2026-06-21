# RS-217-core-shard-config-validation-omits-fields: Public shard config fields have validators that are not enforced together

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/config.rs:90`
- **Confidence**: confirmed

## Description
`ShardConfig` exposes public fields and declares validity predicates, but `validate_shard_config_inputs` validates only four inputs. Invalid `coalesce_window_ticks` and terminal-run retention settings can be constructed by struct literal without hitting a unified config validator.

## Evidence
```rust
45:     pub coalesce_window_ticks: u32,
...
63:     pub max_terminal_runs: usize,
...
70:     pub terminal_runs_ttl_ticks: u64,
...
90: pub const fn is_valid_coalesce_window_ticks(count: u32) -> bool {
91:     count > 0
92: }
...
103: pub fn validate_shard_config_inputs(
104:     command_queue_capacity: usize,
105:     trace_capacity: usize,
106:     step_budget_per_tick: u64,
107:     max_active_runs: usize,
108: ) -> Result<(), crate::RuntimeError> {
```

The validator cannot reject `coalesce_window_ticks == 0` because it does not receive that field. It also cannot validate terminal-run capacity or TTL, even though those fields control bounded retention behavior.

## Adversarial Check
This is not only a missing helper call. The struct fields are public, so callers do not need a constructor to create invalid configs. The presence of `is_valid_coalesce_window_ticks` proves that zero is outside the intended domain, but there is no all-fields validation function in this source file that enforces it.

## Suggested Fix
Add a `ShardConfig::try_new` or `validate(&self)` that checks every public field with a declared invariant, including coalescing and terminal-run retention. Prefer private fields plus checked constructors so invalid configs cannot be built directly.
