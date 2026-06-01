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

// Proptest for nested for_each round-trip properties (vb-xi2f.21).
// Obligations: PO-007, PO-008, PO-013, PO-014.
// State: 5 (proof-writer)
#[cfg(test)]
mod proptest_nested_foreach;

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
