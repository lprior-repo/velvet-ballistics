#![forbid(unsafe_code)]
//! Read-only shard boundary snapshots for scheduler and observability callers.

use std::time::Instant;

use vb_core::action::ActionTicket;
use vb_core::frame::StepState;
use vb_core::ids::{RunId, StepIdx};

use crate::shard::types::{PendingTimerKind, RunState, Shard};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingTimerBoundarySnapshot {
    run_id: RunId,
    step: StepIdx,
    kind: PendingTimerKind,
    generation: u64,
    deadline: Instant,
}

impl PendingTimerBoundarySnapshot {
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }

    #[must_use]
    pub const fn step(&self) -> StepIdx {
        self.step
    }

    #[must_use]
    pub const fn kind(&self) -> PendingTimerKind {
        self.kind
    }

    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    #[must_use]
    pub const fn deadline(&self) -> Instant {
        self.deadline
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingActionBoundarySnapshot {
    run_id: RunId,
    ticket: ActionTicket,
}

impl PendingActionBoundarySnapshot {
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }

    #[must_use]
    pub const fn ticket(&self) -> ActionTicket {
        self.ticket
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingAskTimeoutBoundarySnapshot {
    generation: u64,
    deadline: Instant,
}

impl PendingAskTimeoutBoundarySnapshot {
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    #[must_use]
    pub const fn deadline(&self) -> Instant {
        self.deadline
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingAskBoundarySnapshot {
    run_id: RunId,
    ask_step: StepIdx,
    timeout: Option<PendingAskTimeoutBoundarySnapshot>,
}

impl PendingAskBoundarySnapshot {
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }

    #[must_use]
    pub const fn ask_step(&self) -> StepIdx {
        self.ask_step
    }

    #[must_use]
    pub const fn timeout(&self) -> Option<PendingAskTimeoutBoundarySnapshot> {
        self.timeout
    }

    #[must_use]
    pub fn generation(&self) -> Option<u64> {
        self.timeout.map(|timeout| timeout.generation())
    }

    #[must_use]
    pub fn deadline(&self) -> Option<Instant> {
        self.timeout.map(|timeout| timeout.deadline())
    }
}

struct PendingAskSnapshotSet {
    count: usize,
    items: Box<[PendingAskBoundarySnapshot]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardPendingBoundarySnapshot {
    shard_id: u32,
    command_queue_depth: usize,
    command_queue_capacity: usize,
    active_run_count: usize,
    pending_timer_count: usize,
    pending_action_count: usize,
    pending_ask_count: usize,
    active_runs: Box<[RunId]>,
    pending_timers: Box<[PendingTimerBoundarySnapshot]>,
    pending_actions: Box<[PendingActionBoundarySnapshot]>,
    pending_asks: Box<[PendingAskBoundarySnapshot]>,
    truncated: bool,
}

impl ShardPendingBoundarySnapshot {
    #[must_use]
    pub const fn shard_id(&self) -> u32 {
        self.shard_id
    }

    #[must_use]
    pub const fn command_queue_depth(&self) -> usize {
        self.command_queue_depth
    }

    #[must_use]
    pub const fn command_queue_capacity(&self) -> usize {
        self.command_queue_capacity
    }

    #[must_use]
    pub const fn active_run_count(&self) -> usize {
        self.active_run_count
    }

    #[must_use]
    pub const fn pending_timer_count(&self) -> usize {
        self.pending_timer_count
    }

    #[must_use]
    pub const fn pending_action_count(&self) -> usize {
        self.pending_action_count
    }

    #[must_use]
    pub const fn pending_ask_count(&self) -> usize {
        self.pending_ask_count
    }

    #[must_use]
    pub fn active_runs(&self) -> &[RunId] {
        &self.active_runs
    }

    #[must_use]
    pub fn pending_timers(&self) -> &[PendingTimerBoundarySnapshot] {
        &self.pending_timers
    }

    #[must_use]
    pub fn pending_actions(&self) -> &[PendingActionBoundarySnapshot] {
        &self.pending_actions
    }

    #[must_use]
    pub fn pending_asks(&self) -> &[PendingAskBoundarySnapshot] {
        &self.pending_asks
    }

    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.truncated
    }
}

impl Shard {
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
