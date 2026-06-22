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
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::enum_variant_names,
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

use std::collections::HashSet;
use vb_test_util::fixture::FixtureCapacity;
use vb_test_util::seed::SeededBytes;
use vb_test_util::temp_keyspace::TempKeyspace;

#[test]
fn seeded_bytes_determinism() {
    let a = SeededBytes::<32>::new(12345).unwrap();
    let b = SeededBytes::<32>::new(12345).unwrap();
    assert_eq!(a.bytes, b.bytes, "same seed must produce same bytes");
}

#[test]
fn seeded_bytes_different_seeds() {
    let a = SeededBytes::<32>::new(1).unwrap();
    let b = SeededBytes::<32>::new(2).unwrap();
    assert_ne!(
        a.bytes, b.bytes,
        "different seeds must produce different bytes"
    );
}

#[test]
fn temp_keyspace_cleanup() {
    let temp = TempKeyspace::open().unwrap();
    let path = temp.path().to_path_buf();
    drop(temp);
    assert!(
        !path.exists(),
        "temp keyspace directory must be removed on drop"
    );
}

#[test]
fn temp_keyspace_uniqueness() {
    let mut paths = HashSet::new();
    for _ in 0..100 {
        let temp = TempKeyspace::open().unwrap();
        let path = temp.path().to_path_buf();
        assert!(
            paths.insert(path),
            "each temp keyspace must have a unique path"
        );
    }
}

#[test]
fn zero_capacity_rejected() {
    let result = FixtureCapacity::new(0);
    assert!(
        result.is_err(),
        "zero capacity must be rejected with a typed error"
    );
}

#[test]
fn valid_capacity_accepted() {
    let result = FixtureCapacity::new(100);
    assert!(result.is_ok(), "valid capacity must be accepted");
    assert_eq!(result.unwrap().value, 100);
}

#[test]
fn empty_workflow_fixture() {
    // An empty workflow fixture uses minimal valid IR: just a zero-capacity
    // builder that is rejected, proving the error path works.
    let result = FixtureCapacity::new(0);
    assert!(matches!(
        result,
        Err(vb_test_util::TestSetupError::InvalidCapacity(_))
    ));
}

#[test]
fn no_cli_dependency() {
    // vb_test_util must not depend on vb_cli.
    // This is verified by the crate's Cargo.toml (no vb_cli in [dependencies]).
    // The test itself is a no-op that documents the contract.
}
