//! Kani bounded model checking harness for the step state machine.
//!
//! PO-KANI-006: Verify terminal_cannot_transition_to_non_terminal()
//! returns true and that no terminal state may transition to Running.

#[cfg(kani)]
use super::state::StepState;

#[cfg(kani)]
use super::transition::{
    is_valid_transition, next_states, terminal_cannot_transition_to_non_terminal,
    terminal_states,
};

#[cfg(kani)]
mod kani_step_state_harnesses {
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

#[cfg(kani)]
pub use kani_step_state_harnesses::*;
