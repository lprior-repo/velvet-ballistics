#![forbid(unsafe_code)]

//! Bounded run-frame state for one shard-owned workflow run.
//!
//! This module owns the `RunFrame` value type, the `StepState` enum
//! driving its state machine, and the pure transition predicate
//! `is_valid_step_state_transition` shared by runtime validation and
//! proof harnesses.
//!
//! The implementation is split across focused chunks under `parts/` to
//! keep the front of the module (and each chunk) under the 300-line
//! repository source-length cap. `include!` keeps the chunks in the
//! same module scope so the public API and re-exports are unchanged:
//!
//! - `impl_001_construct` - `RunFrame::new` and `RunFrame::reinitialize`
//! - `impl_002_accessors` - read accessors, parallel-in-flight counter,
//!   `set_pc`, `increment_executed`
//! - `impl_003_slots_taints` - slot read/write, taint read/write,
//!   `find_handle_taint`, and the slot/taint/states snapshot helpers
//! - `impl_004_state_machines` - `mark_*` transitions, the private
//!   `write_step_state` and `validate_transition` helpers
//! - `initialized_slot_entry` - the free function used by
//!   `RunFrame::initialized_slots`
//!
//! Kani harnesses live alongside under the same directory and are
//! `include!`-d only when `cfg(kani)` is active so non-Kani builds do
//! not see them:
//!
//! - `kani_helpers` - shared `validate_transition_inline` and
//!   `step_state_from_u8` helpers used by every K-F proof
//! - `kani_f1_exhaustive` - PO-RUST-001-FRAME-KANI: 64-pair exhaustive
//!   `validate_transition` proof (one large proof, 547 lines, retained
//!   as a single harness for stability of the proof obligation)
//! - `kani_f2345_transitions` - K-F2 .. K-F5 transition proofs
//! - `kani_pc_proofs` - K-PC1 .. K-PC3 program-counter proofs
//! - `kani_slot_proofs` - K-S1 .. K-S2 slot read/write proofs
//! - `kani_parallel` - parallel-in-flight counter proofs (was the
//!   `parallel_in_flight_kani` module)

use crate::errors::{CoreError, CoreResult};
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};

/// Per-step execution state stored in the hot run frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub enum StepState {
    /// Step has not been entered.
    Pending,
    /// Step is currently executing.
    Running,
    /// Step completed successfully.
    Succeeded,
    /// Step failed.
    Failed,
    /// Step was skipped by control flow.
    Skipped,
    /// Step is suspended on a wait primitive.
    Waiting,
    /// Step is suspended on an ask primitive.
    Asking,
    /// Step was cancelled.
    Cancelled,
}

/// Pure transition predicate shared by runtime validation and proof harnesses.
#[must_use]
pub fn is_valid_step_state_transition(current: StepState, new: StepState) -> bool {
    if current == new {
        return true;
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
        (StepState::Succeeded, StepState::Pending),
        (StepState::Failed, StepState::Failed),
        (StepState::Cancelled, StepState::Cancelled),
        (StepState::Skipped, StepState::Skipped),
    ];
    for &(f, t) in VALID_TRANSITIONS {
        if f == current && t == new {
            return true;
        }
    }
    false
}

/// Runtime state for one workflow run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFrame {
    run_id: RunId,
    pc: StepIdx,
    executed: u64,
    step_count: u16,
    slot_count: u16,
    max_parallel_in_flight: u16,
    parallel_in_flight: u16,
    states: Box<[StepState]>,
    slots: Box<[Option<SlotValue>]>,
    taint: Box<[Taint]>,
}

include!("frame/parts/impl_001_construct.rs");
include!("frame/parts/impl_002_accessors.rs");
include!("frame/parts/impl_003_slots_taints.rs");
include!("frame/parts/impl_004_state_machines.rs");
include!("frame/parts/initialized_slot_entry.rs");

#[cfg(test)]
#[path = "frame/tests_and_verification.rs"]
mod tests_and_verification;

#[cfg(kani)]
include!("frame/parts/kani_helpers.rs");

#[cfg(kani)]
include!("frame/parts/kani_f1_exhaustive.rs");

#[cfg(kani)]
include!("frame/parts/kani_f2345_transitions.rs");

#[cfg(kani)]
include!("frame/parts/kani_pc_proofs.rs");

#[cfg(kani)]
include!("frame/parts/kani_slot_proofs.rs");

#[cfg(kani)]
include!("frame/parts/kani_parallel.rs");
