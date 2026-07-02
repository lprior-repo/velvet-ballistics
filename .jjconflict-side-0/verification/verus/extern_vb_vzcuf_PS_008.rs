// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SHIM for vb-vzcuf-PS-008 Verus spec
// ============================================================================
//
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance):
//
// This file is a thin shim. It does NOT contain the production mirror
// itself; instead it `#[path]`-includes the in-tree mirror at
// `verification/verus/production_inner/vb_vzcuf_PS_008_production.rs`.
//
// The production-binding gate (`check-verus-production-binding.sh`)
// recognises this file as the WEAK companion extern pattern: the spec
// file at `verification/verus/vb-vzcuf-PS-008.rs` `#[path]`-includes
// this shim, and this shim in turn `#[path]`-includes the in-tree
// mirror, which is the production surface that the binding gate
// audit walks to.
//
// The in-tree mirror is a verbatim copy of the production
// `JournalWriteBatch<'j>::append_event` guard ordering surface from
// `crates/vb_storage/src/batch/append_event.rs` with minimal
// substitutions (spec-mode type aliases `SpecJournalError` /
// `SpecJournalWriteBatch`, Fjall internals replaced by
// `inner_len: usize` and `journal_has_key: bool` projections, and
// `encode_record` abstracted to `encode_ok: bool`). Drift between
// the mirror and the production source is tracked by
// `check-production-inner-drift.sh`.
//
// Why this is WEAK rather than STRONG: the production `append_event`
// transitively reaches `fjall::OwnedWriteBatch`, `&FjallJournal`,
// `encode_record`, and the `JournalEvent` enum. None of these are
// Verus-modelable in a single-file `verus --crate-type=lib`
// invocation. The mirror sidesteps every blocker while preserving
// the 7-guard ordering byte-for-byte, so any drift in field names,
// method signatures, or guard ordering breaks the
// `extern_vb_vzcuf_PS_008` Verus build.
//
// The `assume_specification` bridge in the companion spec file
// (`vb-vzcuf-PS-008.rs`) attaches the production contract to the
// `SpecJournalWriteBatch::append_event` exec fn and the exec wrappers
// in that file exercise the bridge from `verus!` context, so the
// bridge is not used as a vacuum.
#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::collections::HashSet;
use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION MIRROR INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the in-tree mirror at
// `production_inner/vb_vzcuf_PS_008_production.rs`. The mirror is the
// production surface for `append_event` (see header above). Any drift
// in the mirror or in the production source breaks this Verus build.
//
// Note on the inner module name `production_inner`: this avoids the
// name collision with the outer `production` module that the spec
// file expects. The `pub use` re-exports below surface the spec-mode
// type aliases at the top level of this extern file so the spec
// file's `assume_specification[ production::SpecJournalWriteBatch::append_event ]`
// and `pub use production::{SpecJournalError, SpecJournalWriteBatch}`
// resolve correctly.
#[path = "production_inner/vb_vzcuf_PS_008_production.rs"]
pub mod production_inner;

} // verus!

// Re-export the production types so the spec file can reference them
// as `production::SpecJournalError` etc. The `pub use` is at the top
// level so `production::SpecJournalWriteBatch::append_event` resolves
// through the spec file's `mod production` declaration.
pub use production_inner::{SpecJournalError, SpecJournalWriteBatch};