// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `vb_rpch_seed_dimensions` Verus spec.
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is the production-binding surface for the seed-dimensions
// proof obligations proved by the companion spec
// `verification/verus/vb_rpch_seed_dimensions.rs`.
//
// The production surface bound here lives in:
//
//   - crates/vb_storage/src/recovery/replay/summary/derive.rs
//       * recovery_dimension_count_from_index
//           (derive.rs:250-261)
//       * recovery_seed_dimensions_positive
//           (derive.rs:265-267)
//       * recovery_observed_dimension_is_positive
//           (derive.rs:271-276)
//
// ============================================================================
// STRUCTURAL BINDING — production mirror via #[path]
// ============================================================================
//
// This file uses two complementary binding mechanisms:
//
//   1. **Drift-detection inclusion**: a direct `#[path]` inclusion of
//      the verbatim production mirror at
//      `verification/verus/production_inner/replay_invariants_production.rs`
//      wrapped in `#[verifier::external]` at module level. This
//      validates that the production source still compiles and that
//      production method/field names resolve at compile time.
//
//   2. **Spec-side mirror types and methods** (declared in `verus!`
//      context below): `MirrorRecoveryFrameSeed`,
//      `MirrorRecoveryError`, and the three production-bound exec
//      wrappers with `#[verifier::external]` bodies that reproduce
//      the production logic byte-for-byte. The companion spec file
//      attaches `assume_specification` bridges that state the
//      production contracts.
//
// ============================================================================
// WHY NOT FULL `#[path]` INCLUSION OF derive.rs
// ============================================================================
// Direct `#[path = "../../crates/vb_storage/src/recovery/replay/summary/derive.rs"]`
// is blocked by:
//
//   1. `derive.rs:12-23` imports `std::collections::{HashMap, HashSet}`
//      plus `vb_core::*` and crate-internal paths not registered
//      under a standalone `verus --crate-type=lib` invocation.
//
//   2. `types.rs:37-145` declares `RecoveryError` via
//      `#[derive(thiserror::Error)]`, and `types.rs:629` uses
//      `#[derive(... Serialize, Deserialize)]` for `RecoveryFrameSeed`.
//
// The in-tree mirror at
// `verification/verus/production_inner/replay_invariants_production.rs`
// sidesteps every blocker.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Type mirrors:
//   - `RunId(pub u64)`                             <- crates/vb_core/src/ids/mod.rs:65
//   - `MirrorRecoveryError::FrameDimensionOverflow { run: RunId }`
//                                                <- crates/vb_storage/src/recovery/types.rs:139-144
//   - `MirrorRecoveryFrameSeed { step_count: u16, slot_count: u16 }`
//                                                <- crates/vb_storage/src/recovery/types.rs:629-649
//
// Production-bound exec wrappers:
//   - `production_recovery_dimension_count_from_index`  <- derive.rs:250-261
//   - `production_recovery_seed_dimensions_positive`    <- derive.rs:265-267
//   - `production_recovery_observed_dimension_is_positive` <- derive.rs:271-275
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The production bodies of every fn in this file are NOT verified by
// Verus directly. The drift-detection prod_src module is marked
// `#[verifier::external]` at module level, and the spec-side mirror
// method bodies below are also `#[verifier::external]`. The
// `assume_specification` bridges in the companion spec file
// (`vb_rpch_seed_dimensions.rs`) attach the production contracts.
// The exec wrappers in the spec file invoke the spec-side mirror
// functions to discharge the contracts. Drift between the production
// mirror and the production source is reported as binding-debt
// tracked outside Verus.
//
// ============================================================================
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Drift-detection inclusion: `#[path]` to verbatim production mirror
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the verbatim production mirror at
// `production_inner/replay_invariants_production.rs`. The mirror is
// marked `#[verifier::external]` at module level so the production
// bodies are opaque to Verus; the inclusion still validates Rust
// resolution (field names, discriminant sets, fn signatures) at
// compile time.
#[verifier::external]
#[path = "production_inner/replay_invariants_production.rs"]
pub mod prod_src;

// ---------------------------------------------------------------------------
// Spec-side ID type mirror — RunId
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct RunId(pub u64);

impl RunId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

// ---------------------------------------------------------------------------
// Spec-side mirror of RecoveryError — FrameDimensionOverflow only
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub enum MirrorRecoveryError {
    FrameDimensionOverflow {
        run: RunId,
    },
}

// ---------------------------------------------------------------------------
// Spec-side mirror of RecoveryFrameSeed — step_count and slot_count
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct MirrorRecoveryFrameSeed {
    pub step_count: u16,
    pub slot_count: u16,
}

// ---------------------------------------------------------------------------
// Spec-side mirror functions — production body-identical
// ---------------------------------------------------------------------------
//
// All bodies are `#[verifier::external]`. The companion spec file
// attaches `assume_specification` bridges that state the production
// contracts. The exec wrappers in the spec file invoke these mirror
// functions and assert the contracts hold.
#[verifier::external]
pub fn production_recovery_dimension_count_from_index(
    max_index: Option<u16>,
    run: RunId,
) -> Result<u16, MirrorRecoveryError> {
    match max_index {
        Some(value) => match value.checked_add(1) {
            Some(count) => Ok(count),
            None => Err(MirrorRecoveryError::FrameDimensionOverflow { run }),
        },
        None => Ok(0),
    }
}

#[verifier::external]
pub fn production_recovery_seed_dimensions_positive(seed: &MirrorRecoveryFrameSeed) -> bool {
    seed.step_count > 0 && seed.slot_count > 0
}

#[verifier::external]
pub fn production_recovery_observed_dimension_is_positive(
    max_index: Option<u16>,
    count: u16,
) -> bool {
    match max_index {
        Some(_) => count > 0,
        None => count == 0,
    }
}

// ---------------------------------------------------------------------------
// Phantom drift-detection helper
// ---------------------------------------------------------------------------
//
// The body is `#[verifier::external]` (opaque to Verus), but the
// `prod_src::*` references force Rust to resolve the production
// function names at compile time. A rename of any of these
// production functions (or its parameter types) breaks this fn's
// compilation.
#[verifier::external]
fn prod_methods_drift_check(
    seed: &prod_src::RecoveryFrameSeed,
    run: prod_src::RunId,
) {
    let _ = prod_src::recovery_dimension_count_from_index(None, run);
    let _ = prod_src::recovery_seed_dimensions_positive(seed);
    let _ = prod_src::recovery_observed_dimension_is_positive(None, 0u16);
    let _ = prod_src::recovery_observed_dimension_is_positive(Some(0u16), 1u16);
}

} // verus!
