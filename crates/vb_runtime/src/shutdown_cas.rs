#![forbid(unsafe_code)]
//! CONC-01: atomic compare-and-swap shutdown state machine.
//!
//! `ShutdownState` exposes a three-state transition machine
//! (`IDLE → SHUTTING_DOWN → SHUTDOWN`) backed by an `AtomicU8`. Concurrent
//! callers race on `try_begin_shutdown`, which performs a
//! `compare_exchange(IDLE, SHUTTING_DOWN)`. Exactly one caller observes the
//! `Begin` outcome; every other concurrent caller observes `AlreadyShuttingDown`.
//! This eliminates the TOCTOU window between "check shutting_down" and "set
//! shutting_down" that exists in the previous `bool`-flag shutdown path.
//!
//! The atomic is `AcqRel` so that all prior writes by the thread that
//! successfully transitioned to `SHUTTING_DOWN` are visible to threads that
//! subsequently observe `SHUTDOWN`. The previous `bool` field used a plain
//! `SeqCst` store which provided the same guarantee but with a coarser
//! memory footprint and no compare-and-swap primitive.

use std::sync::atomic::{AtomicU8, Ordering};

/// Possible shutdown states for `ShutdownState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ShutdownPhase {
    /// Shard / runtime has not yet begun shutdown. Initial state.
    Idle = 0,
    /// A caller has successfully observed the IDLE state and begun shutdown.
    /// Subsequent callers will observe `AlreadyShuttingDown`.
    ShuttingDown = 1,
    /// Shutdown is complete. The terminal state; no further transitions.
    Shutdown = 2,
}

impl ShutdownPhase {
    /// Returns the wire-byte representation.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Idle => 0,
            Self::ShuttingDown => 1,
            Self::Shutdown => 2,
        }
    }

    /// Returns the phase for a wire byte, or `None` if the byte is invalid.
    #[must_use]
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Idle),
            1 => Some(Self::ShuttingDown),
            2 => Some(Self::Shutdown),
            _ => None,
        }
    }
}

/// Outcome of a `try_begin_shutdown` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownTransition {
    /// Caller successfully transitioned the state from `Idle` to `ShuttingDown`.
    /// The caller is responsible for draining pending work and then calling
    /// `complete_shutdown`.
    Begin,
    /// Another caller has already begun shutdown; the current state is
    /// `ShuttingDown`. The caller must NOT begin draining work again.
    AlreadyShuttingDown,
    /// Shutdown has already completed. The state is `Shutdown`.
    AlreadyShutdown,
}

/// Atomic shutdown state machine.
#[derive(Debug)]
pub struct ShutdownState {
    state: AtomicU8,
}

