//! Private compile lowering module.

mod part_01;
mod part_02;
mod part_03;
mod part_04;
pub(crate) mod part_05;
mod part_06;
mod part_07;
mod part_08;
mod part_09;
mod part_10;
mod part_11;
mod part_12;
mod part_13;
mod part_14;

// Kani harnesses for ForEach digest coverage (PO-K-FE-01 through PO-K-FE-10).
// Bead: vb-xi2f.28 | State: 5 (proof-writer)
// Placed here to access pub(super) items from part_05 (canonical_digest, digest_step_primitive).
#[cfg(kani)]
mod kani_proofs;

// Kani harnesses for choose lowering fix (PO-KANI-001 through PO-KANI-013).
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
#[cfg(kani)]
mod kani;

#[cfg(test)]
mod tests;

// vb-qb2k2: the stale aggregate `property_tests` module is retired because no
// `property_tests.rs` or `property_tests/mod.rs` exists. Register the focused
// lowering property suites here so they can exercise private lowering seams
// without a silent disabled module declaration.
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

#[allow(unused_imports)]
pub(crate) use part_01::*;
// compile_source is needed by external integration tests.
pub use part_01::compile_source;
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
