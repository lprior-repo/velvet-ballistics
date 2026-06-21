use crate::shard::config::validate_shard_config_inputs;
use crate::shard::bounded_outcomes::DEFAULT_MAX_TERMINAL_OUTCOMES;

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
            1,
            crate::shard::DEFAULT_MAX_TERMINAL_RUNS,
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
            max_terminal_outcomes: DEFAULT_MAX_TERMINAL_OUTCOMES,
        })
    }

    /// Creates a new ShardConfig with full field control.
    ///
    /// Validates every capacity/interval/TTL field against typed
    /// `RuntimeError` variants so struct-literal bypass via
    /// `Shard::new(ShardConfig { .. })` cannot sneak past invariants.
    /// Added in RQ-W0-15.
    #[allow(clippy::too_many_arguments)]
    pub fn new_full(
        command_queue_capacity: usize,
        trace_capacity: usize,
        step_budget_per_tick: u64,
        max_active_runs: usize,
        policy: vb_core::policy::RuntimePolicy,
        coalesce_window_ticks: u32,
        snapshot_interval_steps: u64,
        max_terminal_runs: usize,
        terminal_runs_ttl_ticks: u64,
        max_terminal_outcomes: usize,
    ) -> RuntimeResult<Self> {
        validate_shard_config_inputs(
            command_queue_capacity,
            trace_capacity,
            step_budget_per_tick,
            max_active_runs,
            coalesce_window_ticks,
            max_terminal_runs,
        )?;
        Ok(Self {
            command_queue_capacity,
            trace_capacity,
            step_budget_per_tick,
            max_active_runs,
            policy,
            coalesce_window_ticks,
            snapshot_interval_steps,
            max_terminal_runs,
            terminal_runs_ttl_ticks,
            max_terminal_outcomes,
        })
    }
}
