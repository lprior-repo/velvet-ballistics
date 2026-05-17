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
