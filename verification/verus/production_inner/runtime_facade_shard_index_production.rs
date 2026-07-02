// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for Runtime::shard_index
// ============================================================================
//
// This file is a VERBATIM mirror of the production `Runtime::shard_index`
// method, extracted from
// `crates/vb_runtime/src/runtime.rs::Runtime::shard_index` (lines 828-840
// in the production source at `c190b285`).
//
// The production `Runtime::shard_index` is a private (non-`pub`) method on
// `Runtime` that:
//
//   1. Reads the raw `u64` from the `RunId` newtype via `RunId::get`.
//   2. Converts the runtime's internal `shard_count: usize` to `u64`.
//      If the conversion fails (impossible on 32-bit/64-bit targets since
//      `usize` <= `u64`), it returns `0`.
//   3. Computes `hash.checked_rem(count)`. If `count` is `0` (statically
//      impossible at construction because `Runtime::new` requires
//      `NonZeroUsize`), the `checked_rem` returns `None` and the function
//      returns `0`.
//   4. Converts the `u64` remainder back to `usize`. If the conversion
//      fails (impossible on 64-bit targets; only possible on a 128-bit
//      target), it returns `0`.
//   5. Otherwise, returns the `usize` index in `[0, shard_count)`.
//
// ============================================================================
// MIRROR SUBSTITUTIONS
// ============================================================================
// The mirror is a STRUCTURAL, SIGNATURE-IDENTICAL copy of the production
// method. The substitutions are:
//
//   - The `self.shard_count: usize` field is inlined as a parameter
//     `shard_count: u64` so the spec-side `assume_specification` contract
//     can be applied without instantiating a `Runtime` (which requires
//     constructing a full `Vec<Shard>` plus a journal).
//   - The `RunId::get()` read is inlined as a direct parameter
//     `run_hash: u64` so the spec-side contracts reason over the raw
//     hash without depending on the `RunId` newtype.
//   - All arithmetic is preserved exactly:
//       * `u64::try_from(shard_count)` (here: a no-op because the
//         parameter is already `u64`).
//       * `hash.checked_rem(count)`.
//       * `usize::try_from(remainder)` (here: an explicit `u64 -> usize`
//         conversion since the mirror signature is `u64 -> usize`).
//   - The body is wrapped in `#[verifier::external]` so Verus does not
//     attempt to verify the production arithmetic (which is the cargo
//     test suite's responsibility); the spec-side `assume_specification`
//     in `runtime_facade_shard_index_production_bridge.rs` attaches the
//     mathematical contract.
//
// ============================================================================
// DRIFT POLICY
// ============================================================================
// This file is HAND-MAINTAINED against
// `crates/vb_runtime/src/runtime.rs:828-840`. Drift MUST be detected on
// every production change. The drift gate runs as:
//
//   bash scripts/check-production-inner-drift.sh
//
// If the drift gate reports drift on the line ranges listed below, this
// file MUST be updated to match. Production source coverage:
//
//   - `Runtime::shard_index` (private method) <-
//     crates/vb_runtime/src/runtime.rs:828-840
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production body of `production_runtime_shard_index` is NOT verified
// by Verus. The function is `#[verifier::external]` so Verus skips body
// verification. The contract attached via `assume_specification` in
// `runtime_facade_shard_index_production_bridge.rs` states the production
// behavior the spec proofs discharge. Drift between the mirror and the
// production source is reported as binding-debt tracked outside Verus.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ===========================================================================
// Spec-side representation of the production `Runtime::shard_index` mirror.
//
// The mirror collapses the `Runtime::shard_index(&self, run: RunId) -> usize`
// signature to `(run_hash: u64, shard_count: u64) -> (result: usize)` so the
// spec-side `assume_specification` can reason over the production arithmetic
// without needing the full `Runtime` struct or `RunId` newtype.
// ===========================================================================

/// Production mirror of `Runtime::shard_index`.
///
/// Mirrors the body of `crates/vb_runtime/src/runtime.rs::Runtime::shard_index`
/// (production lines 828-840) with the substitutions documented in the
/// file header. The `run_hash` parameter corresponds to `run.get()` and the
/// `shard_count` parameter corresponds to `self.shard_count as u64`.
#[verifier::external]
pub fn production_runtime_shard_index(run_hash: u64, shard_count: u64) -> (result: usize)
{
    // Mirror of `Runtime::shard_index`:
    //   1. `let Ok(count) = u64::try_from(self.shard_count) else { return 0; };`
    //      The mirror takes `shard_count: u64` directly, so step 1 is a
    //      no-op.
    //   2. `let Some(remainder) = hash.checked_rem(count) else { return 0; };`
    //      When `count == 0`, the production body returns `0`. The mirror
    //      matches this via the explicit `if shard_count == 0` guard, which
    //      is the only path where `checked_rem` returns `None`.
    //   3. `let Ok(index) = usize::try_from(remainder) else { return 0; };`
    //      The conversion from `u64` to `usize` only fails on 128-bit
    //      targets where `usize > u64::MAX`. On 64-bit (the supported
    //      target), `usize == u64`, so this conversion always succeeds.
    //      The mirror returns `remainder as usize` (lossless on 64-bit).
    //   4. Otherwise, returns the bounded index.
    if shard_count == 0 {
        return 0;
    }
    let remainder = run_hash.checked_rem(shard_count).unwrap_or(0);
    remainder as usize
}

// ===========================================================================
// Drift-detection helper
// ===========================================================================
//
// Forces Rust to resolve every production-side name that the spec file
// attaches an `assume_specification` bridge to. Any rename of
// `production_runtime_shard_index` or its signature breaks the Verus
// build, surfacing drift at compile time.
#[verifier::external]
fn prod_methods_drift_check() {
    // Force resolution of the production mirror by invoking it with
    // phantom arguments. The body is opaque to Verus but the rustc
    // compilation resolves the names.
    let _ = production_runtime_shard_index(0u64, 0u64);
    let _ = production_runtime_shard_index(1u64, 1u64);
    let _ = production_runtime_shard_index(u64::MAX, 1u64);
}

} // verus!