// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SURFACE for runtime_facade_shard_index_production_bridge Verus spec
// (WEAK binding via production_inner/)
// ============================================================================
//
// This file is the production-binding surface for the
// `runtime_facade_shard_index_production_bridge.rs` Verus spec. It includes
// the in-tree production mirror at
// `verification/verus/production_inner/runtime_facade_shard_index_production.rs`
// via `#[path]` so that:
//
//   * The companion gate `scripts/check-verus-production-binding.sh`
//     classifies the spec file as WEAK-bound (spec uses
//     `#[path = "extern_runtime_facade_shard_index.rs"]`; this file uses
//     `#[path = "production_inner/runtime_facade_shard_index_production.rs"]`).
//   * Any drift in the production method names or fn signature breaks
//     the `production_inner/runtime_facade_shard_index_production.rs`
//     mirror and the spec proofs that depend on it.
//
// The mirror at `production_inner/runtime_facade_shard_index_production.rs`
// is a hand-written structural copy of the production surface in
// `crates/vb_runtime/src/runtime.rs::Runtime::shard_index` (lines 828-840).
// The substitutions relative to direct production `#[path]` inclusion are
// documented in the mirror's header.
//
// BINDING LEDGER (mirrors production_inner/runtime_facade_shard_index_production.rs)
// ============================================================================
//   - `production_runtime_shard_index`   <- crates/vb_runtime/src/runtime.rs:828-840
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production body of `production_runtime_shard_index` is NOT verified
// by Verus. The fn is `#[verifier::external]` so Verus skips body
// verification. The contract attached via `assume_specification` in the
// companion spec file `runtime_facade_shard_index_production_bridge.rs`
// states the production behavior the spec proofs discharge. Drift between
// the mirror and the production source is reported as binding-debt tracked
// outside Verus.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION MIRROR INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the in-tree mirror at
// `production_inner/runtime_facade_shard_index_production.rs` (NOT the
// actual production source). The mirror is a hand-written structural
// copy of `crates/vb_runtime/src/runtime.rs::Runtime::shard_index` with
// documented substitutions (the production `Runtime` struct is not
// instantiated; the mirror collapses the method signature to a free
// function over `(run_hash: u64, shard_count: u64)`). Any drift in field
// NAME or method signature breaks the verification build.
#[path = "production_inner/runtime_facade_shard_index_production.rs"]
pub mod production_inner;

} // verus!

// Re-export the production types and exec wrappers so the spec file can
// reference them via `crate::production::*`. The mirror module is included
// inside `verus!` so the type declarations are nameable in spec mode; this
// outer re-export makes them visible in exec mode as well.
pub use production_inner::*;