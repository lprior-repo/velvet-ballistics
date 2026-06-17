#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
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
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
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

//! MockMarker discriminant tests.
//!
//! These tests describe the expected behavior of MockMarker once implemented.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-001, RFO-002
//! Contract clauses: C-MOCK-1 (variant count), C-MOCK-2 (unit variants)

#![forbid(unsafe_code)]
#![cfg(feature = "vb-rxru0-mock-marker")]

use vb_core::action::MockMarker;

#[test]
fn test_mock_marker_three_variants() {
    // Exhaustiveness: every MockMarker value matches one of the three variants.
    fn exhaustiveness(m: MockMarker) {
        match m {
            MockMarker::GithubIssueCreate => {}
            MockMarker::AiClassifyTicket => {}
            MockMarker::HttpGet => {}
        }
    }

    exhaustiveness(MockMarker::GithubIssueCreate);
    exhaustiveness(MockMarker::AiClassifyTicket);
    exhaustiveness(MockMarker::HttpGet);

    // Discriminant uniqueness: each variant has a distinct discriminant.
    assert_ne!(
        MockMarker::GithubIssueCreate as u8,
        MockMarker::AiClassifyTicket as u8,
        "GithubIssueCreate and AiClassifyTicket must have different discriminants"
    );
    assert_ne!(
        MockMarker::AiClassifyTicket as u8,
        MockMarker::HttpGet as u8,
        "AiClassifyTicket and HttpGet must have different discriminants"
    );
    assert_ne!(
        MockMarker::GithubIssueCreate as u8,
        MockMarker::HttpGet as u8,
        "GithubIssueCreate and HttpGet must have different discriminants"
    );

    // Discriminant values are 0, 1, 2 in declaration order.
    assert_eq!(
        MockMarker::GithubIssueCreate as u8,
        0,
        "GithubIssueCreate must have discriminant 0"
    );
    assert_eq!(
        MockMarker::AiClassifyTicket as u8,
        1,
        "AiClassifyTicket must have discriminant 1"
    );
    assert_eq!(
        MockMarker::HttpGet as u8,
        2,
        "HttpGet must have discriminant 2"
    );
}

#[test]
fn test_mock_marker_copy_trait() {
    // Compile-time: Copy trait must exist.
    fn compile_copy(m: MockMarker) -> MockMarker {
        let _copied = m; // Copy: m is moved, not borrowed
        m // m can be returned because it was copied
    }

    let m = MockMarker::GithubIssueCreate;
    let _returned = compile_copy(m);

    // Size check: all variants are zero-sized (unit), repr(u8).
    assert_eq!(
        std::mem::size_of::<MockMarker>(),
        1,
        "MockMarker size must be 1 byte (repr(u8) with 3 unit variants)"
    );

    // Copy implies Clone.
    let m = MockMarker::GithubIssueCreate;
    let _cloned = m.clone();
    let _copied2 = m; // m not moved — Copy
}
