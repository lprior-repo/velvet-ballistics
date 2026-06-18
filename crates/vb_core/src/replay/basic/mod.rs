#![forbid(unsafe_code)]
//! Basic deterministic step handlers.
//!
//! Handles the simple node kinds: Nop, SetConst, Copy, EvalExpr, BuildObject,
//! BuildList, Finish, Jump, and the non-deterministic suspend passthrough.
//!
//! Also provides shared step-advance helpers (`advance_to_next`,
//! `increment_replay_executed`) used by this module and the dispatcher.
//!
//! Implementation is delegated to the `handlers` submodule.

pub(crate) use handlers::replay_step_kind;

// Re-export shared helpers for sibling modules (e.g. collect)
pub(crate) use handlers::{advance_to_next, increment_replay_executed};

mod handlers;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
