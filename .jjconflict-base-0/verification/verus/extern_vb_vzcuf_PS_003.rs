// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SHIM for vb-vzcuf-PS-003 Verus spec
// ============================================================================
//
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance):
//
// This file is a thin shim. It does NOT contain the production mirror
// itself; instead it `#[path]`-includes the in-tree mirror at
// `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs`.
//
// The production-binding gate (`check-verus-production-binding.sh`)
// recognises this file as the WEAK companion extern pattern: the spec
// file at `verification/verus/vb-vzcuf-PS-003.rs` `#[path]`-includes
// this shim, and this shim in turn `#[path]`-includes the in-tree
// mirror, which is the production surface that the binding gate
// audit walks to.
//
// The in-tree mirror is a verbatim copy of the production codec
// entry points in `crates/vb_storage/src/codec/` (`encode_record`,
// `decode_record`, `validate_record_kind_family`, payload and header
// helpers) with minimal substitutions (spec-mode type aliases like
// `SpecRecordKind` / `SpecRecordEnvelope` / `SpecJournalError` and
// `#[verifier::external]` bodies on `encode_record` / `decode_record`).
// Drift between the mirror and the production source is tracked by
// `check-production-inner-drift.sh`.
//
// Why this is WEAK rather than STRONG: the production codec module
// transitively reaches `postcard`, `serde::Serialize`,
// `serde::de::DeserializeOwned`, `blake3`, `crc32c`, and the
// `cfg(fuzzing)`-gated `fuzz_validation` sub-module. None of these are
// Verus-modelable in a single-file `verus --crate-type=lib`
// invocation, and Rust 2024 let-chains in production require an
// edition flag not available in the unit-test invocation profile. The
// mirror sidesteps every blocker while preserving the codec
// discriminant set, field names, and fn signatures byte-for-byte, so
// any drift in field names, discriminant sets, or fn signatures
// breaks the `extern_vb_vzcuf_PS_003` Verus build.
//
// The `assume_specification` bridges in the companion spec file
// (`vb-vzcuf-PS-003.rs`) attach the production contracts to the
// `encode_record` and `decode_record` exec fns and the exec wrappers
// in that file exercise the bridges from `verus!` context, so the
// bridges are not used as vacuums.
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
// `production_inner/vb_vzcuf_PS_003_production.rs`. The mirror is the
// production surface for the codec entry points (see header above).
// Any drift in the mirror or in the production source breaks this
// Verus build.
//
// Note on the inner module name `production_inner`: this avoids the
// name collision with the outer `production` module that the spec
// file expects. The spec file's `assume_specification[
// production::encode_record ]` and the `let r = production::encode_record(...)`
// wrappers resolve through the `pub use` re-exports below, which
// surface `encode_record` / `decode_record` (and the spec-mode type
// aliases) at the top level of this extern file.
#[path = "production_inner/vb_vzcuf_PS_003_production.rs"]
pub mod production_inner;

} // verus!

// Re-export the production types and exec fns so the spec file can
// reference them as `production::SpecRecordKind` etc. The spec file
// does `#[path = "extern_vb_vzcuf_PS_003.rs"] mod production;` and
// then `assume_specification[ production::encode_record ]( ... )`,
// so `encode_record` / `decode_record` must be reachable at the top
// level of this extern module (i.e., as `production::encode_record`
// from the spec file's perspective).
pub use production_inner::{
    spec_kind_family_valid, decode_record, encode_record, EnforceKindParity,
    SpecJournalError, SpecJournalEvent, SpecNonJournalPayload, SpecRecordEnvelope,
    SpecRecordHeader, SpecRecordKind,
};