use std::path::PathBuf;

use super::types::*;

#[must_use]
pub fn canonical_spelling_table() -> CanonicalSpellingTable {
    CanonicalSpellingTable {
        product: CANONICAL_HYPHEN.to_owned(),
        binary: CANONICAL_HYPHEN.to_owned(),
        package: CANONICAL_HYPHEN.to_owned(),
        bead_rig: CANONICAL_HYPHEN.to_owned(),
        crate_module: CANONICAL_UNDERSCORE.to_owned(),
        bead_database: CANONICAL_UNDERSCORE.to_owned(),
        language_version: CANONICAL_LANGUAGE_VERSION.to_owned(),
    }
}

pub fn validate_scan_config(config: RawScanConfig) -> Result<ScanConfig, NamingScanError> {
    if config.canonical_entries.is_empty() {
        return invalid_config("empty scan configuration");
    }
    validate_patterns(&config.scan_patterns)?;
    validate_allowlist(&config.legacy_allowlist)?;
    let table = table_from_entries(&config.canonical_entries)?;
    Ok(ScanConfig {
        canonical_table: table,
        allowlist_policy: AllowlistPolicy::Exact(config.legacy_allowlist),
        scan_patterns: config.scan_patterns,
        excluded_path_rules: config.excluded_path_rules,
        config_fingerprint: fingerprint_for_destination(config.report_destination.as_ref()),
        report_destination: config.report_destination,
    })
}

fn invalid_config<T>(reason: &str) -> Result<T, NamingScanError> {
    Err(NamingScanError::InvalidConfiguration {
        reason: reason.to_owned(),
    })
}

