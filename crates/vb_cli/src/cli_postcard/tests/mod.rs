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
//! - `typed_payloads_wave_c` covers the three "validate-fallback"
//!   per-command round-trips (SystemStatus, AiContextPacket,
//!   WorkflowDiffReport) and the shape-mismatch fallback negative test.
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

#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::io_other_error,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::manual_unwrap_or_default,
    clippy::map_clone,
    clippy::map_flatten,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::new_without_default,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_sort_by,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_asref,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]

mod envelopes;
mod errors;
mod round_trip;
mod typed_payloads;
mod typed_payloads_reports;
mod typed_payloads_wave_c;
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
