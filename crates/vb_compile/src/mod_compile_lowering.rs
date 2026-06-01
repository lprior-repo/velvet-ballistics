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

// Verus extern_spec proofs for nested reduce body lowering (vb-xi2f.24).
// GOD RULE 2: extern_spec binds spec functions to production body_width,
// canonical_body_step_width, checked_step_offset, and emit_single_body_set.
// This module does NOT modify production behavior — only adds verification specs.
#[cfg(verus)]
mod verus_reduce_proofs;

// ============================================================
// Kani harnesses for nested reduce body lowering (vb-xi2f.24).
// 11 harnesses covering width parity, offset monotonicity, chain integrity,
// overflow, nested next, empty body, regression, ForEach width, no-panic,
// diagnostics, and try_from_parts end-to-end validation.
// ============================================================
#[cfg(kani)]
mod kani_reduce_body_width;
#[cfg(kani)]
mod kani_reduce_chain;
#[cfg(kani)]
mod kani_reduce_diagnostics;
#[cfg(kani)]
mod kani_reduce_empty;
#[cfg(kani)]
mod kani_reduce_foreach;
#[cfg(kani)]
mod kani_reduce_nested_next;
#[cfg(kani)]
mod kani_reduce_nopanic;
#[cfg(kani)]
mod kani_reduce_offset;
#[cfg(kani)]
mod kani_reduce_overflow;
#[cfg(kani)]
mod kani_reduce_regression;
#[cfg(kani)]
mod kani_reduce_tryfromparts;

// ============================================================
// Proptest properties for nested reduce body lowering (vb-xi2f.24).
// 13 properties covering width parity, offset monotonicity, chain integrity,
// overflow, nested next, empty body, regression, nested ForEach layout,
// no-panic, digest determinism, diagnostic codes, try_from_parts, and
// together-collision regression.
// ============================================================
#[cfg(test)]
mod reduce_body_width_parity;
#[cfg(test)]
mod reduce_body_offset_monotonic;
#[cfg(test)]
mod reduce_body_chain_integrity;
#[cfg(test)]
mod reduce_body_width_overflow;
#[cfg(test)]
mod reduce_nested_next;
#[cfg(test)]
mod reduce_empty_body;
#[cfg(test)]
mod reduce_single_step_regression;
#[cfg(test)]
mod reduce_nested_foreach_layout;
#[cfg(test)]
mod reduce_lowering_no_panic;
#[cfg(test)]
mod reduce_digest_determinism;
#[cfg(test)]
mod reduce_diagnostic_codes;
#[cfg(test)]
mod reduce_multi_step_try_from_parts;
#[cfg(test)]
mod reduce_together_collision;

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

// ── vb-xi2f.22: Nested Together Body Lowering tests ──
#[cfg(test)]
mod together_e2e_tests;
#[cfg(test)]
mod together_integration_tests;
#[cfg(test)]
mod together_lowering_tests;
#[cfg(test)]
mod together_width_tests;
// ── end vb-xi2f.22 tests ──

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
