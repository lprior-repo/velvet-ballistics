//! Test helpers for TimerWheel inspection
//!
//! These helpers expose internal state for testing purposes only.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::time::Instant;

use vb_core::ids::RunId;

use crate::shard::types::PendingTimerKind;

/// Extension trait to expose internal state for testing
pub trait TimerWheelInspect {
    fn by_deadline_contains(&self, deadline: Instant, run: RunId) -> bool;
    fn by_run_contains(&self, run: RunId) -> bool;
    fn by_deadline_len(&self) -> usize;
    fn by_run_len(&self) -> usize;
    fn by_deadline_is_empty(&self) -> bool;
    fn by_run_is_empty(&self) -> bool;
    fn deadline_for_run(&self, run: RunId) -> Option<Instant>;
}

impl TimerWheelInspect for crate::shard::timer_wheel::TimerWheel {
    fn by_deadline_contains(&self, deadline: Instant, run: RunId) -> bool {
        self.by_deadline
            .get(&deadline)
            .map(|entries| entries.iter().any(|e| e.run == run))
            .unwrap_or(false)
    }

    fn by_run_contains(&self, run: RunId) -> bool {
        self.by_run.contains_key(&run)
    }

    fn by_deadline_len(&self) -> usize {
        self.by_deadline.len()
    }

    fn by_run_len(&self) -> usize {
        self.by_run.len()
    }

    fn by_deadline_is_empty(&self) -> bool {
        self.by_deadline.is_empty()
    }

    fn by_run_is_empty(&self) -> bool {
        self.by_run.is_empty()
    }

    fn deadline_for_run(&self, run: RunId) -> Option<Instant> {
        self.by_run.get(&run).map(|(deadline, _)| *deadline)
    }
}
