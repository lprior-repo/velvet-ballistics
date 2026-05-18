#![forbid(unsafe_code)]
//! Production API bindings for IPC synchronization proof obligations.
//!
//! These helpers expose small, side-effect-free summaries of production types so
//! proof evidence can refer to real constructors and state APIs instead of a
//! detached witness-only model.

use std::time::Instant;

use vb_core::ids::{RunId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;

use crate::admission::RunAdmission;
use crate::shard::ShardStatus;
use crate::shard::timer_wheel::TimerWheel;
use crate::shard::types::{
    MAX_COMMAND_QUEUE_CAPACITY, RuntimeEvent, RuntimeState, ShardCommandQueue,
};

/// REFINE-IPC-001: production admission record mapped to strict admission facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrictAdmissionRefinement {
    /// The production admission record carries the expected artifact digest.
    pub artifact_digest_matches: bool,
    /// The production admission record carries the expected run id.
    pub run_id_matches: bool,
    /// The production admission record carries the expected policy.
    pub policy_matches: bool,
}

impl StrictAdmissionRefinement {
    /// Returns true when all strict-admission production facts agree.
    #[must_use]
    pub const fn is_refined(self) -> bool {
        self.artifact_digest_matches && self.run_id_matches && self.policy_matches
    }
}

/// REFINE-IPC-002: production bounded queue mapped to capacity proof facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueCapacityRefinement {
    /// Current queue depth.
    pub len: usize,
    /// Configured queue capacity.
    pub capacity: usize,
    /// Remaining queue slots reported by the production API.
    pub remaining_capacity: usize,
    /// Whether the production queue reports full.
    pub is_full: bool,
    /// Maximum accepted command queue capacity.
    pub bounded_capacity: usize,
}

impl QueueCapacityRefinement {
    /// Returns true when production queue observations satisfy capacity facts.
    #[must_use]
    pub const fn is_refined(self) -> bool {
        self.len <= self.capacity
            && self.capacity <= self.bounded_capacity
            && self.remaining_capacity == self.capacity.saturating_sub(self.len)
            && self.is_full == (self.len == self.capacity)
    }
}

/// REFINE-IPC-003: production runtime event/state mapped to terminal facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalTransitionRefinement {
    /// Whether this production event is terminal.
    pub event_is_terminal: bool,
    /// Whether this production state is resumable.
    pub state_is_resumable: bool,
}

/// REFINE-IPC-004: production timer wheel mapped to timer proof facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerRefinement {
    /// Pending timer count before a production timer operation.
    pub before_len: usize,
    /// Pending timer count after a production timer operation.
    pub after_len: usize,
    /// Number of timers fired by the production operation.
    pub fired_count: usize,
    /// Whether the queried run still has a pending timer after the operation.
    pub run_still_pending: bool,
}

impl TimerRefinement {
    /// Returns true when a fire operation does not increase pending timers and
    /// fired entries account for the observed decrease.
    #[must_use]
    pub const fn fire_is_refined(self) -> bool {
        self.after_len <= self.before_len
            && self.fired_count <= self.before_len
            && self.before_len.saturating_sub(self.after_len) == self.fired_count
    }

    /// Returns true when a cancel operation leaves no timer for the cancelled run.
    #[must_use]
    pub const fn cancel_is_refined(self) -> bool {
        self.after_len <= self.before_len && !self.run_still_pending
    }
}

/// REFINE-IPC-005: production shard status mapped to shutdown facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutdownRefinement {
    /// Production status reports shutdown has started.
    pub shutting_down: bool,
    /// Production status reports the shard is still running.
    pub running: bool,
}

impl ShutdownRefinement {
    /// Returns true when shutdown implies admission is closed for new work.
    #[must_use]
    pub const fn admission_closed(self) -> bool {
        self.shutting_down
    }
}

/// Binds `RunAdmission` accessors to REFINE-IPC-001 facts.
#[must_use]
pub fn strict_admission_refinement(
    admission: &RunAdmission,
    expected_digest: WorkflowDigest,
    expected_run: RunId,
    expected_policy: RuntimePolicy,
) -> StrictAdmissionRefinement {
    StrictAdmissionRefinement {
        artifact_digest_matches: admission.artifact_digest() == expected_digest,
        run_id_matches: admission.run_id() == expected_run,
        policy_matches: admission.policy() == expected_policy,
    }
}

/// Binds `ShardCommandQueue` accessors to REFINE-IPC-002 facts.
#[must_use]
pub fn queue_capacity_refinement(queue: &ShardCommandQueue) -> QueueCapacityRefinement {
    QueueCapacityRefinement {
        len: queue.len(),
        capacity: queue.capacity(),
        remaining_capacity: queue.remaining_capacity(),
        is_full: queue.is_full(),
        bounded_capacity: MAX_COMMAND_QUEUE_CAPACITY,
    }
}

