// SPDX-License-Identifier: MIT
//
// Extern surface for strict_admission_witness Verus spec.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// PRODUCTION INCLUSION via #[path]:
// Direct `#[path]` inclusion of
// verification/verus/production_inner/strict_admission_witness_production.rs.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

#[path = "production_inner/strict_admission_witness_production.rs"]
pub mod prod;

verus! {

pub use prod::{
    production_storage_backed, production_strict_like, strict_admission_witness_decision,
    SpecRuntimePolicy, SpecStrictWitnessResult, SpecWitnessKind,
};

} // verus!