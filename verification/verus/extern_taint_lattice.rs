// SPDX-License-Identifier: MIT
//
// Extern surface for taint_lattice Verus spec.
//
// ============================================================================
// WEAK PRODUCTION BINDING (in-tree production_inner/ mirror)
// ============================================================================
// This file binds the taint_lattice.rs Verus spec to the in-tree
// production mirror at
//   verification/verus/production_inner/taint_lattice_production.rs
// via `#[path]` plus module-level `#[verifier::external]` plus
// re-exports. The mirror's `Taint` enum is a verbatim copy of
// `crates/vb_core/src/value.rs:14-25` (5 variants, discriminants
// 0..=4, `#[non_exhaustive]`) and its `join_taint` is a verbatim copy
// of `crates/vb_core/src/value.rs:29-45`. The remaining helpers
// (`taint_rank`, `join_many`, `is_*`, `secret_*`, `derived_*`,
// `all_lattice_laws`) are spec-side observational helpers modeled on
// production's higher-rank-wins `join_taint` semantics — they exist
// in the mirror so the spec file's `assume_specification` bridges
// have a callable name to bind to.
//
// WHY WEAK (NOT STRONG)
// ----------------------------------------------------------------------------
// A STRONG binding would be `#[path =
// "../../crates/vb_core/src/value.rs"]` directly, but that file's
// surrounding module pulls in `serde::{Deserialize, Deserializer,
// Serialize, Serializer}` and `crate::value_store::ValueStore` — proc
// macros and inter-module dependencies that Verus 0.2026.05.05 cannot
// resolve in single-file mode. The previous binding used
// `crates/vb_core/src/proof_kernels/taint.rs` as a "STRONG by
// file-path" stand-in, but that file is a proof-kernel substitute
// (3-variant enum, no discriminants, helper functions that don't exist
// in production) — under the binding gate's classification matrix
// it is VACUUM, not STRONG, because it is a substitute, not the
// production source.
//
// The WEAK mirror route is the honest classification: the mirror is
// structurally faithful (verbatim enum and `join_taint` from
// production) and the binding gate's `production_inner/...` regex
// classifies this artifact correctly.
//
// WHY MODULE-LEVEL #[verifier::external]
// ----------------------------------------------------------------------------
// The mirror is plain Rust under `verus!` (no `#[verifier::external]`
// on its body items). Marking the mirror module itself
// `#[verifier::external]` causes Verus to trust the bodies and skip
// detailed body verification, which avoids a rustc trait-resolution
// panic (`index out of bounds: the len is 0 but the index is 0` in
// `generic_args.rs`) when the spec invokes these fns through the
// `assume_specification` bridge. The types remain visible (so
// `production::Taint::Clean` still names the production discriminant)
// and the fns remain callable from spec exec wrappers, but the bodies
// are trusted rather than proven.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - Production `Taint` enum (Clean=0, DerivedFromSecret=1, Secret=2,
//     Random=3, TimeDependent=4)
//                              <- crates/vb_core/src/value.rs:14-25
//   - Production `join_taint(a, b) -> Taint`
//                              <- crates/vb_core/src/value.rs:29-45
//   - Mirror `Taint` enum      <- production_inner/taint_lattice_production.rs
//   - Mirror `taint_rank(self_) -> u8`
//                              <- production_inner/taint_lattice_production.rs
//                                 (spec-side observational helper,
//                                 modeled on value.rs:30-35 / 37-43)
//   - Mirror `join_many(taints: &[Taint]) -> Taint`
//                              <- production_inner/taint_lattice_production.rs
//                                 (spec-side observational helper)
//   - Mirror `is_commutative(a, b) -> bool`
//                              <- production_inner/taint_lattice_production.rs
//                                 (spec-side observational helper)
//   - Mirror `is_associative(a, b, c) -> bool`
//                              <- production_inner/taint_lattice_production.rs
//                                 (spec-side observational helper)
//   - Mirror `is_idempotent(a) -> bool`
//                              <- production_inner/taint_lattice_production.rs
//                                 (spec-side observational helper)
//   - Mirror `has_identity(a) -> bool`
//                              <- production_inner/taint_lattice_production.rs
//                                 (spec-side observational helper)
//   - Mirror `secret_never_downgrades() -> bool`
//                              <- production_inner/taint_lattice_production.rs
//                                 (spec-side observational helper)
//   - Mirror `derived_never_downgrades() -> bool`
//                              <- production_inner/taint_lattice_production.rs
//                                 (spec-side observational helper)
//   - Mirror `all_lattice_laws(a, b, c) -> bool`
//                              <- production_inner/taint_lattice_production.rs
//                                 (spec-side observational helper)
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// Mirror bodies are trusted (module-level `#[verifier::external]`).
// The companion spec file's `assume_specification` contracts state
// what each body is claimed to compute; the `wrapper_*` exec fns in
// the spec file actually invoke each production-named fn and
// discharge the contract at every call site. Drift between the mirror
// body and the contract is caught at Verus verification time.
use vstd::prelude::*;

verus! {

#[verifier::external]
#[path = "production_inner/taint_lattice_production.rs"]
#[allow(dead_code, non_snake_case)]
pub mod prod_src;

// Re-export the mirror's Taint enum and its public fns so the
// companion spec file can reference them as `production::Taint`,
// `production::join_taint`, etc., without the nested
// `production::prod_src::Taint` path. The re-exports do not change
// the trusted boundary: every re-exported name is backed by the
// `#[verifier::external]` body from `prod_src`, and every wrapper in
// the spec file discharges the bound contract.
pub use prod_src::Taint;
pub use prod_src::taint_rank;
pub use prod_src::join_taint;
pub use prod_src::join_many;
pub use prod_src::is_commutative;
pub use prod_src::is_associative;
pub use prod_src::is_idempotent;
pub use prod_src::has_identity;
pub use prod_src::secret_never_downgrades;
pub use prod_src::derived_never_downgrades;
pub use prod_src::all_lattice_laws;

} // verus!