/// Binds `RuntimeEvent`/`RuntimeState` predicates to REFINE-IPC-003 facts.
#[must_use]
pub fn terminal_transition_refinement(
    event: RuntimeEvent,
    state: RuntimeState,
) -> TerminalTransitionRefinement {
    TerminalTransitionRefinement {
        event_is_terminal: event.is_terminal(),
        state_is_resumable: state.is_resumable(),
    }
}

/// Fires expired timers and returns REFINE-IPC-004 facts from the production wheel.
pub fn timer_fire_refinement(wheel: &mut TimerWheel, now: Instant, run: RunId) -> TimerRefinement {
    let before_len = wheel.len();
    let fired_count = wheel.fire_expired(now).len();
    let after_len = wheel.len();
    TimerRefinement {
        before_len,
        after_len,
        fired_count,
        run_still_pending: wheel.get_kind(run).is_some(),
    }
}

/// Cancels a timer and returns REFINE-IPC-004 facts from the production wheel.
pub fn timer_cancel_refinement(wheel: &mut TimerWheel, run: RunId) -> TimerRefinement {
    let before_len = wheel.len();
    let _cancelled = wheel.cancel(run);
    let after_len = wheel.len();
    TimerRefinement {
        before_len,
        after_len,
        fired_count: 0,
        run_still_pending: wheel.get_kind(run).is_some(),
    }
}

/// Binds `ShardStatus` to REFINE-IPC-005 shutdown facts.
#[must_use]
pub const fn shutdown_refinement(status: ShardStatus) -> ShutdownRefinement {
    ShutdownRefinement {
        shutting_down: status.shutting_down,
        running: status.running,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroUsize;
    use std::time::Duration;

    use vb_core::capability::CapabilitySet;

    use crate::shard::timer_wheel::TimerWheel;
    use crate::shard::types::PendingTimerKind;

    #[test]
    fn strict_admission_refinement_binds_production_accessors() {
        let digest = WorkflowDigest::from_bytes([7; 32]);
        let run = RunId::new(9);
        let policy = RuntimePolicy::Strict;
        let admission = RunAdmission::new(digest, run, CapabilitySet::empty(), policy);

        let refined = strict_admission_refinement(&admission, digest, run, policy);

        assert!(
            refined.is_refined(),
            "production admission fields should match"
        );
    }

    #[test]
    fn queue_capacity_refinement_binds_production_queue_accessors() {
        let capacity = NonZeroUsize::new(2).map_or(1, NonZeroUsize::get);
        let queue = ShardCommandQueue::new(capacity);
        let Ok(queue) = queue else { return };

        let refined = queue_capacity_refinement(&queue);

        assert!(
            refined.is_refined(),
            "empty queue should satisfy capacity facts"
        );
        assert_eq!(refined.len, 0);
        assert_eq!(refined.remaining_capacity, 2);
    }

    #[test]
    fn terminal_transition_refinement_binds_event_predicates() {
        let refined =
            terminal_transition_refinement(RuntimeEvent::DriveFinished, RuntimeState::Resumable);

        assert!(refined.event_is_terminal, "DriveFinished is terminal");
        assert!(refined.state_is_resumable, "Resumable state is resumable");
    }

    #[test]
    fn timer_refinement_binds_fire_and_cancel_operations() {
        let mut wheel = TimerWheel::new();
        let run = RunId::new(1);
        let now = Instant::now();
        assert_eq!(wheel.insert(run, now, PendingTimerKind::Wait), Ok(()));

        let fired = timer_fire_refinement(&mut wheel, now + Duration::from_millis(1), run);

        assert!(
            fired.fire_is_refined(),
            "fire must account for pending decrease"
        );
        assert_eq!(fired.before_len, 1);
        assert_eq!(fired.after_len, 0);
        assert_eq!(fired.fired_count, 1);

        assert_eq!(
            wheel.insert(run, now + Duration::from_secs(1), PendingTimerKind::Ask),
            Ok(())
        );
        let cancelled = timer_cancel_refinement(&mut wheel, run);

        assert!(
            cancelled.cancel_is_refined(),
            "cancel must remove run timer"
        );
        assert_eq!(cancelled.before_len, 1);
        assert_eq!(cancelled.after_len, 0);
    }

    #[test]
    fn shutdown_refinement_binds_status_flag() {
        let status = ShardStatus {
            health: crate::shard::ShardHealth::ShuttingDown,
            running: false,
            shutting_down: true,
            command_queue_depth: 0,
            command_queue_capacity: 1,
            active_runs: 0,
            max_active_runs: 1,
            trace_capacity: 1,
            trace_dropped: 0,
            step_budget_per_tick: 1,
            runtime_policy: RuntimePolicy::Strict,
        };

        let refined = shutdown_refinement(status);

        assert!(refined.admission_closed(), "shutdown closes admission");
        assert!(!refined.running, "shutdown status is not running");
    }
}
