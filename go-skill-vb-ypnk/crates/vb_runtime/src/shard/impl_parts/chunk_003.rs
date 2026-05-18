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
