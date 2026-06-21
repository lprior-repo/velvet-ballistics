impl Shard {
    fn flush_slot_written(
        &mut self,
        run: RunId,
        slot: SlotIdx,
        value: vb_core::value::SlotValue,
        taint: vb_core::Taint,
        extra: Option<crate::primitives::collect::CollectPaginationState>,
    ) -> RuntimeResult<()> {
        let encoded = postcard::to_allocvec(&value).map_err(|_| RuntimeError::EncodeFailed)?;
        let encoded_extra = extra
            .map(|state| postcard::to_allocvec(&state))
            .transpose()
            .map_err(|_| RuntimeError::EncodeFailed)?;
        self.trace_ring.push(TraceEvent::SlotWritten {
            run,
            slot,
            value: encoded.clone(),
        });
        self.append_journal_event(RuntimeJournalEvent::SlotWritten {
            run,
            slot,
            value: encoded,
            taint,
            extra: encoded_extra,
        })
    }

    pub(crate) fn take_frame_for(
        &mut self,
        run: RunId,
        workflow: &CompiledWorkflow,
    ) -> RuntimeResult<RunFrame> {
        let step_count = workflow.node_count();
        let slot_count = workflow.slot_count();
        let key = (step_count, slot_count);
        if !self.frame_pools.contains_key(&key) {
            let pool = FramePool::new(step_count, slot_count, self.max_active_runs)
                .map_err(|_| RuntimeError::FramePoolUnavailable)?;
            self.frame_pools.insert(key, pool);
        }
        let pool = self
            .frame_pools
            .get_mut(&key)
            .ok_or(RuntimeError::FramePoolUnavailable)?;
        pool.take(run, workflow.entry())
            .map_err(|_| RuntimeError::FramePoolUnavailable)
    }

    pub(crate) fn release_frame(&mut self, frame: RunFrame) {
        let key = (frame.step_count(), frame.slot_count());
        if let Some(pool) = self.frame_pools.get_mut(&key) {
            pool.release(frame);
        }
    }

    /// Drains the command queue by processing commands until shutdown or capacity limit.
    pub fn drain_for_shutdown(&mut self) -> RuntimeResult<()> {
        let limit = self.command_queue.capacity();
        let mut processed = 0usize;
        while processed < limit {
            if !self.tick()? {
                self.cancel_pending_timers_for_shutdown()?;
                return Ok(());
            }
            processed = processed.saturating_add(1);
        }
        Err(RuntimeError::ShutdownInProgress)
    }

    /// Journals a `WaitCancelled` or `AskCancelled` event for every pending
    /// timer, then clears the timer map. Called once the `Shutdown` command
    /// has been observed so the durable journal records the cancellation
    /// before the runs are dropped from the in-memory state.
    fn cancel_pending_timers_for_shutdown(&mut self) -> RuntimeResult<()> {
        let pending: Vec<(RunId, StepIdx, PendingTimerKind)> = self
            .pending_timers
            .iter()
            .map(|(run_id, timer)| (*run_id, timer.step, timer.kind))
            .collect();
        for (run, step, kind) in pending {
            match kind {
                PendingTimerKind::Wait => {
                    self.append_journal_event(RuntimeJournalEvent::WaitCancelled { run, step })?;
                }
                PendingTimerKind::Ask => {
                    self.append_journal_event(RuntimeJournalEvent::AskCancelled { run, step })?;
                }
            }
        }
        self.pending_timers.clear();
        Ok(())
    }

    /// Drains currently queued commands, then marks the shard shut down.
    ///
    /// Pending timers are journaled via
    /// [`Self::cancel_pending_timers_for_shutdown`] before being cleared so
    /// the durable record matches the in-memory cleanup (RQ-W0-12, fixes
    /// RS-206 which previously bypassed the cancellation journal).
    pub fn drain_pending_and_shutdown(&mut self) -> RuntimeResult<()> {
        if self.is_shutting_down() {
            self.cancel_pending_timers_for_shutdown()?;
            return Ok(());
        }
        self.drain_pending_commands(self.command_queue.len())?;
        self.shutting_down
            .store(true, std::sync::atomic::Ordering::Release);
        self.cancel_pending_timers_for_shutdown()?;
        Ok(())
    }

    fn drain_pending_commands(&mut self, command_count: usize) -> RuntimeResult<()> {
        (0..command_count).try_for_each(|_| {
            if self.command_queue.is_empty() || self.is_shutting_down() {
                return Ok(());
            }
            if !self.tick()? {
                // Journal the timer cancellations for runs that were
                // suspended before the queue drained so the durable journal
                // records the cancellation before the in-memory state is
                // dropped (RQ-W0-12).
                self.cancel_pending_timers_for_shutdown()?;
            }
            Ok(())
        })
    }
}
