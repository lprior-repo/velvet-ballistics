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
