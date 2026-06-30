// SPDX-License-Identifier: MIT
//
// Extern surface for yaml_e2e_digest_roles Verus spec.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is a thin re-export surface for the production mirror at
// `verification/verus/production_inner/yaml_e2e_digest_roles_production.rs`,
// which is a structural mirror of:
//   - `canonical_digest` at
//     `crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs:25-51`
//     (SOURCE-role digest)
//   - `compute_compiled_digest` at
//     `crates/vb_compile/src/mod_compile_core.rs:114-116`
//     (ARTIFACT-role digest)
//
// The companion spec file (`yaml_e2e_digest_roles.rs`) attaches spec
// contracts to the projections via `assume_specification`, and every
// proof below the bridge exercises the production wrappers through
// exec wrappers. There are zero vacuous proofs in the rewritten spec.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production mirror is `#[verifier::external]` at each fn so Verus
// skips body verification; the production contract is attached via
// `assume_specification` in the companion spec file. Drift between
// the mirror and the production source is reported as binding-debt
// outside Verus.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ============================================================================
// PRODUCTION MIRROR INCLUSION via #[path] (WEAK BINDING)
// ============================================================================
#[path = "production_inner/yaml_e2e_digest_roles_production.rs"]
pub mod prod_src;

pub use prod_src::{
    SpecChainError,
    SpecDigest32,
    SpecDigestRole,
    SpecShellTarget,
    digest_eq,
    spec_canonical_digest,
    spec_classify_role_mismatch,
    spec_compute_compiled_digest,
    spec_recovery_error_classification,
    spec_recovery_success_allowed,
};

} // verus!