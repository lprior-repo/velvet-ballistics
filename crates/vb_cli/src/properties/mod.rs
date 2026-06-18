//! Delta comparison types, computation, and property tests.
//!
//! Covers PO-004 (delta computation correctness) and PO-025 (SlotValue serialization determinism).
//!
//! Module layout:
//! - `delta_types` — `SlotDelta`, `TaintDelta`, `StateDelta`, `PcDelta` structs
//! - `delta_compute` — pure delta computation functions
//! - `prop_gen` — proptest generators (test-only)
//! - `delta_tests` — property-based and unit tests (test-only)

pub(crate) mod delta_compute;
pub(crate) mod delta_types;

#[cfg(test)]
mod delta_tests;
#[cfg(test)]
mod prop_gen;

// Re-exports for downstream test code that may want to reference delta types.
pub(crate) use delta_types::{PcDelta, SlotDelta, StateDelta, TaintDelta};

#[cfg(test)]
mod inline_test {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn simple(x in 0u32..10u32) {
            prop_assert!(x < 10);
        }
    }
}
