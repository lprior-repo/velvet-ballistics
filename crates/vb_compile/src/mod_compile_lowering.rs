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

// Kani harnesses for ForEach digest coverage (PO-K-FE-01 through PO-K-FE-10).
// Bead: vb-xi2f.28 | State: 5 (proof-writer)
// Placed here to access pub(super) items from part_05 (canonical_digest, digest_step_primitive).
#[cfg(kani)]
mod kani_proofs;

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
