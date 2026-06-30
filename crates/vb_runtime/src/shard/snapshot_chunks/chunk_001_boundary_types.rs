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