/// Validation pipeline for scan configuration.
///
/// Takes a [`RawScanConfig`] and produces a [`ScanConfig`], checking
/// allowlist rules, scan patterns, and entry completeness in one
/// deterministic pipeline.
use super::config_build::table_from_entries;
use super::config_types::*;
use super::types::*;

/// Validate and build a [`ScanConfig`] from raw input.
///
/// This is the single entry point for turning untrusted configuration
/// into a trusted, fully-validated [`ScanConfig`].
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

/// Validate that every scan pattern has balanced brackets.
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

/// Validate the legacy allowlist, rejecting overly broad rules.
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
