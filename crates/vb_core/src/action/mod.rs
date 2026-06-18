#![forbid(unsafe_code)]

//! Action ABI contract for the do/retry/on_error primitives.

pub mod classification;
pub mod error;
pub mod failure;
pub mod journal;
pub mod key;
pub mod model;
pub mod taint;
pub mod validate;

#[cfg(verus)]
mod proof;

// Re-export ids and value types so tests using `use super::*` can access them.
// These were previously at the top of the monolithic action.rs file.
#[allow(unused_imports)]
pub(crate) use crate::capability::Capability;
#[allow(unused_imports)]
pub(crate) use crate::frame::RunFrame;
#[allow(unused_imports)]
pub(crate) use crate::ids::{ActionId, BlobId, RunId, SeqNo, SlotIdx, StepIdx};
#[allow(unused_imports)]
pub(crate) use crate::value::{SlotValue, Taint};
#[allow(unused_imports)]
pub(crate) use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
pub(crate) use std::hash::Hash;
#[allow(unused_imports)]
pub(crate) use thiserror::Error;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

// ───────────────────────────────────────────────────────────────────────────
// Re-exports — public API surface
// ───────────────────────────────────────────────────────────────────────────

// Classification
pub use classification::{
    ActionName, ActionNameError, Idempotency, IdempotencyViolation, MockMarker, RetryPolicy,
    RetrySafety, SideEffect, is_idempotent, is_retry_safe, is_retry_safe_with_key,
};

// Error
pub use error::ActionError;

// Failure
pub use failure::{ActionFailure, ActionFailureCode};

// Model
pub use model::{
    ActionContract, ActionInput, ActionOutcome, ActionOutput, ActionOutputReady, ActionTicket,
};

/// Result alias for action operations.
pub type ActionResult<T> = Result<T, ActionError>;

// Key
pub use key::{action_ticket_has_valid_key, compute_action_idempotency_key};

// Taint
pub use taint::propagate_action_taint;

// Validate
pub use validate::{
    issue_action_ticket, validate_action_dispatch, validate_action_outcome,
    validate_idempotency_key_ingredients, verify_idempotency,
};

// Journal
pub use journal::ActionJournalEvent;
