//! CLI Postcard Test Submodule Root
//!
//! vb-k8ut.5: the 751-line monolithic `tests.rs` is split into five
//! concern-grouped submodules so the source-length gate stays under the
//! 300-line cap. All 33 `#[test]` functions are preserved verbatim and
//! continue to import the same `super::*` items plus
//! `crate::cli_envelope::Kind` and `crate::exit_code::CliExitCode`.
//!
//! - `round_trip` covers encode/decode infrastructure: magic/header
//!   constants, header parsing, encode/decode round-trips, and the
//!   version-too-old/new negative path.
//! - `typed_payloads` covers the four "core" per-command round-trips
//!   (Diagnostic, Validate, Events, Replay).
//! - `typed_payloads_reports` covers the four "multi-section"
//!   per-command round-trips (Verify, Explain, Trace, Diff).
//! - `wire_format` covers the typed `bool` / typed `String` kind-tag
//!   wire-format contract.
//! - `errors` covers malformed-header rejection, CRC/digest mismatches,
//!   version-too-old/new, wrong-kind rejection, payload-too-large,
//!   truncated-header, and garbage-bytes rejection.
//! - `envelopes` covers envelope-shape classification: known-kind routing,
//!   unknown-kind rejection, missing-kind-field rejection, generic
//!   migration-fallback, and the `CliPostcardKind` discriminant /
//!   `From<EnvelopeKind>` tables.
//!
//! Shared test fixtures live in this file as `pub(super)` so the sibling
//! submodules can reuse them through `super::encode_test_postcard`,
//! `super::write_test_header_prefix`, and `super::write_test_bytes`.

#![allow(dead_code)]

mod envelopes;
mod errors;
mod round_trip;
mod typed_payloads;
mod typed_payloads_reports;
mod wire_format;

use super::{CLI_MAGIC, CLI_POSTCARD_KIND, CLI_SCHEMA_VERSION, HEADER_SIZE_U32};

pub(super) fn encode_test_postcard(schema_version: u16, kind: u16, payload: &[u8]) -> Vec<u8> {
    super::encode_postcard(schema_version, kind, payload).expect("test postcard encodes")
}

pub(super) fn write_test_header_prefix(data: &mut [u8], payload_len: u32) {
    write_test_bytes(data, 0..4, &CLI_MAGIC);
    write_test_bytes(data, 4..6, &CLI_SCHEMA_VERSION.to_le_bytes());
    write_test_bytes(data, 6..8, &CLI_POSTCARD_KIND.to_le_bytes());
    write_test_bytes(data, 8..12, &HEADER_SIZE_U32.to_le_bytes());
    write_test_bytes(data, 12..16, &payload_len.to_le_bytes());
}

pub(super) fn write_test_bytes(data: &mut [u8], range: std::ops::Range<usize>, bytes: &[u8]) {
    assert_eq!(range.len(), bytes.len());
    assert!(data.get_mut(range.clone()).is_some());
    if let Some(target) = data.get_mut(range) {
        target.copy_from_slice(bytes);
    }
}
