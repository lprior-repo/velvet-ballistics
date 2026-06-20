/// Local mirror of `vb_core::ids::RunId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub(super) struct RunId(u64);

impl RunId {
    pub(super) const fn new(value: u64) -> Self {
        Self(value)
    }

    pub(super) const fn get(self) -> u64 {
        self.0
    }
}

/// Local mirror of `vb_runtime::shard::timer::TimerTick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct TimerTick(u64);

impl TimerTick {
    pub(super) const fn new(value: u64) -> Self {
        Self(value)
    }

    pub(super) const fn get(self) -> u64 {
        self.0
    }
}

/// Local error mirror of `RuntimeError::TerminalRunsLruFull`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LruFullError {
    pub(super) capacity: usize,
}

/// Diagnostic counters — mirror of `LruRingCounters`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct LruRingCounters {
    pub(super) expired_evictions: u64,
    pub(super) capacity_overflows: u64,
}

/// Capacity matches `DEFAULT_MAX_TERMINAL_RUNS`.
pub(super) const CAPACITY: usize = 100_000;
/// TTL matches `DEFAULT_TERMINAL_RUNS_TTL_TICKS`.
pub(super) const TTL_TICKS: u64 = 86_400;
/// Tick at which the old half is expired but the young half is alive.
pub(super) const SWEEP_NOW: u64 = TTL_TICKS;
/// Inner-loop batch size for force-insert benches.
pub(super) const FORCE_INNER: usize = 4_096;
/// Inner-loop batch size for remove benches.
pub(super) const REMOVE_INNER: usize = 4_096;
