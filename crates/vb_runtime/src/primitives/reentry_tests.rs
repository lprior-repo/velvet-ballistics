//! Body-re-entry unit tests for loop primitives.
//!
//! These tests verify that loop primitives correctly handle re-entry
//! into body steps after a previous iteration has completed.
//!
//! Bug: When a loop body step completes (Succeeded) and control returns
//! to the loop primitive, the step is still in Succeeded state.
//! The missing Succeeded→Pending transition prevents proper re-entry.

use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;

use crate::primitives::collect::{CollectStates, collect_next, collect_page, collect_start};
use crate::primitives::for_each::{for_each_next, for_each_start};
use crate::primitives::reduce::{reduce_next, reduce_start};
use crate::primitives::repeat::{repeat_attempt, repeat_check, repeat_start};
use crate::test_harness::list_in_slot;

fn fresh_frame() -> RunFrame {
    crate::test_harness::fresh_frame(6, 10)
}

include!("reentry_tests/basic_reentry_tests.rs");
include!("reentry_tests/gwt_reentry_tests.rs");
include!("reentry_tests/proptest_reentry_tests.rs");
