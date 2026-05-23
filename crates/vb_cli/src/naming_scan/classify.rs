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
    if let Some(canonical) = canonical_occurrence(text, &config.canonical_table) {
        return Ok(canonical);
    }
    if let Some(exception) = exact_exception(text, config) {
        return Ok(OccurrenceClass::AllowedLegacy { exception });
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
            RepoPath::new("test"), LineNumber::new(1), ColumnNumber::new(1), "", &test_config(),
        ).unwrap();
        assert_eq!(result, OccurrenceClass::NoOccurrence);
    }

    #[test]
    fn classify_occurrence_returns_canonical_for_product_spelling() {
        let result = classify_occurrence(
            RepoPath::new("test.rs"), LineNumber::new(1), ColumnNumber::new(1),
            CANONICAL_HYPHEN, &test_config(),
        ).unwrap();
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
            RepoPath::new("test.rs"), LineNumber::new(1), ColumnNumber::new(1),
            CANONICAL_UNDERSCORE, &test_config(),
        ).unwrap();
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
            RepoPath::new("test.rs"), LineNumber::new(1), ColumnNumber::new(1),
            CANONICAL_LANGUAGE_VERSION, &test_config(),
        ).unwrap();
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
        cfg.allowlist_policy = AllowlistPolicy::Exact(vec![
            LegacyAllowRule::RepositoryPath { path: "velvet-ballistics".into() },
        ]);
        let result = classify_occurrence(
            RepoPath::new("test.rs"), LineNumber::new(1), ColumnNumber::new(1),
            "velvet-ballistics", &cfg,
        ).unwrap();
        match result {
            OccurrenceClass::AllowedLegacy { .. } => {}
            _ => panic!("expected AllowedLegacy"),
        }
    }

    #[test]
    fn classify_occurrence_returns_invalid_legacy_for_unrecognized_legacy_text() {
        let cfg = test_config();
        let result = classify_occurrence(
            RepoPath::new("test.rs"), LineNumber::new(1), ColumnNumber::new(1),
            "velvet-ballistics is bad", &cfg,
        ).unwrap();
        match result {
            OccurrenceClass::InvalidLegacy { .. } => {}
            _ => panic!("expected InvalidLegacy"),
        }
    }

    #[test]
    fn classify_occurrence_returns_no_occurrence_for_unrelated_text() {
        let cfg = test_config();
        let result = classify_occurrence(
            RepoPath::new("test.rs"), LineNumber::new(1), ColumnNumber::new(1),
            "completely unrelated text", &cfg,
        ).unwrap();
        assert_eq!(result, OccurrenceClass::NoOccurrence);
    }
}
