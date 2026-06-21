//! Section 38 property test modules for vb_storage.
//!
//! Each submodule covers one named property from master plan §38:
//! - `digest_stability`     — §38 row "Digest stability"
//! - `layout_stability`     — §38 row "Layout stability"
//! - `for_each_ordering`    — §38 row "Ordering invariants"
//! - `bound_enforcement`    — §38 row "Bound enforcement"
//! - `state_machine`        — §38 row "State machine"
//! - `taint_safety`         — §38 row "Taint safety"

mod proptest_digest_stability;
mod proptest_layout_stability;
mod proptest_for_each_ordering;
mod proptest_bound_enforcement;
mod proptest_state_machine;
mod proptest_taint_safety;
