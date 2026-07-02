impl Shard {
    /// Returns a read-only snapshot of this shard's pending boundary state.
    ///
    /// Mirrors the read-only contract documented on
    /// [`crate::runtime::Runtime::pending_boundary_snapshot`]: `&self` is
    /// required and the returned value is a tree of immutable slices plus
    /// plain integers, so handing the snapshot to a separate observer
    /// cannot leak mutable state back into the shard.
    ///
    /// `max_items` is applied per collection (active runs, pending timers,
    /// pending actions, pending asks) and the per-collection sort is stable
    /// (by `run_id`, then `ask_step` for asks), so two consecutive calls
    /// with the same shard state and the same `max_items` produce bit-equal
    /// snapshots. The per-collection item count is the **untruncated** count
    /// (e.g. `pending_ask_count` is the full count of asking steps across
    /// all active runs), while the boxed-slice payload is truncated to
    /// `max_items`. The `truncated` flag is `true` when any of the four
    /// collections was truncated.
    ///
    /// Pending asks are derived from the run-state scheduler (every step
    /// whose `StepState` is `Asking`), not from the timer wheel, so an
    /// `Ask` node declared with `timeout_slot = None` shows up in
    /// `pending_asks` with `timeout() == None`. The associated
    /// `PendingAskTimeoutBoundarySnapshot` is only populated when a matching
    /// `PendingTimerKind::Ask` entry is registered for `(run_id, ask_step)`.
    #[must_use]
    pub fn pending_boundary_snapshot(
        &self,
        shard_id: u32,
        max_items: usize,
    ) -> ShardPendingBoundarySnapshot {
        let PendingAskSnapshotSet {
            count: pending_ask_count,
            items: pending_asks,
        } = self.pending_ask_snapshots(max_items);
        ShardPendingBoundarySnapshot {
            shard_id,
            command_queue_depth: self.command_queue.len(),
            command_queue_capacity: self.command_queue.capacity(),
            active_run_count: self.runs.len(),
            pending_timer_count: self.pending_timers.len(),
            pending_action_count: self.pending_actions.len(),
            pending_ask_count,
            active_runs: self.active_run_snapshots(max_items),
            pending_timers: self.pending_timer_snapshots(max_items),
            pending_actions: self.pending_action_snapshots(max_items),
            pending_asks,
            truncated: self.snapshot_truncated(max_items, pending_ask_count),
        }
    }

    fn pending_ask_count(&self) -> usize {
        self.runs.values().fold(0usize, |count, state| {
            count.saturating_add(asking_step_count(state))
        })
    }

    fn snapshot_truncated(&self, max_items: usize, pending_ask_count: usize) -> bool {
        self.runs.len() > max_items
            || self.pending_timers.len() > max_items
            || self.pending_actions.len() > max_items
            || pending_ask_count > max_items
    }

    fn active_run_snapshots(&self, max_items: usize) -> Box<[RunId]> {
        let mut runs = Vec::with_capacity(snapshot_capacity(self.runs.len(), max_items));
        for run_id in self.runs.keys().copied().take(max_items) {
            runs.push(run_id);
        }
        runs.sort();
        runs.into_boxed_slice()
    }

    fn pending_timer_snapshots(&self, max_items: usize) -> Box<[PendingTimerBoundarySnapshot]> {
        let mut timers =
            Vec::with_capacity(snapshot_capacity(self.pending_timers.len(), max_items));
        for (run_id, timer) in self.pending_timers.iter().take(max_items) {
            timers.push(PendingTimerBoundarySnapshot {
                run_id: *run_id,
                step: timer.step,
                kind: timer.kind,
                generation: timer.generation,
                deadline: timer.deadline,
            });
        }
        timers.sort_by_key(PendingTimerBoundarySnapshot::run_id);
        timers.into_boxed_slice()
    }

    fn pending_action_snapshots(&self, max_items: usize) -> Box<[PendingActionBoundarySnapshot]> {
        let mut actions =
            Vec::with_capacity(snapshot_capacity(self.pending_actions.len(), max_items));
        for (run_id, ticket) in self.pending_actions.iter().take(max_items) {
            actions.push(PendingActionBoundarySnapshot {
                run_id: *run_id,
                ticket: *ticket,
            });
        }
        actions.sort_by_key(PendingActionBoundarySnapshot::run_id);
        actions.into_boxed_slice()
    }

    fn pending_ask_snapshots(&self, max_items: usize) -> PendingAskSnapshotSet {
        let ask_count = self.pending_ask_count();
        let mut asks = Vec::with_capacity(snapshot_capacity(ask_count, max_items));
        for (run_id, state) in &self.runs {
            self.push_asking_step_snapshots(*run_id, state, max_items, &mut asks);
            if asks.len() >= max_items {
                break;
            }
        }
        asks.sort_by_key(|ask| (ask.run_id(), ask.ask_step()));
        PendingAskSnapshotSet {
            count: ask_count,
            items: asks.into_boxed_slice(),
        }
    }

    fn push_asking_step_snapshots(
        &self,
        run_id: RunId,
        state: &RunState,
        max_items: usize,
        asks: &mut Vec<PendingAskBoundarySnapshot>,
    ) {
        for step_index in 0..state.frame.step_count() {
            if asks.len() >= max_items {
                break;
            }
            let step = StepIdx::new(step_index);
            if frame_step_is_asking(state, step) {
                asks.push(PendingAskBoundarySnapshot {
                    run_id,
                    ask_step: step,
                    timeout: self.ask_timeout_snapshot(run_id, step),
                });
            }
        }
    }

    fn ask_timeout_snapshot(
        &self,
        run_id: RunId,
        ask_step: StepIdx,
    ) -> Option<PendingAskTimeoutBoundarySnapshot> {
        let timer = self.pending_timers.get(&run_id)?;
        if timer.kind == PendingTimerKind::Ask && timer.step == ask_step {
            Some(PendingAskTimeoutBoundarySnapshot {
                generation: timer.generation,
                deadline: timer.deadline,
            })
        } else {
            None
        }
    }
}

fn asking_step_count(state: &RunState) -> usize {
    let mut count = 0usize;
    for step_index in 0..state.frame.step_count() {
        if frame_step_is_asking(state, StepIdx::new(step_index)) {
            count = count.saturating_add(1);
        }
    }
    count
}

fn frame_step_is_asking(state: &RunState, step: StepIdx) -> bool {
    state.frame.step_state(step) == Ok(StepState::Asking)
}

fn snapshot_capacity(total: usize, max_items: usize) -> usize {
    total.min(max_items)
}