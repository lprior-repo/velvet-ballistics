//! Step state machine proof kernel.
//!
//! Local-only step-state sanity kernel. This file defines a mirror `StepState`
//! and transition relation; it is not bound to production `vb_core::frame` state
//! transition code. Retained Verus checks are local model checks only and must
//! not be cited as production deductive evidence.
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

// ── Verus verified layer ────────────────────────────────────────────────────
#[cfg(verus_keep_ghost)]
verus! {

// ── StepState enum ─────────────────────────────────────────────────────
#[derive(Clone, Copy)]
pub enum StepState {
    Pending,
    Running,
    Waiting,
    Asking,
    Succeeded,
    Failed,
    Cancelled,
    Skipped,
}

impl StepState {
    // Exec-mode equality used in exec fn bodies.
    pub fn eq(&self, other: &StepState) -> (result: bool) {
        match (self, other) {
            (StepState::Pending, StepState::Pending)
            | (StepState::Running, StepState::Running)
            | (StepState::Waiting, StepState::Waiting)
            | (StepState::Asking, StepState::Asking)
            | (StepState::Succeeded, StepState::Succeeded)
            | (StepState::Failed, StepState::Failed)
            | (StepState::Cancelled, StepState::Cancelled)
            | (StepState::Skipped, StepState::Skipped) => true,
            _ => false,
        }
    }
}

// ── Spec: step state equality ──────────────────────────────────────────
pub open spec fn spec_step_state_eq(a: StepState, b: StepState) -> bool {
    matches!((a, b), (StepState::Pending, StepState::Pending)
        | (StepState::Running, StepState::Running)
        | (StepState::Waiting, StepState::Waiting)
        | (StepState::Asking, StepState::Asking)
        | (StepState::Succeeded, StepState::Succeeded)
        | (StepState::Failed, StepState::Failed)
        | (StepState::Cancelled, StepState::Cancelled)
        | (StepState::Skipped, StepState::Skipped))
}

// ── Spec: transition relation (canonical mathematical definition) ─────
pub open spec fn spec_valid_transition(from: StepState, to: StepState) -> bool {
    from == to || (from == StepState::Pending && (to == StepState::Running || to
        == StepState::Succeeded || to == StepState::Failed || to == StepState::Cancelled || to
        == StepState::Skipped)) || (from == StepState::Running && (to == StepState::Succeeded || to
        == StepState::Failed || to == StepState::Waiting || to == StepState::Asking || to
        == StepState::Cancelled || to == StepState::Skipped)) || (from == StepState::Waiting && to
        == StepState::Running) || (from == StepState::Asking && to == StepState::Running) || (from
        == StepState::Succeeded && to == StepState::Succeeded) || (from == StepState::Failed && to
        == StepState::Failed) || (from == StepState::Cancelled && to == StepState::Cancelled) || (
    from == StepState::Skipped && to == StepState::Skipped)
}

// ── Spec: is_terminal ──────────────────────────────────────────────────
pub open spec fn spec_is_terminal(s: StepState) -> bool {
    matches!(
            s,
            StepState::Succeeded | StepState::Failed | StepState::Cancelled | StepState::Skipped
        )
}

// ── Lemma: terminal states have no non-terminal successors ───────────
proof fn lemma_terminal_has_no_non_terminal_successor(terminal: StepState)
    requires
        spec_is_terminal(terminal),
    ensures
        !spec_valid_transition(terminal, StepState::Pending) && !spec_valid_transition(
            terminal,
            StepState::Running,
        ) && !spec_valid_transition(terminal, StepState::Waiting) && !spec_valid_transition(
            terminal,
            StepState::Asking,
        ),
{
    // Only Succeeded, Failed, Cancelled, Skipped are terminal.
    // Each has only self-transition; none transition to Pending/Running/Waiting/Asking.
    assert(!spec_valid_transition(terminal, StepState::Pending));
    assert(!spec_valid_transition(terminal, StepState::Running));
    assert(!spec_valid_transition(terminal, StepState::Waiting));
    assert(!spec_valid_transition(terminal, StepState::Asking));
}

// ── Lemma: pending is non-terminal ────────────────────────────────────
proof fn lemma_pending_is_non_terminal()
    ensures
        !spec_is_terminal(StepState::Pending),
{
    assert(!spec_is_terminal(StepState::Pending));
}

// ── Lemma: running is non-terminal ────────────────────────────────────
proof fn lemma_running_is_non_terminal()
    ensures
        !spec_is_terminal(StepState::Running),
{
    assert(!spec_is_terminal(StepState::Running));
}

// ── Lemma: waiting is non-terminal ────────────────────────────────────
proof fn lemma_waiting_is_non_terminal()
    ensures
        !spec_is_terminal(StepState::Waiting),
{
    assert(!spec_is_terminal(StepState::Waiting));
}

// ── Lemma: asking is non-terminal ─────────────────────────────────────
proof fn lemma_asking_is_non_terminal()
    ensures
        !spec_is_terminal(StepState::Asking),
{
    assert(!spec_is_terminal(StepState::Asking));
}

// ── Lemma: succeeded is terminal ──────────────────────────────────────
proof fn lemma_succeeded_is_terminal()
    ensures
        spec_is_terminal(StepState::Succeeded),
{
    assert(spec_is_terminal(StepState::Succeeded));
}

// ── Lemma: failed is terminal ─────────────────────────────────────────
proof fn lemma_failed_is_terminal()
    ensures
        spec_is_terminal(StepState::Failed),
{
    assert(spec_is_terminal(StepState::Failed));
}

// ── Lemma: cancelled is terminal ──────────────────────────────────────
proof fn lemma_cancelled_is_terminal()
    ensures
        spec_is_terminal(StepState::Cancelled),
{
    assert(spec_is_terminal(StepState::Cancelled));
}

// ── Lemma: skipped is terminal ────────────────────────────────────────
proof fn lemma_skipped_is_terminal()
    ensures
        spec_is_terminal(StepState::Skipped),
{
    assert(spec_is_terminal(StepState::Skipped));
}

// ── Lemma: terminal self-transitions are always valid ─────────────────
proof fn lemma_terminal_self_transition_valid(terminal: StepState)
    requires
        spec_is_terminal(terminal),
    ensures
        spec_valid_transition(terminal, terminal),
{
    // Succeeded->Succeeded, Failed->Failed, Cancelled->Cancelled, Skipped->Skipped
    // are all listed in spec_valid_transition.
    assert(spec_valid_transition(terminal, terminal));
}

} // verus!
// ── Regular Rust implementation (non-Verus compilation) ─────────────────────
#[cfg(not(verus_keep_ghost))]
mod cargo_kernel {
    /// Step states for the proof kernel state machine.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[non_exhaustive]
    pub enum StepState {
        Pending,
        Running,
        Waiting,
        Asking,
        Succeeded,
        Failed,
        Cancelled,
        Skipped,
    }

