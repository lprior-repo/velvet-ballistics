#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::panic
)]
use super::types::*;

pub(crate) fn exact_exception(text: &str, config: &ScanConfig) -> Option<LegacyException> {
    match &config.allowlist_policy {
        AllowlistPolicy::Exact(rules) => {
            rules.iter().find_map(|rule| exception_for_rule(text, rule))
        }
    }
}

fn exception_for_rule(text: &str, rule: &LegacyAllowRule) -> Option<LegacyException> {
    match rule {
        LegacyAllowRule::RepositoryPath { path } if text == path => {
            Some(LegacyException::RepositoryPath { path: path.clone() })
        }
        LegacyAllowRule::MasterFilename { filename } if text == filename => {
            Some(LegacyException::MasterFilename {
                filename: filename.clone(),
            })
        }
        LegacyAllowRule::MigrationReference {
            label,
            artifact,
            legacy_text,
        } if text == format!("{label} {artifact} {legacy_text}") => {
            Some(migration_exception(label, artifact, legacy_text))
        }
        LegacyAllowRule::RepositoryPath { .. }
        | LegacyAllowRule::MasterFilename { .. }
        | LegacyAllowRule::MigrationReference { .. }
        | LegacyAllowRule::Wildcard { .. }
        | LegacyAllowRule::PrefixOnly { .. }
        | LegacyAllowRule::Substring { .. } => None,
    }
}

fn migration_exception(label: &str, artifact: &str, legacy_text: &str) -> LegacyException {
    LegacyException::MigrationReference {
        artifact: artifact.to_owned(),
        label: label.to_owned(),
        legacy_text: legacy_text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_rules(rules: Vec<LegacyAllowRule>) -> ScanConfig {
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
            allowlist_policy: AllowlistPolicy::Exact(rules),
            scan_patterns: vec![],
            excluded_path_rules: vec![],
            config_fingerprint: "test".into(),
            report_destination: None,
        }
    }

    #[test]
    fn exact_exception_returns_none_for_empty_rules() {
        let cfg = config_with_rules(vec![]);
        assert!(exact_exception("anything", &cfg).is_none());
    }

    #[test]
    fn exact_exception_matches_repository_path() {
        let cfg = config_with_rules(vec![LegacyAllowRule::RepositoryPath {
            path: "src/old.rs".into(),
        }]);
        let ex = exact_exception("src/old.rs", &cfg).unwrap();
        assert_eq!(
            ex,
            LegacyException::RepositoryPath {
                path: "src/old.rs".into()
            }
        );
    }

    #[test]
    fn exact_exception_matches_master_filename() {
        let cfg = config_with_rules(vec![LegacyAllowRule::MasterFilename {
            filename: "legacy.md".into(),
        }]);
        let ex = exact_exception("legacy.md", &cfg).unwrap();
        assert_eq!(
            ex,
            LegacyException::MasterFilename {
                filename: "legacy.md".into()
            }
        );
    }

    #[test]
    fn exact_exception_matches_migration_reference() {
        let cfg = config_with_rules(vec![LegacyAllowRule::MigrationReference {
            label: "mig1".into(),
            artifact: "file.toml".into(),
            legacy_text: "old-name".into(),
        }]);
        let ex = exact_exception("mig1 file.toml old-name", &cfg).unwrap();
        match ex {
            LegacyException::MigrationReference {
                label,
                artifact,
                legacy_text,
            } => {
                assert_eq!(label, "mig1");
                assert_eq!(artifact, "file.toml");
                assert_eq!(legacy_text, "old-name");
            }
            _ => panic!("wrong exception variant"),
        }
    }

    #[test]
    fn exact_exception_returns_none_for_mismatched_repository_path() {
        let cfg = config_with_rules(vec![LegacyAllowRule::RepositoryPath {
            path: "src/old.rs".into(),
        }]);
        assert!(exact_exception("src/new.rs", &cfg).is_none());
    }

    #[test]
    fn exact_exception_returns_none_for_wildcard_rule() {
        let cfg = config_with_rules(vec![LegacyAllowRule::Wildcard {
            pattern: "*".into(),
        }]);
        assert!(exact_exception("anything", &cfg).is_none());
    }

    #[test]
    fn exact_exception_returns_none_for_prefix_only_rule() {
        let cfg = config_with_rules(vec![LegacyAllowRule::PrefixOnly {
            prefix: "old-".into(),
        }]);
        assert!(exact_exception("old-name", &cfg).is_none());
    }
}
