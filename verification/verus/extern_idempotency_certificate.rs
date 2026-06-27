// SPDX-License-Identifier: MIT
//
// Extern surface for idempotency_certificate_summary Verus spec.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// PRODUCTION INCLUSION via #[path]:
// Direct `#[path]` inclusion of
// verification/verus/production_inner/idempotency_certificate_production.rs.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

#[path = "production_inner/idempotency_certificate_production.rs"]
pub mod prod;

verus! {

pub use prod::{
    is_contract_idempotency_accepted, requires_idempotency_key,
    runtime_missing_idempotency_attestation, storage_certificate_accepts_action,
    IdempotencyClass, RetrySafetyClass, SideEffectClass,
};

} // verus!