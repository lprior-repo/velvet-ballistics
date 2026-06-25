#![forbid(unsafe_code)]

//! Action ABI contract for the do/retry/on_error primitives.

mod contract;
mod error;
mod journal;
mod lifecycle_error;
mod name;
mod payload;
mod ticket;
mod validation;

#[cfg(test)]
use crate::capability::Capability;
#[cfg(test)]
use crate::frame::RunFrame;
#[cfg(test)]
use crate::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
#[cfg(test)]
use crate::value::SlotValue;

pub use contract::{
    ActionContract, Idempotency, IdempotencyViolation, RetryPolicy, RetrySafety, SideEffect,
};
pub use error::{ActionError, ActionFailure, ActionFailureCode, ActionResult};
pub use journal::ActionJournalEvent;
pub use lifecycle_error::{ActionFailureReport, ActionResumeRejection};
pub use name::{ActionName, ActionNameError};
pub use payload::{
    ActionInput, ActionOutcome, ActionOutput, ActionOutputReady, EncodedActionInputLen,
};
pub use ticket::{
    ActionTicket, action_ticket_has_valid_key, compute_action_idempotency_key, issue_action_ticket,
};
pub use validation::{
    propagate_action_taint, validate_action_dispatch, validate_action_outcome,
    validate_idempotency_key_ingredients, verify_idempotency,
};

#[cfg(test)]
#[path = "action/journal_tests.rs"]
mod journal_tests;

#[cfg(test)]
#[path = "action/tests.rs"]
mod tests;
