// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SHIM for vb-vzcuf-PS-001 Verus spec
// ============================================================================
//
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance):
//
// This file is a thin shim. It does NOT contain the production mirror
// itself; instead it `#[path]`-includes the in-tree mirror at
// `verification/verus/production_inner/vb_vzcuf_PS_001_production.rs`.
//
// The production-binding gate (`check-verus-production-binding.sh`)
// recognises this file as the WEAK companion extern pattern: the spec
// file at `verification/verus/vb-vzcuf-PS-001.rs` `#[path]`-includes
// this shim, and this shim in turn `#[path]`-includes the in-tree
// mirror, which is the production surface that the binding gate
// audit walks to.
//
// The in-tree mirror is a verbatim copy of the production byte-
// admission arithmetic at `crates/vb_storage/src/batch/append_event.rs`
// with minimal substitutions (spec-mode type aliases like
// `SpecJournalError` / `SpecJournalWriteBatch` and a `#[verifier::external]`
// body on `byte_admit`). Drift between the mirror and the production
// source is tracked by `check-production-inner-drift.sh`.
//
// Why this is WEAK rather than STRONG: the production `append_event.rs`
// transitively reaches `fjall::OwnedWriteBatch`, `&FjallJournal`,
// `crate::codec::encode_record`, `crate::events::JournalEvent`,
// `crate::records::RecordKind`, `crate::error::JournalError`, and
// `crate::keys::run_event_key`, none of which are Verus-modelable in a
// single-file `verus --crate-type=lib` invocation. The mirror sidesteps
// every blocker while preserving the byte-admission arithmetic byte-
// for-byte, so any drift in field names, primitive choices (e.g.
// `checked_add` -> `wrapping_add`), or guard ordering breaks the
// `extern_vb_vzcuf_PS_001` Verus build.
//
// The `assume_specification` bridge in the companion spec file
// (`vb-vzcuf-PS-001.rs`) attaches the production contract to the
// `SpecJournalWriteBatch::byte_admit` exec fn and the exec wrappers in
// that file exercise the bridge from `verus!` context, so the bridge
// is not used as a vacuum.
#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION MIRROR INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the in-tree mirror at
// `production_inner/vb_vzcuf_PS_001_production.rs`. The mirror is the
// production surface for the byte-admission guard (see header above).
// Any drift in the mirror or in the production source breaks this
// Verus build.
#[path = "production_inner/vb_vzcuf_PS_001_production.rs"]
pub mod production;

} // verus!

// Re-export the production types and constants so the spec file can
// reference them as `production::SpecJournalError` etc.
pub use production::{
    SPEC_MAX_JOURNAL_BATCH_BYTES_LIMIT, SpecJournalError, SpecJournalWriteBatch,
};