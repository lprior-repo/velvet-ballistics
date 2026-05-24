use crate::shard::types::{is_valid_step_budget_per_tick, is_valid_trace_capacity};

impl ShardConfig {
    /// Creates a new ShardConfig, validating capacity limits.
    pub fn new(
        command_queue_capacity: usize,
        trace_capacity: usize,
        step_budget_per_tick: u64,
        max_active_runs: usize,
        policy: vb_core::policy::RuntimePolicy,
    ) -> RuntimeResult<Self> {
        if !is_valid_command_queue_capacity(command_queue_capacity) {
            return Err(RuntimeError::CommandQueueCapacityExceeded {
                capacity: command_queue_capacity,
                max: MAX_COMMAND_QUEUE_CAPACITY,
            });
        }
        if !is_valid_trace_capacity(trace_capacity) {
            return Err(RuntimeError::UnsupportedOperation {
                operation: "trace_capacity_zero",
            });
        }
        if !is_valid_step_budget_per_tick(step_budget_per_tick) {
            return Err(RuntimeError::UnsupportedOperation {
                operation: "step_budget_per_tick_zero",
            });
        }
        if max_active_runs == 0 {
            return Err(RuntimeError::ActiveRunCapacityZero);
        }
        Ok(Self {
            command_queue_capacity,
            trace_capacity,
            step_budget_per_tick,
            max_active_runs,
            policy,
        })
    }
}