    const VALID_TRANSITIONS: &[(StepState, StepState)] = &[
        (StepState::Pending, StepState::Running),
        (StepState::Pending, StepState::Succeeded),
        (StepState::Pending, StepState::Failed),
        (StepState::Pending, StepState::Cancelled),
        (StepState::Pending, StepState::Skipped),
        (StepState::Running, StepState::Succeeded),
        (StepState::Running, StepState::Failed),
        (StepState::Running, StepState::Waiting),
        (StepState::Running, StepState::Asking),
        (StepState::Running, StepState::Cancelled),
        (StepState::Running, StepState::Skipped),
        (StepState::Waiting, StepState::Running),
        (StepState::Asking, StepState::Running),
        (StepState::Succeeded, StepState::Succeeded),
        (StepState::Failed, StepState::Failed),
        (StepState::Cancelled, StepState::Cancelled),
        (StepState::Skipped, StepState::Skipped),
    ];

    impl StepState {
        #[must_use]
        pub fn is_terminal(&self) -> bool {
            matches!(
                self,
                StepState::Succeeded
                    | StepState::Failed
                    | StepState::Cancelled
                    | StepState::Skipped
            )
        }
    }

    pub fn is_valid_transition(from: StepState, to: StepState) -> bool {
        if from == to {
            return true;
        }
        for &(f, t) in VALID_TRANSITIONS {
            if f == from && t == to {
                return true;
            }
        }
        false
    }

