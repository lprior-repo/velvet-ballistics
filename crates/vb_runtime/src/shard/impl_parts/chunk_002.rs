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
                self.pending_timers.clear();
                return Ok(());
            }
            processed = processed.saturating_add(1);
        }
        Err(RuntimeError::ShutdownInProgress)
    }
}