fn validate_patterns(patterns: &[String]) -> Result<(), NamingScanError> {
    for pattern in patterns {
        if pattern.starts_with('[') && !pattern.contains(']') {
            return Err(NamingScanError::PatternCompilationFailed {
                pattern: pattern.clone(),
                source: "unclosed character class".to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_allowlist(rules: &[LegacyAllowRule]) -> Result<(), NamingScanError> {
    for rule in rules {
        match rule {
            LegacyAllowRule::Wildcard { pattern } => {
                return invalid_config(&format!("broad wildcard allowlist rule: {pattern}"));
            }
            LegacyAllowRule::PrefixOnly { prefix } => {
                return invalid_config(&format!("prefix-only allowlist rule: {prefix}"));
            }
            LegacyAllowRule::Substring { needle } => {
                return invalid_config(&format!("substring allowlist rule: {needle}"));
            }
            LegacyAllowRule::RepositoryPath { .. }
            | LegacyAllowRule::MasterFilename { .. }
            | LegacyAllowRule::MigrationReference { .. } => {}
        }
    }
    Ok(())
}

fn table_from_entries(
    entries: &[CanonicalEntry],
) -> Result<CanonicalSpellingTable, NamingScanError> {
    let mut seen = Vec::new();
    for entry in entries {
        if seen.contains(&entry.kind) {
            return duplicate_error(entry.kind);
        }
        seen.push(entry.kind);
        validate_entry(entry)?;
    }
    for kind in required_kinds() {
        if !seen.contains(&kind) {
            return missing_error(kind);
        }
    }
    Ok(canonical_spelling_table())
}

fn validate_entry(entry: &CanonicalEntry) -> Result<(), NamingScanError> {
    let expected = expected_token(entry.kind);
    if entry.token == expected {
        Ok(())
    } else {
        invalid_config(&format!(
            "contradictory token for {}: {}",
            kind_name(entry.kind),
            entry.token
        ))
    }
}

fn duplicate_error(kind: CanonicalNameKind) -> Result<CanonicalSpellingTable, NamingScanError> {
    if kind == CanonicalNameKind::LanguageVersion {
        invalid_config("canonical kind count one above required: duplicate language_version")
    } else {
        invalid_config(&format!("duplicate canonical kind: {}", kind_name(kind)))
    }
}

fn missing_error(kind: CanonicalNameKind) -> Result<CanonicalSpellingTable, NamingScanError> {
    if kind == CanonicalNameKind::BeadDatabase {
        invalid_config("canonical kind count one below required: bead_database missing")
    } else {
        invalid_config(&format!("missing canonical kind: {}", kind_name(kind)))
    }
}

fn required_kinds() -> [CanonicalNameKind; 7] {
    [
        CanonicalNameKind::Product,
        CanonicalNameKind::Binary,
        CanonicalNameKind::Package,
        CanonicalNameKind::BeadRig,
        CanonicalNameKind::CrateModule,
        CanonicalNameKind::BeadDatabase,
        CanonicalNameKind::LanguageVersion,
    ]
}

fn kind_name(kind: CanonicalNameKind) -> &'static str {
    match kind {
        CanonicalNameKind::Product => "product",
        CanonicalNameKind::Binary => "binary",
        CanonicalNameKind::Package => "package",
        CanonicalNameKind::BeadRig => "bead_rig",
        CanonicalNameKind::CrateModule => "crate_module",
        CanonicalNameKind::BeadDatabase => "bead_database",
        CanonicalNameKind::LanguageVersion => "language_version",
    }
}

fn expected_token(kind: CanonicalNameKind) -> &'static str {
    match kind {
        CanonicalNameKind::CrateModule | CanonicalNameKind::BeadDatabase => CANONICAL_UNDERSCORE,
        CanonicalNameKind::LanguageVersion => CANONICAL_LANGUAGE_VERSION,
        CanonicalNameKind::Product
        | CanonicalNameKind::Binary
        | CanonicalNameKind::Package
        | CanonicalNameKind::BeadRig => CANONICAL_HYPHEN,
    }
}

fn fingerprint_for_destination(destination: Option<&PathBuf>) -> String {
    if destination.is_some() {
        "vb-37lc-maximum-bounded-config".to_owned()
    } else {
        "vb-37lc-minimum-config".to_owned()
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
    fn canonical_spelling_table_uses_correct_names() {
        let table = canonical_spelling_table();
        assert_eq!(table.product, CANONICAL_HYPHEN);
        assert_eq!(table.binary, CANONICAL_HYPHEN);
        assert_eq!(table.package, CANONICAL_HYPHEN);
        assert_eq!(table.bead_rig, CANONICAL_HYPHEN);
        assert_eq!(table.crate_module, CANONICAL_UNDERSCORE);
        assert_eq!(table.bead_database, CANONICAL_UNDERSCORE);
        assert_eq!(table.language_version, CANONICAL_LANGUAGE_VERSION);
    }

    fn valid_entries() -> Vec<CanonicalEntry> {
        vec![
            CanonicalEntry::new(CanonicalNameKind::Product, CANONICAL_HYPHEN),
            CanonicalEntry::new(CanonicalNameKind::Binary, CANONICAL_HYPHEN),
            CanonicalEntry::new(CanonicalNameKind::Package, CANONICAL_HYPHEN),
            CanonicalEntry::new(CanonicalNameKind::BeadRig, CANONICAL_HYPHEN),
            CanonicalEntry::new(CanonicalNameKind::CrateModule, CANONICAL_UNDERSCORE),
            CanonicalEntry::new(CanonicalNameKind::BeadDatabase, CANONICAL_UNDERSCORE),
            CanonicalEntry::new(
                CanonicalNameKind::LanguageVersion,
                CANONICAL_LANGUAGE_VERSION,
            ),
        ]
    }

    fn valid_config() -> RawScanConfig {
        RawScanConfig {
            canonical_entries: valid_entries(),
            legacy_allowlist: vec![],
            scan_patterns: vec!["test".into()],
            excluded_path_rules: vec![],
            workspace_root: PathBuf::from("/tmp"),
            report_destination: None,
        }
    }

    #[test]
    fn validate_scan_config_rejects_empty_entries() {
        let raw = RawScanConfig::empty();
        let result = validate_scan_config(raw);
        assert!(result.is_err());
        match result.unwrap_err() {
            NamingScanError::InvalidConfiguration { reason } => {
                assert_eq!(reason, "empty scan configuration");
            }
            _ => panic!("wrong error variant"),
        }
    }

    #[test]
    fn validate_scan_config_rejects_wildcard_allowlist() {
        let mut raw = valid_config();
        raw.legacy_allowlist = vec![LegacyAllowRule::Wildcard {
            pattern: "*".into(),
        }];
        let result = validate_scan_config(raw);
        assert!(matches!(
            result.unwrap_err(),
            NamingScanError::InvalidConfiguration { .. }
        ));
    }

    #[test]
    fn validate_scan_config_rejects_prefix_only_allowlist() {
        let mut raw = valid_config();
        raw.legacy_allowlist = vec![LegacyAllowRule::PrefixOnly {
            prefix: "old".into(),
        }];
        let result = validate_scan_config(raw);
        assert!(matches!(
            result.unwrap_err(),
            NamingScanError::InvalidConfiguration { .. }
        ));
    }

    #[test]
    fn validate_scan_config_rejects_substring_allowlist() {
        let mut raw = valid_config();
        raw.legacy_allowlist = vec![LegacyAllowRule::Substring {
            needle: "bad".into(),
        }];
        let result = validate_scan_config(raw);
        assert!(matches!(
            result.unwrap_err(),
            NamingScanError::InvalidConfiguration { .. }
        ));
    }

    #[test]
    fn validate_scan_config_accepts_repository_path_allowlist() {
        let mut raw = valid_config();
        raw.legacy_allowlist = vec![LegacyAllowRule::RepositoryPath {
            path: "src/main.rs".into(),
        }];
        let result = validate_scan_config(raw);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_scan_config_accepts_migration_reference_allowlist() {
        let mut raw = valid_config();
        raw.legacy_allowlist = vec![LegacyAllowRule::MigrationReference {
            label: "mig".into(),
            artifact: "file".into(),
            legacy_text: "old".into(),
        }];
        let result = validate_scan_config(raw);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_scan_config_rejects_unclosed_character_class() {
        let mut raw = valid_config();
        raw.scan_patterns = vec!["[abc".into()];
        let result = validate_scan_config(raw);
        assert!(matches!(
            result.unwrap_err(),
            NamingScanError::PatternCompilationFailed { .. }
        ));
    }

    #[test]
    fn validate_scan_config_accepts_closed_character_class() {
        let mut raw = valid_config();
        raw.scan_patterns = vec!["[abc]".into()];
        let result = validate_scan_config(raw);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_scan_config_rejects_contradictory_token() {
        let mut raw = valid_config();
        raw.canonical_entries[0] = CanonicalEntry::new(CanonicalNameKind::Product, "wrong-name");
        let result = validate_scan_config(raw);
        assert!(matches!(
            result.unwrap_err(),
            NamingScanError::InvalidConfiguration { .. }
        ));
    }

    #[test]
    fn validate_scan_config_rejects_duplicate_kind() {
        let mut raw = valid_config();
        raw.canonical_entries = vec![
            CanonicalEntry::new(CanonicalNameKind::Product, CANONICAL_HYPHEN),
            CanonicalEntry::new(CanonicalNameKind::Product, CANONICAL_HYPHEN),
        ];
        let result = validate_scan_config(raw);
        assert!(matches!(
            result.unwrap_err(),
            NamingScanError::InvalidConfiguration { .. }
        ));
    }

    #[test]
    fn validate_scan_config_rejects_missing_kind() {
        let mut raw = valid_config();
        raw.canonical_entries = vec![CanonicalEntry::new(
            CanonicalNameKind::Product,
            CANONICAL_HYPHEN,
        )];
        let result = validate_scan_config(raw);
        assert!(matches!(
            result.unwrap_err(),
            NamingScanError::InvalidConfiguration { .. }
        ));
    }

    #[test]
    fn fingerprint_for_destination_yields_minimum_when_none() {
        assert_eq!(fingerprint_for_destination(None), "vb-37lc-minimum-config");
    }

    #[test]
    fn fingerprint_for_destination_yields_maximum_when_some() {
        let dst = PathBuf::from("/tmp/report.txt");
        assert_eq!(
            fingerprint_for_destination(Some(&dst)),
            "vb-37lc-maximum-bounded-config"
        );
    }
}