    pub fn validate_transition(from: StepState, to: StepState) -> Result<StepState, &'static str> {
        if is_valid_transition(from, to) {
            Ok(to)
        } else {
            Err("invalid_state_transition")
        }
    }

    pub fn next_states(from: StepState) -> Vec<StepState> {
        let mut result = vec![from];
        for &(f, t) in VALID_TRANSITIONS {
            if f == from && !result.contains(&t) {
                result.push(t);
            }
        }
        result
    }

    pub fn terminal_states() -> Vec<StepState> {
        vec![
            StepState::Succeeded,
            StepState::Failed,
            StepState::Cancelled,
            StepState::Skipped,
        ]
    }

    pub fn non_terminal_states() -> Vec<StepState> {
        vec![
            StepState::Pending,
            StepState::Running,
            StepState::Waiting,
            StepState::Asking,
        ]
    }

    pub fn terminal_cannot_transition_to_non_terminal() -> bool {
        for terminal in terminal_states() {
            let next = next_states(terminal);
            if next.len() != 1 || next.first() != Some(&terminal) {
                return false;
            }
        }
        true
    }

    pub fn all_transitions_exhaustive() -> bool {
        for terminal in terminal_states() {
            if !terminal.is_terminal() {
                return false;
            }
        }
        for non_terminal in non_terminal_states() {
            if non_terminal.is_terminal() {
                return false;
            }
        }
        true
    }
}
#[cfg(not(verus_keep_ghost))]
pub use cargo_kernel::*;

