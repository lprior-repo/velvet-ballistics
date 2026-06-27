// SPDX-License-Identifier: MIT
//
// Extern surface for idempotency_decision Verus spec.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// PRODUCTION INCLUSION via #[path]:
// Direct `#[path]` inclusion of
// verification/verus/production_inner/idempotency_decision_production.rs.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

#[path = "production_inner/idempotency_decision_production.rs"]
pub mod prod;

verus! {

pub use prod::{
    is_contract_idempotency_accepted, is_statically_idempotent_contract, ActionContract, ActionId,
    Idempotency, IdempotencyContractViolation, RetrySafety, SideEffect,
};

} // verus!