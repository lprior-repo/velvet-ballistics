#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]

//! CLI Postcard Round-Trip Tests (Infrastructure)
//!
//! vb-k8ut.5: encode/decode round-trips, magic/header constants, header
//! parsing, and the version-too-old/new negative path. The per-command
//! typed payload round-trips live in `typed_payloads.rs`.

use super::super::*;
use super::encode_test_postcard;

#[test]
fn test_valid_magic() {
    assert_eq!(CLI_MAGIC, [0x56, 0x43, 0x4C, 0x41]);
    assert_eq!(CLI_MAGIC, *b"VCLA");
}

#[test]
fn test_max_payload() {
    assert_eq!(MAX_PAYLOAD, 65536);
}

#[test]
fn test_header_size() {
    assert_eq!(HEADER_SIZE, 52);
}

#[test]
fn test_postcard_header_from_bytes() {
    let data = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, &[0u8; 100]);

    let header = PostcardHeader::from_bytes(&data).expect("test header decodes");
    assert_eq!(header.magic, CLI_MAGIC);
    assert_eq!(header.schema_version, CLI_SCHEMA_VERSION);
    assert_eq!(header.kind, CLI_POSTCARD_KIND);
    assert_eq!(header.header_len, HEADER_SIZE_U32);
    assert_eq!(header.payload_len, 100);
}

#[test]
fn test_decode_valid_postcard() {
    let data = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, &[0u8; 100]);

    let (header, payload) = decode_postcard(&data).expect("valid postcard decodes");
    assert_eq!(header.len(), HEADER_SIZE);
    assert_eq!(payload.len(), 100);
}

#[test]
fn test_encode_postcard() {
    let payload = b"test payload";
    let encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, payload);

    assert_eq!(encoded.get(0..4), Some(CLI_MAGIC.as_slice()));
    assert_eq!(encoded.len(), HEADER_SIZE + payload.len());
}

#[test]
fn test_roundtrip() {
    let payload = b"Hello, Postcard!";
    let encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, payload);

    let (header, extracted_payload) = decode_postcard(&encoded).expect("roundtrip decodes");
    assert_eq!(header.len(), HEADER_SIZE);
    assert_eq!(extracted_payload, payload);
}

#[test]
fn decode_rejects_old_and_future_versions() {
    let old = encode_test_postcard(0, CLI_POSTCARD_KIND, b"payload");
    let future = encode_postcard(
        CLI_SCHEMA_VERSION.saturating_add(1),
        CLI_POSTCARD_KIND,
        b"payload",
    )
    .expect("future-version postcard encodes");
    assert_eq!(decode_postcard(&old), Err(PostcardError::VersionTooOld));
    assert_eq!(decode_postcard(&future), Err(PostcardError::VersionTooNew));
}