// ── Tests (compiled in both modes) ──────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_valid_transitions() {
        let next = next_states(StepState::Pending);
        assert!(next.contains(&StepState::Pending));
        assert!(next.contains(&StepState::Running));
        assert!(next.contains(&StepState::Succeeded));
        assert!(next.contains(&StepState::Failed));
        assert!(next.contains(&StepState::Cancelled));
        assert!(next.contains(&StepState::Skipped));
        assert_eq!(next.len(), 6);
    }

    #[test]
    fn test_running_valid_transitions() {
        let next = next_states(StepState::Running);
        assert!(next.contains(&StepState::Running));
        assert!(next.contains(&StepState::Succeeded));
        assert!(next.contains(&StepState::Failed));
        assert!(next.contains(&StepState::Waiting));
        assert!(next.contains(&StepState::Asking));
        assert!(next.contains(&StepState::Cancelled));
        assert!(next.contains(&StepState::Skipped));
        assert_eq!(next.len(), 7);
    }

    #[test]
    fn test_all_idempotent_transitions() {
        for state in [
            StepState::Pending,
            StepState::Running,
            StepState::Waiting,
            StepState::Asking,
            StepState::Succeeded,
            StepState::Failed,
            StepState::Cancelled,
            StepState::Skipped,
        ] {
            assert!(is_valid_transition(state, state));
        }
    }

    #[test]
    fn test_waiting_to_running() {
        assert!(is_valid_transition(StepState::Waiting, StepState::Running));
    }

    #[test]
    fn test_asking_to_running() {
        assert!(is_valid_transition(StepState::Asking, StepState::Running));
    }

    #[test]
    fn test_terminal_self_transition() {
        assert!(is_valid_transition(
            StepState::Succeeded,
            StepState::Succeeded
        ));
        assert!(is_valid_transition(StepState::Failed, StepState::Failed));
        assert!(is_valid_transition(
            StepState::Cancelled,
            StepState::Cancelled
        ));
        assert!(is_valid_transition(StepState::Skipped, StepState::Skipped));
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(!is_valid_transition(StepState::Running, StepState::Pending));
        assert!(!is_valid_transition(
            StepState::Succeeded,
            StepState::Running
        ));
        assert!(!is_valid_transition(StepState::Failed, StepState::Running));
    }

    #[test]
    fn test_terminal_immutable() {
        assert!(terminal_cannot_transition_to_non_terminal());
    }

    #[test]
    fn test_terminal_states() {
        let terminals = terminal_states();
        assert_eq!(terminals.len(), 4);
        for t in terminals {
            assert!(t.is_terminal());
        }
    }

    #[test]
    fn test_non_terminal_states() {
        let non_terminals = non_terminal_states();
        assert_eq!(non_terminals.len(), 4);
        for t in non_terminals {
            assert!(!t.is_terminal());
        }
    }

    #[test]
    fn test_is_terminal_pending() {
        assert!(!StepState::Pending.is_terminal());
    }

    #[test]
    fn test_is_terminal_running() {
        assert!(!StepState::Running.is_terminal());
    }

    #[test]
    fn test_is_terminal_waiting() {
        assert!(!StepState::Waiting.is_terminal());
    }

    #[test]
    fn test_is_terminal_asking() {
        assert!(!StepState::Asking.is_terminal());
    }

    #[test]
    fn test_is_terminal_succeeded() {
        assert!(StepState::Succeeded.is_terminal());
    }

    #[test]
    fn test_is_terminal_failed() {
        assert!(StepState::Failed.is_terminal());
    }

    #[test]
    fn test_is_terminal_cancelled() {
        assert!(StepState::Cancelled.is_terminal());
    }

    #[test]
    fn test_is_terminal_skipped() {
        assert!(StepState::Skipped.is_terminal());
    }

    #[test]
    fn test_validate_transition_pending_to_running_ok() {
        assert_eq!(
            validate_transition(StepState::Pending, StepState::Running),
            Ok(StepState::Running)
        );
    }

    #[test]
    fn test_validate_transition_pending_to_succeeded_ok() {
        assert_eq!(
            validate_transition(StepState::Pending, StepState::Succeeded),
            Ok(StepState::Succeeded)
        );
    }

    #[test]
    fn test_validate_transition_pending_to_failed_ok() {
        assert_eq!(
            validate_transition(StepState::Pending, StepState::Failed),
            Ok(StepState::Failed)
        );
    }

    #[test]
    fn test_validate_transition_running_to_waiting_ok() {
        assert_eq!(
            validate_transition(StepState::Running, StepState::Waiting),
            Ok(StepState::Waiting)
        );
    }

    #[test]
    fn test_validate_transition_running_to_asking_ok() {
        assert_eq!(
            validate_transition(StepState::Running, StepState::Asking),
            Ok(StepState::Asking)
        );
    }

    #[test]
    fn test_validate_transition_waiting_to_running_ok() {
        assert_eq!(
            validate_transition(StepState::Waiting, StepState::Running),
            Ok(StepState::Running)
        );
    }

    #[test]
    fn test_validate_transition_asking_to_running_ok() {
        assert_eq!(
            validate_transition(StepState::Asking, StepState::Running),
            Ok(StepState::Running)
        );
    }

    #[test]
    fn test_validate_transition_terminal_idempotent() {
        assert_eq!(
            validate_transition(StepState::Succeeded, StepState::Succeeded),
            Ok(StepState::Succeeded)
        );
        assert_eq!(
            validate_transition(StepState::Failed, StepState::Failed),
            Ok(StepState::Failed)
        );
        assert_eq!(
            validate_transition(StepState::Cancelled, StepState::Cancelled),
            Ok(StepState::Cancelled)
        );
        assert_eq!(
            validate_transition(StepState::Skipped, StepState::Skipped),
            Ok(StepState::Skipped)
        );
    }

    #[test]
    fn test_validate_transition_invalid_pending_to_waiting() {
        assert_eq!(
            validate_transition(StepState::Pending, StepState::Waiting),
            Err("invalid_state_transition")
        );
    }

    #[test]
    fn test_validate_transition_invalid_running_to_pending() {
        assert_eq!(
            validate_transition(StepState::Running, StepState::Pending),
            Err("invalid_state_transition")
        );
    }

    #[test]
    fn test_validate_transition_invalid_waiting_to_succeeded() {
        assert_eq!(
            validate_transition(StepState::Waiting, StepState::Succeeded),
            Err("invalid_state_transition")
        );
    }

    #[test]
    fn test_validate_transition_invalid_asking_to_failed() {
        assert_eq!(
            validate_transition(StepState::Asking, StepState::Failed),
            Err("invalid_state_transition")
        );
    }

    #[test]
    fn test_all_transitions_exhaustive_returns_true() {
        assert!(all_transitions_exhaustive());
    }

    #[test]
    fn test_step_state_debug() {
        assert_eq!(format!("{:?}", StepState::Pending), "Pending");
        assert_eq!(format!("{:?}", StepState::Succeeded), "Succeeded");
        assert_eq!(format!("{:?}", StepState::Running), "Running");
    }

    #[test]
    fn test_step_state_clone_and_copy() {
        let state = StepState::Running;
        let _cloned = state;
        let _copied: StepState = state;
        assert_eq!(state, StepState::Running);
    }

    #[test]
    fn test_next_states_waiting() {
        let next = next_states(StepState::Waiting);
        assert!(next.contains(&StepState::Waiting));
        assert!(next.contains(&StepState::Running));
        assert_eq!(next.len(), 2);
    }

    #[test]
    fn test_next_states_asking() {
        let next = next_states(StepState::Asking);
        assert!(next.contains(&StepState::Asking));
        assert!(next.contains(&StepState::Running));
        assert_eq!(next.len(), 2);
    }

    #[test]
    fn test_next_states_terminal_unique() {
        for terminal in terminal_states() {
            let next = next_states(terminal);
            assert_eq!(next.len(), 1);
            assert_eq!(next.first().copied(), Some(terminal));
        }
    }
}

