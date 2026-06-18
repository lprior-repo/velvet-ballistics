use super::allowlist::exact_exception;
use super::legacy::{class_for_text, contains_legacy};
use super::types::*;

pub fn classify_occurrence(
    _path: RepoPath,
    _line: LineNumber,
    _column: ColumnNumber,
    text: &str,
    config: &ScanConfig,
) -> Result<OccurrenceClass, NamingScanError> {
    if text.is_empty() {
        return Ok(OccurrenceClass::NoOccurrence);
    }
    if let Some(exception) = exact_exception(text, config) {
        return Ok(OccurrenceClass::AllowedLegacy { exception });
    }
    if let Some(canonical) = canonical_occurrence(text, &config.canonical_table) {
        return Ok(canonical);
    }
    if contains_legacy(text) {
        return Ok(class_for_text(text));
    }
    Ok(OccurrenceClass::NoOccurrence)
}

fn canonical_occurrence(text: &str, table: &CanonicalSpellingTable) -> Option<OccurrenceClass> {
    if text == table.language_version {
        return Some(canonical_language_version(text));
    }
    if text == table.crate_module || text == table.bead_database {
        return Some(canonical_crate_module(text));
    }
    if text == table.product
        || text == table.binary
        || text == table.package
        || text == table.bead_rig
    {
        return Some(canonical_product(text));
    }
    None
}

fn canonical_language_version(text: &str) -> OccurrenceClass {
    OccurrenceClass::CanonicalLanguageVersion {
        canonical: text.to_owned(),
        kind: CanonicalNameKind::LanguageVersion,
    }
}

fn canonical_crate_module(text: &str) -> OccurrenceClass {
    OccurrenceClass::CanonicalCrateModule {
        canonical: text.to_owned(),
        kind: CanonicalNameKind::CrateModule,
    }
}

fn canonical_product(text: &str) -> OccurrenceClass {
    OccurrenceClass::CanonicalProduct {
        canonical: text.to_owned(),
        kind: CanonicalNameKind::Product,
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
        unused_variables
    )]

    use super::*;

    fn test_config() -> ScanConfig {
        ScanConfig {
            canonical_table: CanonicalSpellingTable {
                product: CANONICAL_HYPHEN.to_owned(),
                binary: CANONICAL_HYPHEN.to_owned(),
                package: CANONICAL_HYPHEN.to_owned(),
                bead_rig: CANONICAL_HYPHEN.to_owned(),
                crate_module: CANONICAL_UNDERSCORE.to_owned(),
                bead_database: CANONICAL_UNDERSCORE.to_owned(),
                language_version: CANONICAL_LANGUAGE_VERSION.to_owned(),
            },
            allowlist_policy: AllowlistPolicy::Exact(vec![]),
            scan_patterns: vec![],
            excluded_path_rules: vec![],
            config_fingerprint: "test".into(),
            report_destination: None,
        }
    }

    #[test]
    fn classify_occurrence_returns_no_occurrence_for_empty_text() {
        let result = classify_occurrence(
            RepoPath::new("test"),
            LineNumber::new(1),
            ColumnNumber::new(1),
            "",
            &test_config(),
        )
        .unwrap();
        assert_eq!(result, OccurrenceClass::NoOccurrence);
    }

    #[test]
    fn classify_occurrence_returns_canonical_for_product_spelling() {
        let result = classify_occurrence(
            RepoPath::new("test.rs"),
            LineNumber::new(1),
            ColumnNumber::new(1),
            CANONICAL_HYPHEN,
            &test_config(),
        )
        .unwrap();
        match result {
            OccurrenceClass::CanonicalProduct { canonical, .. } => {
                assert_eq!(canonical, CANONICAL_HYPHEN);
            }
            _ => panic!("expected CanonicalProduct"),
        }
    }

    #[test]
    fn classify_occurrence_returns_canonical_for_crate_module_spelling() {
        let result = classify_occurrence(
            RepoPath::new("test.rs"),
            LineNumber::new(1),
            ColumnNumber::new(1),
            CANONICAL_UNDERSCORE,
            &test_config(),
        )
        .unwrap();
        match result {
            OccurrenceClass::CanonicalCrateModule { canonical, .. } => {
                assert_eq!(canonical, CANONICAL_UNDERSCORE);
            }
            _ => panic!("expected CanonicalCrateModule"),
        }
    }

    #[test]
    fn classify_occurrence_returns_canonical_for_language_version_spelling() {
        let result = classify_occurrence(
            RepoPath::new("test.rs"),
            LineNumber::new(1),
            ColumnNumber::new(1),
            CANONICAL_LANGUAGE_VERSION,
            &test_config(),
        )
        .unwrap();
        match result {
            OccurrenceClass::CanonicalLanguageVersion { canonical, .. } => {
                assert_eq!(canonical, CANONICAL_LANGUAGE_VERSION);
            }
            _ => panic!("expected CanonicalLanguageVersion"),
        }
    }

    #[test]
    fn classify_occurrence_returns_allowed_legacy_for_exact_exception() {
        let mut cfg = test_config();
        let legacy_path = "legacy/path/velvet_ballistics";
        cfg.allowlist_policy = AllowlistPolicy::Exact(vec![LegacyAllowRule::RepositoryPath {
            path: legacy_path.into(),
        }]);
        let result = classify_occurrence(
            RepoPath::new("test.rs"),
            LineNumber::new(1),
            ColumnNumber::new(1),
            legacy_path,
            &cfg,
        )
        .unwrap();
        match result {
            OccurrenceClass::AllowedLegacy { .. } => {}
            _ => panic!("expected AllowedLegacy"),
        }
    }

    #[test]
    fn classify_occurrence_returns_invalid_legacy_for_unrecognized_legacy_text() {
        let cfg = test_config();
        let result = classify_occurrence(
            RepoPath::new("test.rs"),
            LineNumber::new(1),
            ColumnNumber::new(1),
            "velvet-ballistics is bad",
            &cfg,
        )
        .unwrap();
        match result {
            OccurrenceClass::InvalidLegacy { .. } => {}
            _ => panic!("expected InvalidLegacy"),
        }
    }

    #[test]
    fn classify_occurrence_returns_no_occurrence_for_unrelated_text() {
        let cfg = test_config();
        let result = classify_occurrence(
            RepoPath::new("test.rs"),
            LineNumber::new(1),
            ColumnNumber::new(1),
            "completely unrelated text",
            &cfg,
        )
        .unwrap();
        assert_eq!(result, OccurrenceClass::NoOccurrence);
    }
}
