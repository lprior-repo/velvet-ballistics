impl Shard {
    fn check_result_bytes_budget(
        state: &RunState,
        signal: &RuntimeSignal,
    ) -> RuntimeEngineResult<()> {
        let RuntimeSignal::Finished(value) = signal else {
            return Ok(());
        };
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let size = state
            .store
            .deep_size(value)
            .map_err(crate::engine::types::RuntimeEngineError::Core)?;
        if size > u64::from(budget.max_result_bytes) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "result_bytes",
                    limit: u64::from(budget.max_result_bytes),
                },
            ));
        }
        Ok(())
    }

    fn check_run_time_budget(state: &mut RunState) -> RuntimeEngineResult<()> {
        let Some(budget) = state
            .admission
            .as_ref()
            .and_then(crate::admission::RunAdmission::budget)
        else {
            return Ok(());
        };
        let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or(
            crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::InternalInvariantViolation {
                    reason: "wall clock unavailable".into(),
                },
            ),
        )?;
        let started = match state.run_started_millis {
            Some(started) => started,
            None => {
                return Err(crate::engine::types::RuntimeEngineError::Core(
                    vb_core::engine::EngineError::InternalInvariantViolation {
                        reason: "budget_origin_missing".into(),
                    },
                ));
            }
        };
        let Ok(lifetime) = vb_core::budget::RunLifetimeBudget::from_budget(started, &budget) else {
            return Self::run_time_budget_exceeded(budget.max_run_time_seconds);
        };
        if lifetime.check(now).is_err() {
            return Self::run_time_budget_exceeded(budget.max_run_time_seconds);
        }
        Ok(())
    }

    fn run_time_budget_exceeded(limit: u64) -> RuntimeEngineResult<()> {
        Err(crate::engine::types::RuntimeEngineError::Core(
            vb_core::engine::EngineError::BudgetExceeded {
                budget: "run_time",
                limit,
            },
        ))
    }

    /// Fails at most one live run whose admitted wall-clock lifetime has
    /// expired. A suspended run holds no instruction budget and may never be
    /// driven again, so a tick-time sweep is the only enforcement point that
    /// bounds its wall-clock lifetime.
    pub(crate) fn expire_overdue_run_lifetime(&mut self) -> RuntimeResult<()> {
        let Some(now) = crate::shard::helpers::wall_clock_millis_since_epoch() else {
            return Ok(());
        };
        let expired = self.runs.iter().find_map(|(&run, state)| {
            let budget = state.admission.as_ref()?.budget()?;
            let Some(started) = state.run_started_millis else {
                return Some(run);
            };
            match vb_core::budget::RunLifetimeBudget::from_budget(started, &budget) {
                Ok(lifetime) if !lifetime.is_expired(now) => None,
                _ => Some(run),
            }
        });
        let Some(run) = expired else {
            return Ok(());
        };
        let state = self.take_run_state(run)?;
        match self.fail_run_state(run, state)? {
            crate::shard::transitions::TransitionOutcome::Applied => Ok(()),
            crate::shard::transitions::TransitionOutcome::JournalBudgetExceeded(state) => self
                .fail_run_on_journal_budget(run, *state, Vec::new())
                .map_err(|failure| match failure {
                    crate::shard::lifecycle::DriveApplyFailure::BeforeCommit(error)
                    | crate::shard::lifecycle::DriveApplyFailure::AfterCommit(error) => error,
                }),
        }
    }

    fn check_suspension_budgets(
        state: &RunState,
        manual_open_gates: u64,
    ) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let mut ask_gates: u64 = 0;
        let mut signal_subscriptions: u64 = 0;
        for index in 0..state.frame.step_count() {
            let step = vb_core::ids::StepIdx::new(index);
            let Some(node) = state.workflow.node(step) else {
                continue;
            };
            if matches!(
                state.frame.step_state(step),
                Ok(vb_core::frame::StepState::Asking)
            ) && matches!(node.kind, vb_core::workflow::CompiledNodeKind::Ask { .. })
            {
                ask_gates = ask_gates.saturating_add(1);
            }
            if matches!(
                state.frame.step_state(step),
                Ok(vb_core::frame::StepState::Waiting)
            ) && matches!(
                node.kind,
                vb_core::workflow::CompiledNodeKind::WaitEvent { .. }
            ) {
                signal_subscriptions = signal_subscriptions.saturating_add(1);
            }
        }
        let open_gates = ask_gates.saturating_add(manual_open_gates);
        if open_gates > u64::from(budget.max_open_gates) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "open_gates",
                    limit: u64::from(budget.max_open_gates),
                },
            ));
        }
        if signal_subscriptions > u64::from(budget.max_signal_subscriptions) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "signal_subscriptions",
                    limit: u64::from(budget.max_signal_subscriptions),
                },
            ));
        }
        Ok(())
    }

    fn check_value_store_budget(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let limit = budget.max_value_store_entries;
        if state.store.total_arena_count() > limit {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "value_store_entries",
                    limit,
                },
            ));
        }
        Ok(())
    }

    /// Tick transition allowance for a run from its admitted total-transition
    /// budget, or the full per-tick budget when the run carries no admitted
    /// budget. Remaining transitions derive from the frame executed count,
    /// which round-trips through the run frame checkpoint, so a recovered
    /// run resumes with the same allowance and no new durable state is
    /// required. Exhaustion fails the run, matching the typed outcome for
    /// total-transition exhaustion.
    fn transition_tick_budget(
        state: &RunState,
        step_budget_per_tick: u64,
    ) -> RuntimeEngineResult<vb_core::engine::StepBudget> {
        let tick = vb_core::engine::StepBudget::new(step_budget_per_tick);
        let Some(admission) = state.admission.as_ref() else {
            return Ok(tick);
        };
        let Some(budget) = admission.budget() else {
            return Ok(tick);
        };
        let limit = budget.max_transitions;
        let Some(left) = limit.checked_sub(state.frame.executed()) else {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "transitions",
                    limit,
                },
            ));
        };
        if left == 0 {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "transitions",
                    limit,
                },
            ));
        }
        Ok(vb_core::engine::StepBudget::new(
            step_budget_per_tick.min(left),
        ))
    }

    fn check_cost_budget(state: &RunState, signal: &RuntimeSignal) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let mut issued: u64 = 0;
        for attempts in state.action_attempts.iter() {
            issued = issued.saturating_add(u64::from(*attempts));
        }
        if let RuntimeSignal::AwaitingAction(ticket) = signal {
            let recorded = match state.action_attempts.get(ticket.step.as_usize()) {
                Some(value) => *value,
                None => 0,
            };
            issued = issued
                .saturating_sub(u64::from(recorded))
                .saturating_add(u64::from(recorded.max(ticket.attempt)));
        }
        if issued > budget.max_cost_units {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "cost_units",
                    limit: budget.max_cost_units,
                },
            ));
        }
        Ok(())
    }

    fn check_action_ticket_budget(
        state: &RunState,
        signal: &RuntimeSignal,
    ) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let mut distinct: u64 = 0;
        for attempts in state.action_attempts.iter() {
            if *attempts > 0 {
                distinct = distinct.saturating_add(1);
            }
        }
        if let RuntimeSignal::AwaitingAction(ticket) = signal {
            let recorded = match state.action_attempts.get(ticket.step.as_usize()) {
                Some(value) => *value,
                None => 0,
            };
            if recorded == 0 {
                distinct = distinct.saturating_add(1);
            }
        }
        if distinct > u64::from(budget.max_action_tickets) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "action_tickets",
                    limit: u64::from(budget.max_action_tickets),
                },
            ));
        }
        Ok(())
    }
}
