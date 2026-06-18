//! `RunFrame` struct definition.
//!
//! This file contains ONLY the struct definition and its doc comment.
//! All method impl blocks live in focused submodules:
//! - `lifecycle` — construction and reinitialization
//! - `accessors` — const fn getters
//! - `parallel` — in-flight tracking
//! - `pc` — program counter and execution counter
//! - `slots` — slot I/O and taint
//! - `transitions` — step state machine transitions

use crate::ids::{RunId, StepIdx};
use crate::value::{SlotValue, Taint};

use super::step_state::StepState;

/// Runtime state for one workflow run.
///
/// Fields are visible only inside the `frame` module tree so focused sibling
/// modules can share the data while keeping the struct opaque outside `frame`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFrame {
    pub(super) run_id: RunId,
    pub(super) pc: StepIdx,
    pub(super) executed: u64,
    pub(super) step_count: u16,
    pub(super) slot_count: u16,
    pub(super) max_parallel_in_flight: u16,
    pub(super) parallel_in_flight: u16,
    pub(super) states: Box<[StepState]>,
    pub(super) slots: Box<[Option<SlotValue>]>,
    pub(super) taint: Box<[Taint]>,
}
