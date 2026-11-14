impl Shard {
    fn restore_suspended_boundary(
        &mut self,
        run: RunId,
        boundary: crate::recovery::RecoveredRunBoundary,
        retry_timer: Option<PendingTimer>,
    ) -> RuntimeResult<bool> {
        match boundary.kind() {
            crate::recovery::RecoveredRunBoundaryKind::RetryTimer => {
                self.restore_retry_timer(run, retry_timer)
            }
            crate::recovery::RecoveredRunBoundaryKind::PendingAction => {
                self.restore_pending_action(run, boundary)
            }
            crate::recovery::RecoveredRunBoundaryKind::OpenAsk => {
                self.restore_open_ask(run, boundary)
            }
            crate::recovery::RecoveredRunBoundaryKind::OpenWaitEvent => {
                self.restore_open_wait_event(run, boundary)
            }
            crate::recovery::RecoveredRunBoundaryKind::WaitingTimer => {
                self.restore_waiting_timer(run, boundary)
            }
            crate::recovery::RecoveredRunBoundaryKind::TimedAsk => {
                self.restore_timed_ask(run, boundary)
            }
            crate::recovery::RecoveredRunBoundaryKind::None => Ok(false),
        }
    }

    fn restore_retry_timer(
        &mut self,
        run: RunId,
        timer: Option<PendingTimer>,
    ) -> RuntimeResult<bool> {
        let timer = timer.ok_or(RuntimeError::InvalidTimerFire)?;
        self.pending_timer_insert(run, timer)?;
        self.apply(run, RuntimeEvent::AwaitTimer)?;
        Ok(true)
    }

    fn restore_pending_action(
        &mut self,
        run: RunId,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> RuntimeResult<bool> {
        let Some(ticket) = boundary.pending_action_ticket().map(|value| value.ticket()) else {
            return Ok(false);
        };
        if let Some(state) = self.run_state_get_mut(run) {
            crate::shard::helpers::record_scheduled_attempt(state, ticket);
        }
        self.reinject_pending_action(run, ticket)?;
        self.apply(run, RuntimeEvent::AwaitAction)?;
        Ok(true)
    }

    fn restore_open_ask(
        &mut self,
        run: RunId,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> RuntimeResult<bool> {
        let Some(ask) = boundary.open_ask() else {
            return Ok(false);
        };
        self.record_pending_ask_authority(run, ask.step(), ask.boundary_seq());
        self.apply(run, RuntimeEvent::AwaitTimer)?;
        Ok(true)
    }

    pub(crate) fn restore_open_wait_event(
        &mut self,
        run: RunId,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> RuntimeResult<bool> {
        let Some(wait) = boundary.open_wait_event() else {
            return Ok(false);
        };
        self.run_state_get_mut(run)
            .ok_or(RuntimeError::RunNotFound)?
            .frame
            .mark_waiting(wait.step())
            .map_err(|_| RuntimeError::Recovery {
                error: String::from("recover open wait frame state"),
            })?;
        self.apply(run, RuntimeEvent::AwaitTimer)?;
        Ok(true)
    }

    fn restore_waiting_timer(
        &mut self,
        run: RunId,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> RuntimeResult<bool> {
        let Some(timer) = boundary.waiting_timer() else {
            return Ok(false);
        };
        self.restore_recovered_timer_boundary(run, timer, PendingTimerKind::Wait)?;
        self.apply(run, RuntimeEvent::AwaitTimer)?;
        Ok(true)
    }

    fn restore_timed_ask(
        &mut self,
        run: RunId,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> RuntimeResult<bool> {
        let Some(ask) = boundary.timed_ask() else {
            return Ok(false);
        };
        self.restore_recovered_timer_boundary(run, ask, PendingTimerKind::Ask)?;
        self.record_pending_ask_authority(run, ask.step(), ask.boundary_seq());
        self.apply(run, RuntimeEvent::AwaitTimer)?;
        Ok(true)
    }
}
