impl Shard {
    fn check_retry_budget(state: &RunState, signal: &RuntimeSignal) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let limit = budget.max_retries_per_action;
        for attempts in state.action_attempts.iter() {
            if attempts.saturating_sub(1) > limit {
                return Err(crate::engine::types::RuntimeEngineError::Core(
                    vb_core::engine::EngineError::BudgetExceeded {
                        budget: "retries_per_action",
                        limit: u64::from(limit),
                    },
                ));
            }
        }
        if let RuntimeSignal::AwaitingAction(ticket) = signal
            && ticket.attempt.saturating_sub(1) > limit
        {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "retries_per_action",
                    limit: u64::from(limit),
                },
            ));
        }
        Ok(())
    }

    fn check_incident_budget(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        if state.frame.executed() > budget.max_incident_emissions {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "incident_emissions",
                    limit: budget.max_incident_emissions,
                },
            ));
        }
        Ok(())
    }

    fn check_timer_entry_budget(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let mut timer_entries: u64 = 0;
        for index in 0..state.frame.step_count() {
            let step = vb_core::ids::StepIdx::new(index);
            if matches!(
                state.frame.step_state(step),
                Ok(vb_core::frame::StepState::Waiting) | Ok(vb_core::frame::StepState::Asking)
            ) {
                timer_entries = timer_entries.saturating_add(1);
            }
        }
        if timer_entries > u64::from(budget.max_timer_entries) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "timer_entries",
                    limit: u64::from(budget.max_timer_entries),
                },
            ));
        }
        Ok(())
    }

    fn check_parallel_in_flight_budget(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let in_flight = u64::from(state.frame.parallel_in_flight());
        if in_flight > u64::from(budget.max_parallel_in_flight) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "parallel_in_flight",
                    limit: u64::from(budget.max_parallel_in_flight),
                },
            ));
        }
        Ok(())
    }

    fn check_total_slots_written_budget(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let written = state.frame.written_slot_count();
        if written > u64::from(budget.max_total_slots_written) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "total_slots_written",
                    limit: u64::from(budget.max_total_slots_written),
                },
            ));
        }
        Ok(())
    }

    fn check_steps_executable_budget(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        if state.frame.executed() > u64::from(budget.max_steps_executable) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "steps_executable",
                    limit: u64::from(budget.max_steps_executable),
                },
            ));
        }
        Ok(())
    }

    fn check_gather_pages_budget(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        if state.collect_states.total_pages_delivered() > u64::from(budget.max_gather_pages) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "gather_pages",
                    limit: u64::from(budget.max_gather_pages),
                },
            ));
        }
        Ok(())
    }
}
