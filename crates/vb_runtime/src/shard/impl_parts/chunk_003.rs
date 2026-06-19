use crate::shard::types::{is_valid_step_budget_per_tick, is_valid_trace_capacity};

fn validate_shard_config_inputs(
    command_queue_capacity: usize,
    trace_capacity: usize,
    step_budget_per_tick: u64,
    max_active_runs: usize,
) -> RuntimeResult<()> {
    validate_command_queue_capacity(command_queue_capacity)?;
    validate_trace_capacity(trace_capacity)?;
    validate_step_budget(step_budget_per_tick)?;
    validate_max_active_runs(max_active_runs)
}

fn validate_command_queue_capacity(capacity: usize) -> RuntimeResult<()> {
    if is_valid_command_queue_capacity(capacity) {
        Ok(())
    } else {
        Err(RuntimeError::CommandQueueCapacityExceeded {
            capacity,
            max: MAX_COMMAND_QUEUE_CAPACITY,
        })
    }
}

fn validate_trace_capacity(capacity: usize) -> RuntimeResult<()> {
    if is_valid_trace_capacity(capacity) {
        Ok(())
    } else {
        Err(RuntimeError::UnsupportedOperation {
            operation: "trace_capacity_zero",
        })
    }
}

fn validate_step_budget(budget: u64) -> RuntimeResult<()> {
    if is_valid_step_budget_per_tick(budget) {
        Ok(())
    } else {
        Err(RuntimeError::UnsupportedOperation {
            operation: "step_budget_per_tick_zero",
        })
    }
}

fn validate_max_active_runs(max_active_runs: usize) -> RuntimeResult<()> {
    if max_active_runs == 0 {
        Err(RuntimeError::ActiveRunCapacityZero)
    } else {
        Ok(())
    }
}

impl ShardConfig {
    /// Creates a new ShardConfig, validating capacity limits.
    pub fn new(
        command_queue_capacity: usize,
        trace_capacity: usize,
        step_budget_per_tick: u64,
        max_active_runs: usize,
        policy: vb_core::policy::RuntimePolicy,
    ) -> RuntimeResult<Self> {
        validate_shard_config_inputs(
            command_queue_capacity,
            trace_capacity,
            step_budget_per_tick,
            max_active_runs,
        )?;
        Ok(Self {
            command_queue_capacity,
            trace_capacity,
            step_budget_per_tick,
            max_active_runs,
            policy,
            coalesce_window_ticks: 1,
            snapshot_interval_steps: 0,
            max_terminal_runs: crate::shard::DEFAULT_MAX_TERMINAL_RUNS,
            terminal_runs_ttl_ticks: crate::shard::DEFAULT_TERMINAL_RUNS_TTL_TICKS,
        })
    }
}
