// SPDX-License-Identifier: MIT
//
// Extern surface for `vb_oewy_bdd_runner_invariant` Verus spec.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is a thin re-export surface for the production mirror at
// `verification/verus/production_inner/bdd_runner_invariant_production.rs`,
// which is a structural mirror of the BDD runner module in
// `crates/workspace_tests/src/bdd_runner.rs`.
//
// The companion spec file (`vb_oewy_bdd_runner_invariant.rs`)
// attaches spec contracts to the projections via
// `assume_specification`, and every proof below the bridge exercises
// the production wrappers through exec wrappers.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production mirror bodies are `#[verifier::external]` so Verus
// skips body verification; the production contract is attached via
// `assume_specification` in the companion spec file. Drift between
// the mirror and the production source is reported as binding-debt
// outside Verus.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

// ============================================================================
// PRODUCTION MIRROR INCLUSION via #[path] (WEAK BINDING)
// ============================================================================
#[path = "production_inner/bdd_runner_invariant_production.rs"]
pub mod prod_src;

// Re-export everything from prod_src into the production module so
// the spec file's `production::Type` references resolve correctly.
pub use prod_src::*;