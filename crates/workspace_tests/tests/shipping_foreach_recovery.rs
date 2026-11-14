#![forbid(unsafe_code)]
//! Public live/persist/drop/reopen/recover/replay parity for owned ForEach.
//!
//! Every positive case begins with accepted owned inputs on Fjall and exercises
//! public cold recovery; the modules keep each case file bounded and reviewable.

#[path = "shipping_foreach_recovery/support.rs"]
mod support;

use std::sync::Arc;
use vb_core::action::{ActionFailure, ActionFailureCode};
use vb_core::value_store::ObjectField;
use vb_core::{
    CompiledNodeKind as K, ListId, RunId, SlotValue, StepIdx, StepState, Taint, ValueStore,
    WorkflowDigest,
};
use vb_runtime::recovery::RuntimeRecoveryProduct;
use vb_runtime::shard::{AskAnswer, PendingTimerKind};
use vb_storage::recovery::replay_owned_for_each;
use vb_storage::{FjallJournal, JournalEvent};
