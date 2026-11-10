#![forbid(unsafe_code)]

//! Type definitions for the runtime engine.
//!
//! Exports evidence collection, error types, retry policy, and signals.

use vb_core::action::{ActionError, ActionTicket};
use vb_core::errors::EngineError;
use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};

use crate::primitives::collect::CollectPaginationState;

const REQUIRED_COLLECT_SLOT_EXTRA: &str = "collect SlotWritten extra";
const REQUIRED_STEP_STARTED: &str = "evidence StepStarted event";
const REQUIRED_STEP_SUCCEEDED: &str = "evidence StepSucceeded event";
const REQUIRED_SLOT_WRITTEN: &str = "evidence SlotWritten event";

include!("types/evidence.rs");

include!("types/errors.rs");

include!("types/policy.rs");

#[cfg(test)]
mod tests;