impl ShutdownState {
    /// Builds a new atomic shutdown state machine in `Idle`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(ShutdownPhase::Idle.as_u8()),
        }
    }

    /// Returns the current phase as observed by this thread.
    #[must_use]
    pub fn phase(&self) -> ShutdownPhase {
        let raw = self.state.load(Ordering::Acquire);
        // The atomic is only ever written with the three valid bytes, so
        // an unexpected byte is a violation of the invariant rather than
        // a user-facing condition we have to model.
        ShutdownPhase::from_u8(raw).unwrap_or(ShutdownPhase::Idle)
    }

    /// Attempts to transition the state from `Idle` to `ShuttingDown`.
    ///
    /// Uses `compare_exchange` so exactly one concurrent caller observes
    /// `Begin`. All other concurrent callers observe `AlreadyShuttingDown`
    /// or `AlreadyShutdown` depending on whether the winner has already
    /// advanced to `Shutdown`.
    pub fn try_begin_shutdown(&self) -> ShutdownTransition {
        match self.state.compare_exchange(
            ShutdownPhase::Idle.as_u8(),
            ShutdownPhase::ShuttingDown.as_u8(),
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => ShutdownTransition::Begin,
            Err(current) => match ShutdownPhase::from_u8(current) {
                Some(ShutdownPhase::ShuttingDown) => ShutdownTransition::AlreadyShuttingDown,
                Some(ShutdownPhase::Shutdown) => ShutdownTransition::AlreadyShutdown,
                // Impossible: compare_exchange observed a non-{Idle,ShuttingDown,
                // Shutdown} byte. Treat as already shutting down so the caller
                // does not race.
                Some(ShutdownPhase::Idle) | None => ShutdownTransition::AlreadyShuttingDown,
            },
        }
    }

    /// Transitions the state from `ShuttingDown` to `Shutdown`. Returns
    /// `true` if the caller performed the transition, `false` if the state
    /// was already `Shutdown` or is still `Idle` (i.e., `try_begin_shutdown`
    /// was not called first).
    pub fn complete_shutdown(&self) -> bool {
        self.state
            .compare_exchange(
                ShutdownPhase::ShuttingDown.as_u8(),
                ShutdownPhase::Shutdown.as_u8(),
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    /// Returns `true` if the state has advanced past `Idle`.
    #[must_use]
    pub fn is_shutting_down_or_complete(&self) -> bool {
        self.phase() != ShutdownPhase::Idle
    }
}

impl Default for ShutdownState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn new_state_is_idle() {
        let state = ShutdownState::new();
        assert_eq!(state.phase(), ShutdownPhase::Idle);
        assert!(!state.is_shutting_down_or_complete());
    }

    #[test]
    fn first_caller_observes_begin() {
        let state = ShutdownState::new();
        assert_eq!(state.try_begin_shutdown(), ShutdownTransition::Begin);
        assert_eq!(state.phase(), ShutdownPhase::ShuttingDown);
        assert!(state.is_shutting_down_or_complete());
    }

    #[test]
    fn second_caller_observes_already_shutting_down() {
        let state = ShutdownState::new();
        assert_eq!(state.try_begin_shutdown(), ShutdownTransition::Begin);
        assert_eq!(
            state.try_begin_shutdown(),
            ShutdownTransition::AlreadyShuttingDown
        );
    }

    #[test]
    fn complete_shutdown_after_begin() {
        let state = ShutdownState::new();
        assert_eq!(state.try_begin_shutdown(), ShutdownTransition::Begin);
        assert!(state.complete_shutdown());
        assert_eq!(state.phase(), ShutdownPhase::Shutdown);
    }

    #[test]
    fn complete_shutdown_without_begin_is_no_op() {
        let state = ShutdownState::new();
        assert!(!state.complete_shutdown());
        assert_eq!(state.phase(), ShutdownPhase::Idle);
    }

    #[test]
    fn double_complete_shutdown_is_idempotent() {
        let state = ShutdownState::new();
        assert_eq!(state.try_begin_shutdown(), ShutdownTransition::Begin);
        assert!(state.complete_shutdown());
        assert!(!state.complete_shutdown());
    }

    #[test]
    fn eight_concurrent_callers_see_exactly_one_begin() {
        // Spawn 8 threads that race on try_begin_shutdown. Exactly one of
        // them must observe Begin; the rest must observe AlreadyShuttingDown.
        let state = Arc::new(ShutdownState::new());
        let mut handles = Vec::with_capacity(8);
        for _ in 0..8 {
            let state = Arc::clone(&state);
            handles.push(thread::spawn(move || state.try_begin_shutdown()));
        }
        let outcomes: Vec<ShutdownTransition> = handles
            .into_iter()
            .map(|h| h.join().expect("thread join"))
            .collect();
        let begin_count = outcomes
            .iter()
            .filter(|o| **o == ShutdownTransition::Begin)
            .count();
        assert_eq!(begin_count, 1, "exactly one caller must observe Begin");
        for outcome in outcomes.iter().filter(|o| **o != ShutdownTransition::Begin) {
            assert_eq!(*outcome, ShutdownTransition::AlreadyShuttingDown);
        }
    }

    #[test]
    fn eight_concurrent_callers_after_complete_see_already_shutdown() {
        // After complete_shutdown, any subsequent try_begin_shutdown must
        // observe AlreadyShutdown (not Begin).
        let state = Arc::new(ShutdownState::new());
        assert_eq!(state.try_begin_shutdown(), ShutdownTransition::Begin);
        assert!(state.complete_shutdown());

        let mut handles = Vec::with_capacity(8);
        for _ in 0..8 {
            let state = Arc::clone(&state);
            handles.push(thread::spawn(move || state.try_begin_shutdown()));
        }
        let outcomes: Vec<ShutdownTransition> = handles
            .into_iter()
            .map(|h| h.join().expect("thread join"))
            .collect();
        for outcome in outcomes {
            assert_eq!(outcome, ShutdownTransition::AlreadyShutdown);
        }
    }

    #[test]
    fn phase_roundtrips_via_u8() {
        for phase in [
            ShutdownPhase::Idle,
            ShutdownPhase::ShuttingDown,
            ShutdownPhase::Shutdown,
        ] {
            assert_eq!(ShutdownPhase::from_u8(phase.as_u8()), Some(phase));
        }
        assert_eq!(ShutdownPhase::from_u8(99), None);
    }

    /// Edge case (tier-a-6-013 deep validation): 100 concurrent callers
    /// race on `try_begin_shutdown`. Exactly one must observe `Begin`; the
    /// other 99 must observe `AlreadyShuttingDown`. This is the core
    /// TOCTOU-elimination contract for the atomic compare-and-swap path.
    #[test]
    fn hundred_concurrent_callers_see_exactly_one_begin() {
        const N: usize = 100;
        let state = Arc::new(ShutdownState::new());
        let mut handles = Vec::with_capacity(N);
        for _ in 0..N {
            let state_clone = Arc::clone(&state);
            handles.push(thread::spawn(move || state_clone.try_begin_shutdown()));
        }
        let outcomes: Vec<ShutdownTransition> = handles
            .into_iter()
            .map(|h| h.join().expect("thread join"))
            .collect();
        let begin_count = outcomes
            .iter()
            .filter(|o| **o == ShutdownTransition::Begin)
            .count();
        assert_eq!(
            begin_count, 1,
            "exactly one of {N} concurrent callers must observe Begin"
        );
        for outcome in outcomes
            .iter()
            .filter(|o| **o != ShutdownTransition::Begin)
        {
            assert_eq!(
                *outcome,
                ShutdownTransition::AlreadyShuttingDown,
                "every loser must observe AlreadyShuttingDown, not AlreadyShutdown"
            );
        }
    }

    /// Edge case: 100 concurrent callers arriving after `complete_shutdown`
    /// must all observe `AlreadyShutdown`. No caller may observe `Begin`
    /// or `AlreadyShuttingDown` once the state has advanced to terminal.
    #[test]
    fn hundred_concurrent_callers_after_complete_see_already_shutdown() {
        const N: usize = 100;
        let state = Arc::new(ShutdownState::new());
        assert_eq!(state.try_begin_shutdown(), ShutdownTransition::Begin);
        assert!(state.complete_shutdown());

        let mut handles = Vec::with_capacity(N);
        for _ in 0..N {
            let state_clone = Arc::clone(&state);
            handles.push(thread::spawn(move || state_clone.try_begin_shutdown()));
        }
        let outcomes: Vec<ShutdownTransition> = handles
            .into_iter()
            .map(|h| h.join().expect("thread join"))
            .collect();
        for outcome in outcomes {
            assert_eq!(
                outcome,
                ShutdownTransition::AlreadyShutdown,
                "post-terminal callers must observe AlreadyShutdown"
            );
        }
    }
}
