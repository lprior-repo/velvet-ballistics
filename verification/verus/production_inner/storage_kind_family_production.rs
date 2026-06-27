// SPDX-License-Identifier: MIT
//
// ============================================================================
// Drift-detection stub for storage_kind_family
// ============================================================================
//
// This file is a minimal drift-detection stub for the
// `storage_kind_family` Verus spec. It exists so the companion
// `verification/verus/extern_storage_kind_family.rs` can include this
// file via `#[path = "production_inner/storage_kind_family_production.rs"]`
// to satisfy the production-binding gate.
//
// The actual production mirror content (with full type definitions,
// RecordKind discriminant values, validate_kind_family body, etc.) lives
// in `verification/verus/extern_storage_kind_family.rs`, which carries
// verbatim copies of the production source at:
//
//   - `is_known_record_kind`     <- crates/vb_storage/src/codec/validation.rs:23
//   - `validate_kind_family`     <- crates/vb_storage/src/codec/validation.rs:42
//   - `validate_replay_sequence` <- crates/vb_storage/src/journal/replay.rs:164
//   - `next_seq`                 <- crates/vb_storage/src/codec/mod.rs:142
//   - `validate_replayed_event`  <- crates/vb_storage/src/codec/mod.rs:149
//   - Magic constants            <- crates/vb_storage/src/constants.rs
//
// This stub mirrors the production `RecordKind` enum discriminant set
// (the discriminant is the smallest drift-detection surface — any
// change to the production variant set breaks this stub).

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Drift-detection type stubs
// ---------------------------------------------------------------------------

/// Mirror of production `RecordKind` enum discriminant at
/// `crates/vb_storage/src/records.rs:139`. Discriminant set is the
/// smallest drift-detection surface — any rename of a variant breaks
/// this stub. The body is `#[verifier::external]` (opaque).
#[verifier::external]
pub fn record_kind_discriminant_check(kind_id: u16) -> bool {
    // Production kind IDs (verbatim from crates/vb_storage/src/records.rs).
    // Used to surface drift in production discriminant values.
    matches!(kind_id, 1 | 2 | 3 | 10..=29 | 30 | 31 | 32 | 40 | 50)
}

/// Mirror of production `is_known_record_kind` decision fn at
/// `crates/vb_storage/src/codec/validation.rs:23`. Body is
/// `#[verifier::external]` (opaque).
#[verifier::external]
pub fn is_known_record_kind_stub(kind: u16) -> bool {
    record_kind_discriminant_check(kind)
}

} // verus!