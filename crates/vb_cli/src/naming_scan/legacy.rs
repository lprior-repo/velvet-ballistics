use super::types::*;

pub(crate) fn contains_legacy(text: &str) -> bool {
    text.contains(LEGACY_PROJECT)
        || text.contains(LEGACY_CRATE)
        || text.contains(LEGACY_LANGUAGE_VERSION)
        || text.contains("Velvet-Ballastics")
        || text.contains("VELVET-BALLASTICS")
        || text.contains("velvet‐ballistics")
}

pub(crate) fn class_for_text(text: &str) -> OccurrenceClass {
    if text.contains(LEGACY_CRATE) {
        OccurrenceClass::InvalidLegacy {
            spelling_class: SpellingClass::LegacyCrateModuleSpelling,
            remediation: CANONICAL_UNDERSCORE.to_owned(),
        }
    } else if text.contains(LEGACY_LANGUAGE_VERSION) {
        OccurrenceClass::InvalidLegacy {
            spelling_class: SpellingClass::LegacyLanguageVersionSpelling,
            remediation: CANONICAL_LANGUAGE_VERSION.to_owned(),
        }
    } else {
        OccurrenceClass::InvalidLegacy {
            spelling_class: SpellingClass::LegacyProjectSpelling,
            remediation: CANONICAL_HYPHEN.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
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
    unused_variables,
)]

    use super::*;

    #[test]
    fn contains_legacy_returns_true_for_legacy_project() {
        assert!(contains_legacy("velvet-ballistics is bad"));
    }

    #[test]
    fn contains_legacy_returns_true_for_legacy_crate() {
        assert!(contains_legacy("velvet_ballistics crate"));
    }

    #[test]
    fn contains_legacy_returns_true_for_legacy_language_version() {
        assert!(contains_legacy("velvet-ballistics/v1"));
    }

    #[test]
    fn contains_legacy_returns_true_for_pascal_case_legacy() {
        assert!(contains_legacy("Velvet-Ballastics rule"));
    }

    #[test]
    fn contains_legacy_returns_true_for_uppercase_legacy() {
        assert!(contains_legacy("VELVET-BALLASTICS"));
    }

    #[test]
    fn contains_legacy_returns_false_for_clean_text() {
        assert!(!contains_legacy("this is clean"));
    }

    #[test]
    fn contains_legacy_returns_false_for_canonical_hyphen() {
        assert!(!contains_legacy("velvet-ballastics"));
    }

    #[test]
    fn class_for_text_returns_legacy_crate_module_spelling() {
        let result = class_for_text("velvet_ballistics is wrong");
        match result {
            OccurrenceClass::InvalidLegacy {
                spelling_class,
                remediation,
            } => {
                assert_eq!(spelling_class, SpellingClass::LegacyCrateModuleSpelling);
                assert_eq!(remediation, CANONICAL_UNDERSCORE);
            }
            _ => panic!("wrong occurrence class"),
        }
    }

    #[test]
    fn class_for_text_returns_legacy_language_version_spelling() {
        let result = class_for_text("use velvet-ballistics/v1 here");
        match result {
            OccurrenceClass::InvalidLegacy {
                spelling_class,
                remediation,
            } => {
                assert_eq!(spelling_class, SpellingClass::LegacyLanguageVersionSpelling);
                assert_eq!(remediation, CANONICAL_LANGUAGE_VERSION);
            }
            _ => panic!("wrong occurrence class"),
        }
    }

    #[test]
    fn class_for_text_returns_legacy_project_spelling() {
        let result = class_for_text("velvet-ballistics is the old name");
        match result {
            OccurrenceClass::InvalidLegacy {
                spelling_class,
                remediation,
            } => {
                assert_eq!(spelling_class, SpellingClass::LegacyProjectSpelling);
                assert_eq!(remediation, CANONICAL_HYPHEN);
            }
            _ => panic!("wrong occurrence class"),
        }
    }
}
