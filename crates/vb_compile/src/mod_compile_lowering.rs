//! Private compile lowering module.

mod canonical_values;
mod list_input;
mod numeric_source_inputs;
mod part_01;
mod part_02;
mod part_03;
mod part_03_collect_spec;
mod part_04;
pub(crate) mod part_05;
mod part_06;
mod part_07;
mod part_08;
mod part_08_legacy;
mod part_09;
mod part_10;
mod part_11;
mod part_12;
mod part_13;
mod part_14;
mod skipped_outputs;
mod source_slot_reservation;

// vb-tkcc7: dead always-false-gated kani/ and kani_proofs/ harness dirs deleted outright
// (stale pre-refactor StepAst refs; no kani CI runner covers vb_compile).

#[cfg(test)]
mod tests;

// vb-qb2k2: the stale aggregate `property_tests` module is retired because no
// `property_tests.rs` or `property_tests/mod.rs` exists. Register the focused
// lowering property suites here so they can exercise private lowering seams
// without a silent disabled module declaration.
#[cfg(test)]
#[path = "mod_compile_lowering/canonical_reduce_proptests.rs"]
mod canonical_reduce_proptests;
#[cfg(test)]
#[path = "mod_compile_lowering/nested_body_tests/mod.rs"]
mod nested_body_tests;
#[cfg(test)]
#[path = "mod_compile_lowering/nested_repeat_proptests.rs"]
mod nested_repeat_proptests;
#[cfg(test)]
#[path = "proptest_body_dispatcher.rs"]
mod proptest_body_dispatcher;
#[cfg(test)]
#[path = "proptest_collect.rs"]
mod proptest_collect;
#[cfg(test)]
#[path = "proptest_error_parity.rs"]
mod proptest_error_parity;
#[cfg(test)]
#[path = "proptest_step_offset.rs"]
mod proptest_step_offset;
// vb-8eocj.9.1: rebound from dead part_03 caller API to the lexical
// N-step bounded_collect production lowering.
#[cfg(test)]
#[path = "mod_compile_lowering/vb_8mdp_7_collect.rs"]
mod vb_8mdp_7_collect;
#[cfg(test)]
#[path = "mod_compile_lowering/canonical_collect_scoped_tests.rs"]
mod canonical_collect_scoped_tests;

#[allow(unused_imports)]
pub(crate) use part_01::*;
// compile_source is needed by external integration tests.
pub use part_01::{author_value_to_owned, compile_source};
#[allow(unused_imports)]
pub(crate) use part_02::*;
#[allow(unused_imports)]
pub(crate) use part_03::*;
#[allow(unused_imports)]
pub(crate) use part_04::*;
#[allow(unused_imports)]
pub use part_05::*;
#[allow(unused_imports)]
pub use part_06::*;
#[allow(unused_imports)]
pub use part_07::*;
#[allow(unused_imports)]
pub(crate) use part_08::*;
#[allow(unused_imports)]
pub(crate) use part_08_legacy::*;
#[allow(unused_imports)]
pub(crate) use part_09::*;
#[allow(unused_imports)]
pub(crate) use part_10::*;
#[allow(unused_imports)]
pub(crate) use part_11::*;
#[allow(unused_imports)]
pub(crate) use part_12::*;
#[allow(unused_imports)]
pub(crate) use part_13::*;
#[allow(unused_imports)]
pub(crate) use part_14::*;
#[allow(unused_imports)]
pub(crate) use vb_core::workflow::{ErrorRouting, NodeContinuation, NodeOutput, NodeRouting};
