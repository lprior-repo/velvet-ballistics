use super::*;
use crate::test_harness::{iterator_state_in_slot, list_in_slot};
use vb_core::value_store::ValueStore;

fn fresh_frame() -> RunFrame {
    crate::test_harness::fresh_frame(4, 8)
}

include!("for_each_tests/for_each_core.rs");
include!("for_each_tests/for_each_phase22.rs");
include!("for_each_tests/for_each_rp016.rs");
