// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SHIM for vb-vzcuf-PS-006 Verus spec
// ============================================================================
//
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance):
//
// This file is a thin shim. It does NOT contain the production mirror
// itself; instead it `#[path]`-includes the in-tree mirror at
// `verification/verus/production_inner/vb_vzcuf_PS_006_production.rs`.
//
// The production-binding gate (`check-verus-production-binding.sh`)
// recognises this file as the WEAK companion extern pattern: the spec
// file at `verification/verus/vb-vzcuf-PS-006.rs` `#[path]`-includes
// this shim, and this shim in turn `#[path]`-includes the in-tree
// mirror, which is the production surface that the binding gate
// audit walks to.
//
// The in-tree mirror is a verbatim copy of the production
// `JournalWriteBatch<'j>` constructor and accessor surface from
// `crates/vb_storage/src/batch/types.rs` with minimal substitutions
// (spec-mode type alias `SpecJournalWriteBatch`, Fjall internals
// replaced by `inner_len: usize`, and parameterized
// `new_with_limit(limit)` to avoid the `&FjallJournal` argument
// that Verus cannot model). Drift between the mirror and the
// production source is tracked by `check-production-inner-drift.sh`.
//
// Why this is WEAK rather than STRONG: the production
// `JournalWriteBatch<'j>` carries `fjall::OwnedWriteBatch` and
// `&FjallJournal` fields that are opaque to Verus in a single-file
// `verus --crate-type=lib` invocation. The mirror sidesteps every
// blocker while preserving the byte-limit-relevant fields,
// constructor signature, and accessor signatures byte-for-byte, so
// any drift in field names or method signatures breaks the
// `extern_vb_vzcuf_PS_006` Verus build.
//
// The `assume_specification` bridges in the companion spec file
// (`vb-vzcuf-PS-006.rs`) attach the production contracts to the
// `SpecJournalWriteBatch::{new_with_limit, new_default, byte_limit,
// staged_event_bytes, len, is_empty, is_aborted}` exec fns, and the
// exec wrappers in that file exercise the bridges from `verus!`
// context, so the bridges are not used as vacuums.
#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION MIRROR INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the in-tree mirror at
// `production_inner/vb_vzcuf_PS_006_production.rs`. The mirror is the
// production surface for the `JournalWriteBatch` constructor and
// accessors (see header above). Any drift in the mirror or in the
// production source breaks this Verus build.
//
// Note on the inner module name `production_inner`: this avoids the
// name collision with the outer `production` module that the spec
// file expects. The `pub use` re-export below surfaces the spec-mode
// type alias at the top level of this extern file so the spec file's
// `assume_specification[ production::SpecJournalWriteBatch::new_with_limit ]`
// and similar references resolve correctly.
#[path = "production_inner/vb_vzcuf_PS_006_production.rs"]
pub mod production_inner;

} // verus!

// Re-export the production type so the spec file can reference it as
// `production::SpecJournalWriteBatch` from its `mod production` import.
pub use production_inner::SpecJournalWriteBatch;