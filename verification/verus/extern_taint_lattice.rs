// SPDX-License-Identifier: MIT
//
// Extern surface for taint_lattice Verus spec.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
// This file binds the taint_lattice.rs Verus spec to the canonical taint
// proof-kernel source at `crates/vb_core/src/proof_kernels/taint.rs` via
// `#[path]` plus module-level `#[verifier::external]`. The binding is
// structural + contractual:
//
//   * Every type, every discriminant, and every fn signature the spec
//     reasons about is sourced from the production file by direct
//     `#[path]` include. There is NO mirror; if the production file
//     renames a variant, changes a discriminant, or alters a signature,
//     the spec file's `assume_specification` and `external_type_specification`
//     bridges fail to resolve and Verus reports a structural error.
//
//   * The production module is marked `#[verifier::external]`, which is
//     the module-level equivalent of "every body in this module is
//     opaque to the verifier". The bodies are still type-checked and
//     signature-checked (so drift in argument types or return type is
//     caught), but their semantics are trusted rather than proven.
//
//   * Mathematical contracts live in the companion spec file
//     (verification/verus/taint_lattice.rs) as `assume_specification`
//     declarations. Each contract is the spec-side statement of what
//     the production body does, and the contract is discharged through
//     `exec fn` wrappers in the spec file that actually invoke the
//     production exec fns. Without that invocation the contract is
//     unused (vacuum); with it, every `assert` in the wrapper is a
//     witness that the production call satisfies the spec contract.
//
// ============================================================================
// WHY MODULE-LEVEL #[verifier::external] (not per-fn external_body)
// ============================================================================
// The production `taint.rs` annotates `Taint` with
// `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`. Direct per-fn
// `#[verifier::external]` on every production fn would still leave
// Verus expanding those derives at the include site, and the derives
// call into `core::fmt::{Error, Formatter}::write_str` and
// `core::intrinsics::discriminant_value` — neither of which Verus
// supports without global std-spec augmentation. Marking the whole
// production module `#[verifier::external]` is the precise mechanism
// Verus provides for "this module's contents are opaque". The types
// remain visible (so `production::Taint::Clean` still names the
// production discriminant) and the fns remain callable from spec exec
// wrappers, but the bodies are trusted. This matches the user's
// "Use `#[verifier::external]` on bodies" intent at the granularity
// Verus actually requires.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `Taint` enum (Clean=0, DerivedFromSecret=1, Secret=2)
//                              <- crates/vb_core/src/proof_kernels/taint.rs:6-12
//   - `Taint::rank(&self) -> u8`
//                              <- crates/vb_core/src/proof_kernels/taint.rs:14-22
//   - `join_taint(a, b) -> Taint`
//                              <- crates/vb_core/src/proof_kernels/taint.rs:24-26
//   - `join_many(taints: &[Taint]) -> Taint`
//                              <- crates/vb_core/src/proof_kernels/taint.rs:28-34
//   - `is_commutative(a, b) -> bool`
//                              <- crates/vb_core/src/proof_kernels/taint.rs:36-38
//   - `is_associative(a, b, c) -> bool`
//                              <- crates/vb_core/src/proof_kernels/taint.rs:40-42
//   - `is_idempotent(a) -> bool`
//                              <- crates/vb_core/src/proof_kernels/taint.rs:44-46
//   - `has_identity(a) -> bool`
//                              <- crates/vb_core/src/proof_kernels/taint.rs:48-50
//   - `secret_never_downgrades() -> bool`
//                              <- crates/vb_core/src/proof_kernels/taint.rs:52-54
//   - `derived_never_downgrades() -> bool`
//                              <- crates/vb_core/src/proof_kernels/taint.rs:56-58
//   - `all_lattice_laws(a, b, c) -> bool`
//                              <- crates/vb_core/src/proof_kernels/taint.rs:60-67
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies in taint.rs are NOT verified by Verus (per the
// `#[verifier::external]` directive above). The mathematical contracts
// attached via `assume_specification` in the companion spec file are
// the trusted base: they state what the production code does, but
// Verus does not independently confirm the production bodies satisfy
// those contracts. The `exec fn` wrappers in the spec file are the
// non-vacuum witnesses that the bound is actually exercised — they
// invoke the production exec fn and assert the spec contract holds.
use vstd::prelude::*;

verus! {

#[verifier::external]
#[path = "../../crates/vb_core/src/proof_kernels/taint.rs"]
#[allow(dead_code, non_snake_case)]
pub mod prod_src;

// Re-export the production Taint enum and its public fns so the
// companion spec file can reference them as `production::Taint`,
// `production::join_taint`, etc., without the nested
// `production::prod_src::Taint` path. The re-exports do not change
// the trusted boundary: every re-exported name is still backed by
// the `#[verifier::external]` body from `prod_src`.
pub use prod_src::Taint;
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
