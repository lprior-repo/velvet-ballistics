// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SHIM for vb-vzcuf-PS-007 Verus spec
// ============================================================================
//
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance):
//
// This file is a thin shim. It does NOT contain the production mirror
// itself; instead it `#[path]`-includes the in-tree mirror at
// `verification/verus/production_inner/vb_vzcuf_PS_007_production.rs`.
//
// The production-binding gate (`check-verus-production-binding.sh`)
// recognises this file as the WEAK companion extern pattern: the spec
// file at `verification/verus/vb-vzcuf-PS-007.rs` `#[path]`-includes
// this shim, and this shim in turn `#[path]`-includes the in-tree
// mirror, which is the production surface that the binding gate
// audit walks to.
//
// The in-tree mirror is a verbatim copy of the production byte-policy
// bridge surface from `crates/vb_core/src/workflow/mod.rs`,
// `crates/vb_core/src/budget.rs`, `crates/vb_core/src/limits.rs`,
// and `crates/vb_storage/src/batch/types.rs` with minimal
// substitutions (spec-mode type aliases `SpecResourceContract`,
// `SpecBoundednessPolicy`, `SpecJournalWriteBatchByDefault` and
// `#[verifier::external]` bodies on the production-default
// functions). Drift between the mirror and the production source is
// tracked by `check-production-inner-drift.sh`.
//
// Why this is WEAK rather than STRONG: the production
// `ResourceContract::DEFAULT` and `BoundednessPolicy::DEFAULT` const
// items trigger Verus's `VerusErasureCtxt has not been initialized`
// panic when included via `#[path]`. The mirror abstracts the const
// defaults into regular `fn` methods returning the literal values,
// while preserving the field names, method names, and default values
// byte-for-byte, so any drift in field names, method signatures, or
// default values breaks the `extern_vb_vzcuf_PS_007` Verus build.
//
// The `assume_specification` bridges in the companion spec file
// (`vb-vzcuf-PS-007.rs`) attach the production contracts to the
// `SpecResourceContract::default_max_journal_batch_bytes`,
// `SpecBoundednessPolicy::default_absolute_max_journal_batch_bytes`,
// `spec_max_journal_batch_bytes_hard_cap`,
// `spec_default_journal_batch_byte_limit`, and
// `SpecJournalWriteBatchByDefault::{new, byte_limit,
// staged_event_bytes, is_aborted}` exec fns, and the exec wrappers
// in that file exercise the bridges from `verus!` context, so the
// bridges are not used as vacuums.
#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION MIRROR INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the in-tree mirror at
// `production_inner/vb_vzcuf_PS_007_production.rs`. The mirror is the
// production surface for the byte-policy bridge (see header above).
// Any drift in the mirror or in the production source breaks this
// Verus build.
//
// Note on the inner module name `production_inner`: this avoids the
// name collision with the outer `production` module that the spec
// file expects. The `pub use` re-exports below surface the spec-mode
// type aliases at the top level of this extern file so the spec
// file's `assume_specification[ production::SpecResourceContract::default_... ]`
// and similar references resolve correctly.
#[path = "production_inner/vb_vzcuf_PS_007_production.rs"]
pub mod production_inner;

} // verus!

// Re-export the production types and exec fns so the spec file can
// reference them as `production::SpecResourceContract` etc. from its
// `mod production` import.
pub use production_inner::{
    spec_default_journal_batch_byte_limit, spec_max_journal_batch_bytes_hard_cap,
    SpecBoundednessPolicy, SpecJournalWriteBatchByDefault, SpecResourceContract,
};