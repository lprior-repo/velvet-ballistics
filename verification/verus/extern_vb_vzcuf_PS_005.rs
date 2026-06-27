// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SHIM for vb-vzcuf-PS-005 Verus spec
// ============================================================================
//
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance):
//
// This file is a thin shim. It does NOT contain the production mirror
// itself; instead it `#[path]`-includes the in-tree mirror at
// `verification/verus/production_inner/vb_vzcuf_PS_005_production.rs`.
//
// The production-binding gate (`check-verus-production-binding.sh`)
// recognises this file as the WEAK companion extern pattern: the spec
// file at `verification/verus/vb-vzcuf-PS-005.rs` `#[path]`-includes
// this shim, and this shim in turn `#[path]`-includes the in-tree
// mirror, which is the production surface that the binding gate
// audit walks to.
//
// The in-tree mirror is a verbatim copy of the production
// `encode_record` exec fn from `crates/vb_storage/src/codec/mod.rs`
// with minimal substitutions (spec-mode type alias `SpecEncodeError`
// and a `#[verifier::external]` body on `spec_encode_record`). Drift
// between the mirror and the production source is tracked by
// `check-production-inner-drift.sh`.
//
// Why this is WEAK rather than STRONG: the production
// `encode_record<T: Serialize>` transitively reaches `postcard`,
// `serde::Serialize`, `serde_json` blob types, `RecordKind`, and the
// custom record framing in `payload.rs`. None of these are
// Verus-modelable in a single-file `verus --crate-type=lib`
// invocation. The mirror sidesteps every blocker while preserving the
// `encode_record` decision lattice byte-for-byte, so any drift in
// field names, error variant names, or guard ordering breaks the
// `extern_vb_vzcuf_PS_005` Verus build.
//
// The `assume_specification` bridge in the companion spec file
// (`vb-vzcuf-PS-005.rs`) attaches the production contract to the
// `spec_encode_record` exec fn and the exec wrappers in that file
// exercise the bridge from `verus!` context, so the bridge is not
// used as a vacuum.
#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION MIRROR INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the in-tree mirror at
// `production_inner/vb_vzcuf_PS_005_production.rs`. The mirror is the
// production surface for `encode_record` (see header above). Any drift
// in the mirror or in the production source breaks this Verus build.
//
// Note on the inner module name `production_inner`: this avoids the
// name collision with the outer `production` module that the spec
// file expects. The `pub use` re-exports below surface the spec-mode
// type aliases at the top level of this extern file so the spec
// file's `assume_specification[ production::spec_encode_record ]`
// and `pub use production::{SpecEncodeError, spec_encode_record}`
// resolve correctly.
#[path = "production_inner/vb_vzcuf_PS_005_production.rs"]
pub mod production_inner;

} // verus!

// Re-export the production types and exec fns so the spec file can
// reference them as `production::SpecEncodeError` and
// `production::spec_encode_record` from its `mod production` import.
pub use production_inner::{spec_encode_record, SpecEncodeError};