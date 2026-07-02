// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SHIM for vb-vzcuf-PS-002 Verus spec
// ============================================================================
//
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance):
//
// This file is a thin shim. It does NOT contain the production mirror
// itself; instead it `#[path]`-includes the in-tree mirror at
// `verification/verus/production_inner/vb_vzcuf_PS_002_production.rs`.
//
// The production-binding gate (`check-verus-production-binding.sh`)
// recognises this file as the WEAK companion extern pattern: the spec
// file at `verification/verus/vb-vzcuf-PS-002.rs` `#[path]`-includes
// this shim, and this shim in turn `#[path]`-includes the in-tree
// mirror, which is the production surface that the binding gate
// audit walks to.
//
// The in-tree mirror is a verbatim copy of the production byte-
// admission arithmetic at `crates/vb_storage/src/batch/append_event.rs`
// with minimal substitutions (spec-mode type aliases like
// `SpecJournalError` / `SpecJournalWriteBatch` and `#[verifier::external]`
// bodies on `byte_admit`, `production_checked_add_u64`,
// `production_u32_to_u64`, `production_try_usize_to_u64`). Drift
// between the mirror and the production source is tracked by
// `check-production-inner-drift.sh`.
//
// Why this is WEAK rather than STRONG: the production `append_event.rs`
// transitively reaches `crate::codec::encode_record`,
// `crate::error::JournalError`, `fjall::OwnedWriteBatch`,
// `&FjallJournal`, none of which are Verus-modelable in a single-file
// `verus --crate-type=lib` invocation. The mirror sidesteps every
// blocker while preserving the byte-admission arithmetic byte-for-
// byte, so any drift in field names, primitive choices (e.g.
// `checked_add` -> `wrapping_add`), or guard ordering breaks the
// `extern_vb_vzcuf_PS_002` Verus build.
//
// The `assume_specification` bridge in the companion spec file
// (`vb-vzcuf-PS-002.rs`) attaches the production contract to the
// `SpecJournalWriteBatch::byte_admit` exec fn and the related
// production-primitive wrappers, and the exec wrappers in that file
// exercise the bridges from `verus!` context, so the bridges are not
// used as vacuums.
#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION MIRROR INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the in-tree mirror at
// `production_inner/vb_vzcuf_PS_002_production.rs`. The mirror is the
// production surface for the byte-admission guard (see header above).
// Any drift in the mirror or in the production source breaks this
// Verus build.
#[path = "production_inner/vb_vzcuf_PS_002_production.rs"]
pub mod production;

} // verus!

// Re-export the production types and exec fns so the spec file can
// reference them as `production::SpecJournalError` etc.
pub use production::{
    production_checked_add_u64, production_try_usize_to_u64, production_u32_to_u64,
    SpecJournalError, SpecJournalWriteBatch,
};