// ── Kani harness (runs under cfg(kani)) ─────────────────────────────────────
#[cfg(kani)]
mod kani_step_state_harnesses {
    use super::*;

    /// PO-KANI-006: Verify terminal_cannot_transition_to_non_terminal()
    /// returns true and that no terminal state may transition to Running.
    #[cfg(kani)]
    #[kani::proof]
    fn terminal_cannot_transition_to_non_terminal_kani() {
        let result = terminal_cannot_transition_to_non_terminal();
        assert!(
            result,
            "terminal_cannot_transition_to_non_terminal must hold (all terminal states absorbing)"
        );

        let t_raw: u8 = kani::any();
        let s_raw: u8 = kani::any();

        let terminals = terminal_states();
        let t = terminals[(t_raw as usize) % terminals.len()];
        let s = match s_raw % 8 {
            0 => StepState::Pending,
            1 => StepState::Running,
            2 => StepState::Succeeded,
            3 => StepState::Failed,
            4 => StepState::Skipped,
            5 => StepState::Waiting,
            6 => StepState::Asking,
            _ => StepState::Cancelled,
        };

        if t != s {
            let valid = is_valid_transition(t, s);
            assert!(!valid, "terminal->other transition must be invalid");
        } else {
            let valid = is_valid_transition(t, t);
            assert!(valid, "terminal->self must always be valid");
        }

        for terminal in terminal_states() {
            let next = next_states(terminal);
            assert!(next.len() == 1, "all terminal states are self-only");
            assert!(next.contains(&terminal));
        }
    }
}
