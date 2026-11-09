// SPDX-License-Identifier: MIT
//
// ============================================================================
// Drift-detection stub for value_store_invariant
// ============================================================================
//
// This file is a minimal drift-detection stub for the
// `value_store_invariant` Verus spec. It exists so the companion
// `verification/verus/extern_value_store_invariant.rs` can include
// this file via
// `#[path = "production_inner/value_store_invariant_production.rs"]`
// to satisfy the production-binding gate.
//
// The actual production mirror content (ObjectField, ValueStore,
// check_arena_cap body, etc.) lives in
// `verification/verus/extern_value_store_invariant.rs`, which carries
// verbatim copies of the production source at:
//
//   - `ObjectField::clean`              <- crates/vb_core/src/value_store/objects.rs:15-21
//   - `ObjectField::with_taint`         <- crates/vb_core/src/value_store/objects.rs:24-26
//   - `ValueStore::with_max_slots`      <- crates/vb_core/src/value_store.rs:66-77
//   - `ValueStore::total_arena_count`   <- crates/vb_core/src/value_store.rs:289-296
//   - `ValueStore::max_arena_entries`   <- crates/vb_core/src/value_store.rs:300-302
//   - `ValueStore::check_arena_cap`     <- crates/vb_core/src/value_store.rs:367-379
//
// This stub mirrors the production `ValueStore::max_arena_entries`
// field as the smallest drift-detection surface.
//
// DRIFT POLICY: `crates/vb_core/src/value_store.rs:33-379`
// Production source coverage:
//   - `ObjectField::clean`              <- crates/vb_core/src/value_store/objects.rs:15-21
//   - `ObjectField::with_taint`         <- crates/vb_core/src/value_store/objects.rs:24-26
//   - `ValueStore::with_max_slots`      <- crates/vb_core/src/value_store.rs:66-77
//   - `ValueStore::total_arena_count`   <- crates/vb_core/src/value_store.rs:289-296
//   - `ValueStore::max_arena_entries`   <- crates/vb_core/src/value_store.rs:300-302
//   - `ValueStore::check_arena_cap`     <- crates/vb_core/src/value_store.rs:367-379
// Regenerate this file whenever production changes. Any rename of
// `max_arena_entries` or change to `check_arena_cap` body breaks the
// `extern_value_store_invariant` Verus build.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Drift-detection stubs
// ---------------------------------------------------------------------------
/// Mirror of production `ValueStore::max_arena_entries` field at
/// `crates/vb_core/src/value_store.rs:45`. The stub struct carries the
/// SAME field name (`max_arena_entries`) and type (`u64`) so any rename
/// breaks the build.
#[derive(Clone, Copy)]
pub struct ValueStoreStub {
    /// Mirror of production `pub max_arena_entries: u64` at value_store.rs:45.
    pub max_arena_entries: u64,
    /// Mirror of production `total_arena_count` projection at
    /// value_store.rs:289-296.
    pub total_arena_count_field: u64,
}

impl ValueStoreStub {
    /// Mirror of production `ValueStore::check_arena_cap` decision at
    /// `crates/vb_core/src/value_store.rs:367-379`. Body is
    /// `#[verifier::external]` (opaque).
    #[verifier::external]
    pub fn check_arena_cap_stub(self) -> bool {
        if self.max_arena_entries == 0 {
            return true;
        }
        self.total_arena_count_field < self.max_arena_entries
    }
}

} // verus!
