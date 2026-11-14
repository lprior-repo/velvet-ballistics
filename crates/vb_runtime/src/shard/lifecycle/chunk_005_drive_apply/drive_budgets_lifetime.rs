impl Shard {
    fn check_run_time_budget(state: &mut RunState) -> RuntimeEngineResult<()> {
        let Some(budget) = state
            .admission
            .as_ref()
            .and_then(crate::admission::RunAdmission::budget)
        else {
            return Ok(());
        };
        let now = crate::shard::helpers::wall_clock_millis_since_epoch()
            .ok_or_else(|| Self::lifetime_invariant("wall clock unavailable"))?;
        let started = state
            .run_started_millis
            .ok_or_else(|| Self::lifetime_invariant("budget_origin_missing"))?;
        let Ok(lifetime) = vb_core::budget::RunLifetimeBudget::from_budget(started, &budget) else {
            return Self::run_time_budget_exceeded(budget.max_run_time_seconds);
        };
        if lifetime.check(now).is_err() {
            return Self::run_time_budget_exceeded(budget.max_run_time_seconds);
        }
        Ok(())
    }

    fn lifetime_invariant(reason: &'static str) -> crate::engine::types::RuntimeEngineError {
        crate::engine::types::RuntimeEngineError::Core(
            vb_core::engine::EngineError::InternalInvariantViolation {
                reason: reason.into(),
            },
        )
    }

    fn run_time_budget_exceeded(limit: u64) -> RuntimeEngineResult<()> {
        Err(crate::engine::types::RuntimeEngineError::Core(
            vb_core::engine::EngineError::BudgetExceeded {
                budget: "run_time",
                limit,
            },
        ))
    }
}
