//! Step state machine proof kernel.
//!
//! Local-only step-state sanity kernel. This module defines a mirror `StepState`
//! and transition relation; it is not bound to production `vb_core::frame` state
//! transition code. Retained Verus checks are local model checks only and must
//! not be cited as production deductive evidence.
//!
//! # Module layout
//!
//! | Module       | Contents                                                    |
//! |--------------|-------------------------------------------------------------|
//! | `state`      | `StepState` enum (Verus `enum` / Cargo `#[non_exhaustive]`) |
//! | `transition` | `VALID_TRANSITIONS`, predicate & Result validators           |
//! | `spec`       | Verus spec functions (`spec_valid_transition`, …)           |
//! | `lemmas`     | Verus proof lemmas (terminal properties, …)                 |
//! | `tests`      | Unit tests (cargo only, cfg-not-verus)                       |
//! | `kani`       | Kani bounded model checking harness                          |

// ── Always-available sub-modules ────────────────────────────────────────────

pub mod state;
pub mod transition;

// ── Verus-only sub-modules (depend on vstd / verus!) ────────────────────────

#[cfg(verus_keep_ghost)]
pub mod lemmas;

#[cfg(verus_keep_ghost)]
pub mod spec;

// ── Cargo-only sub-modules ──────────────────────────────────────────────────

#[cfg(not(verus_keep_ghost))]
pub mod tests;

#[cfg(kani)]
pub mod kani;

// ── Re-exports ──────────────────────────────────────────────────────────────

// Core state type — available in both modes
pub use state::StepState;

// Verus specs — only in verus mode
#[cfg(verus_keep_ghost)]
pub use spec::{spec_is_terminal, spec_step_state_eq, spec_valid_transition};

// Verus lemmas — only in verus mode
#[cfg(verus_keep_ghost)]
pub use lemmas::*;

// Transition helpers — cargo only
#[cfg(not(verus_keep_ghost))]
pub use transition::{
    all_transitions_exhaustive, is_valid_transition, next_states, non_terminal_states,
    terminal_cannot_transition_to_non_terminal, terminal_states, validate_transition, VALID_TRANSITIONS,
};
