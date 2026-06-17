#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
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
    clippy::wildcard_imports
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
