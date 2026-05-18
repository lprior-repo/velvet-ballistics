#![forbid(unsafe_code)]

//! Contract discovery and validation for the contracts-as-data suite.
//!
//! Walks the `contracts/` directory, validates `schema_version` and `kind` fields,
//! runs `cue vet` on each file, and produces a `DiscoveryReport`.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use vb_validate::{ValidationError, ValidationResult};

/// Closed set of contract kinds recognized by the discovery pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ContractKind {
    CliEnvelope,
    UiTokens,
    AcceptedArtifacts,
    EvidenceBundle,
    Diagnostics,
    GateOutput,
}

impl ContractKind {
    /// All valid contract kind values in canonical order.
    pub const fn all_values() -> &'static [Self] {
        &[
            Self::CliEnvelope,
            Self::UiTokens,
            Self::AcceptedArtifacts,
            Self::EvidenceBundle,
            Self::Diagnostics,
            Self::GateOutput,
        ]
    }

    /// Convert a string slice to the corresponding `ContractKind`.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "cli_envelope" => Some(Self::CliEnvelope),
            "ui_tokens" => Some(Self::UiTokens),
            "accepted_artifacts" => Some(Self::AcceptedArtifacts),
            "evidence_bundle" => Some(Self::EvidenceBundle),
            "diagnostics" => Some(Self::Diagnostics),
            "gate_output" => Some(Self::GateOutput),
            _ => None,
        }
    }

    /// Convert self to the CUE string representation.
    pub fn cue_str(&self) -> &'static str {
        match self {
            Self::CliEnvelope => "cli_envelope",
            Self::UiTokens => "ui_tokens",
            Self::AcceptedArtifacts => "accepted_artifacts",
            Self::EvidenceBundle => "evidence_bundle",
            Self::Diagnostics => "diagnostics",
            Self::GateOutput => "gate_output",
        }
    }
}

impl fmt::Display for ContractKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.cue_str())
    }
}

/// Metadata extracted from the top-level CUE schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractMeta {
    pub schema_version: Option<String>,
    pub kind: Option<String>,
}

/// A single contract file discovered and validated by the pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractFile {
    pub path: PathBuf,
    pub schema_version: String,
    pub kind: ContractKind,
    pub vet_errors: Vec<String>,
}

/// A version monotonicity violation found during discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionViolation {
    pub file: PathBuf,
    pub expected: String,
    pub actual: String,
    pub detail: String,
}

/// Summary statistics for a discovery run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total: u32,
    pub valid: u32,
    pub invalid: u32,
    pub errors_by_kind: BTreeMap<ContractKind, u32>,
    pub version_violations: Vec<VersionViolation>,
}

/// Full discovery report produced by walking the contracts directory.
///
/// The `errors` field stores formatted error messages for JSON serializability.
/// The `raw_errors` field is skipped in JSON output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryReport {
    pub files: Vec<ContractFile>,
    pub errors: Vec<String>,
    #[serde(skip)]
    pub raw_errors: Vec<ValidationError>,
    pub summary: ReportSummary,
}

/// Parse the top-level `schema_version` and `kind` fields from CUE file content.
///
/// Looks for lines matching `schema_version: <value>` or `kind: "<value>"`
/// at the top level (not indented within a type definition).
pub fn parse_contract_meta(content: &str) -> ContractMeta {
    let mut schema_version: Option<String> = None;
    let mut kind: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        // Match schema_version: <value> (string without quotes)
        if trimmed.starts_with("schema_version:") {
            let val = trimmed.strip_prefix("schema_version:").unwrap_or("").trim();
            // Strip surrounding quotes if present
            let val = val.trim_matches('"');
            // CUE type annotations (string, int, etc.) are not literal values
            let cue_type_keywords = ["string", "int", "int64", "float64", "bool", "list", "map", "struct", "duration", "bytes"];
            if !val.is_empty() && !cue_type_keywords.contains(&val) {
                schema_version = Some(val.to_string());
            }
            continue;
        }

        // Match kind: "value" (string with quotes)
        if trimmed.starts_with("kind:") {
            let val = trimmed.strip_prefix("kind:").unwrap_or("").trim();
            // Strip surrounding quotes
            let val = val.trim_matches('"');
            if !val.is_empty() {
                kind = Some(val.to_string());
            }
            continue;
        }
    }

    ContractMeta {
        schema_version,
        kind,
    }
}

/// Compare two semver-like version strings: (a, b) -> Ordering.
///
/// Returns `Ok(Ordering)` when both versions are well-formed semver.
/// Returns `Err` when either version cannot be parsed as semver.
///
/// This is the function that OBL-004 (Verus spec) binds to.
pub fn compare_semver(a: &str, b: &str) -> Result<std::cmp::Ordering, ValidationError> {
    let parse_parts = |s: &str| -> Option<(u64, u64, u64)> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let major = parts[0].parse::<u64>().ok()?;
        let minor = parts[1].parse::<u64>().ok()?;
        let patch = parts[2].parse::<u64>().ok()?;
        Some((major, minor, patch))
    };

    let va = parse_parts(a).ok_or(ValidationError::InvalidVersion {
        version: a.to_string(),
    })?;
    let vb = parse_parts(b).ok_or(ValidationError::InvalidVersion {
        version: b.to_string(),
    })?;

    // Compare major, minor, patch in order.
    // Using saturating arithmetic to avoid overflow.
    let cmp = if va.0 != vb.0 {
        va.0.cmp(&vb.0)
    } else if va.1 != vb.1 {
        va.1.cmp(&vb.1)
    } else {
        va.2.cmp(&vb.2)
    };

    Ok(cmp)
}

/// Run `cue vet` on a single file and return exit code + error lines.
///
/// Returns `Ok((exit_code, errors))` where `exit_code` is 0 on success.
/// This function never panics.
pub fn run_cue_vet(file_path: &Path) -> (i32, Vec<String>) {
    let output = Command::new("cue").arg("vet").arg(file_path).output();

    match output {
        Ok(out) => {
            let exit_code = out.status.code().unwrap_or(1);
            let mut errors = Vec::new();
            if !out.stderr.is_empty() {
                for line in out.stderr.split(|&b| b == b'\n') {
                    let text = String::from_utf8_lossy(line);
                    if !text.trim().is_empty() {
                        errors.push(text.to_string());
                    }
                }
            }
            if !out.stdout.is_empty() {
                for line in out.stdout.split(|&b| b == b'\n') {
                    let text = String::from_utf8_lossy(line);
                    if !text.trim().is_empty() {
                        errors.push(text.to_string());
                    }
                }
            }
            errors.sort();
            (exit_code, errors)
        }
        Err(_) => (1, vec!["cue: command not found".to_string()]),
    }
}

/// Validate a single contract file and return `ContractFile` or errors.
pub fn validate_contract_file(
    file_path: &Path,
    manifest: &BTreeMap<String, (String, ContractKind)>,
) -> Result<ContractFile, Vec<ValidationError>> {
    let content = std::fs::read_to_string(file_path).map_err(|e| {
        vec![ValidationError::MissingRequiredField {
            field: format!("read {}: {}", file_path.display(), e),
        }]
    })?;

    let meta = parse_contract_meta(&content);

    let mut errors = Vec::new();

    // Check schema_version present
    let schema_version = match meta.schema_version {
        Some(v) => {
            // Validate semver format
            if !is_valid_semver(&v) {
                errors.push(ValidationError::InvalidVersion { version: v.clone() });
            }
            v
        }
        None => {
            errors.push(ValidationError::MissingSchemaVersion);
            String::new()
        }
    };

    // Check kind present and valid
    let kind_str = match &meta.kind {
        Some(k) => k.clone(),
        None => {
            errors.push(ValidationError::MissingRequiredField {
                field: "kind".to_string(),
            });
            String::new()
        }
    };

    let kind = match ContractKind::from_str(&kind_str) {
        Some(k) => k,
        None => {
            errors.push(ValidationError::InvalidKind {
                kind: kind_str.clone(),
            });
            // Use CliEnvelope as fallback for struct construction
            return Err(errors);
        }
    };

    // Check monotonicity against manifest
    let rel_path = file_path.to_string_lossy().to_string();
    if let Some((prev_version, prev_kind)) = manifest.get(&rel_path) {
        if prev_kind != &kind {
            errors.push(ValidationError::CapabilityActionMismatch {
                contract_action_id: 0,
                capability_action_id: 0,
                capability_index: 0,
            });
        }

        let prev_ver_str = prev_version.clone();
        let curr_ver_str = schema_version.clone();
        match compare_semver(&prev_ver_str, &curr_ver_str) {
            Ok(std::cmp::Ordering::Less) | Ok(std::cmp::Ordering::Equal) => {
                errors.push(ValidationError::VersionMonotonicityBreach {
                    file: file_path.to_string_lossy().to_string(),
                    expected: format!("greater than {prev_ver_str}"),
                    actual: curr_ver_str,
                });
            }
            Ok(std::cmp::Ordering::Greater) => {}
            Err(_) => {
                errors.push(ValidationError::VersionMonotonicityBreach {
                    file: file_path.to_string_lossy().to_string(),
                    expected: format!("semver greater than {prev_ver_str}"),
                    actual: curr_ver_str,
                });
            }
        }
    }

    // Run cue vet
    let (vet_exit, vet_errors) = run_cue_vet(file_path);
    if vet_exit != 0 {
        errors.push(ValidationError::CueVetFailed {
            file: file_path.to_string_lossy().to_string(),
        });
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(ContractFile {
        path: file_path.to_path_buf(),
        schema_version,
        kind,
        vet_errors,
    })
}

/// Check if a string is a valid semver-like version (major.minor.patch).
fn is_valid_semver(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts.iter().all(|p| p.parse::<u64>().is_ok())
}

/// Walk the contracts directory and produce a `DiscoveryReport`.
///
/// Files are sorted by path for deterministic output (INV-005).
pub fn discover_contracts(
    contracts_dir: &Path,
    manifest: &BTreeMap<String, (String, ContractKind)>,
) -> DiscoveryReport {
    let mut files: Vec<ContractFile> = Vec::new();
    let mut raw_errors: Vec<ValidationError> = Vec::new();
    let mut errors_by_kind: BTreeMap<ContractKind, u32> = BTreeMap::new();
    let mut version_violations: Vec<VersionViolation> = Vec::new();

    // Collect and sort paths for deterministic output
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(contracts_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "cue") {
                paths.push(path);
            }
        }
    }
    paths.sort();

    let mut invalid_count: u32 = 0;

    for path in &paths {
        match validate_contract_file(path, manifest) {
            Ok(contract_file) => {
                *errors_by_kind.entry(contract_file.kind).or_insert(0) += 1;
                files.push(contract_file);
            }
            Err(file_errors) => {
                invalid_count += 1;
                for err in file_errors {
                    raw_errors.push(err.clone());
                }
            }
        }
    }

    // Collect version violations from raw errors
    for err in &raw_errors {
        if let ValidationError::VersionMonotonicityBreach {
            file,
            expected,
            actual,
        } = err
        {
            version_violations.push(VersionViolation {
                file: PathBuf::from(file),
                expected: expected.clone(),
                actual: actual.clone(),
                detail: "monotonicity breach".to_string(),
            });
        }
    }
    version_violations.sort_by(|a, b| a.file.cmp(&b.file));

    let valid = (files.len() - invalid_count as usize).max(0) as u32;

    // Build serializable error messages
    let errors: Vec<String> = raw_errors.iter().map(|e| e.to_string()).collect();

    DiscoveryReport {
        files,
        errors,
        raw_errors,
        summary: ReportSummary {
            total: paths.len() as u32,
            valid,
            invalid: invalid_count,
            errors_by_kind,
            version_violations,
        },
    }
}

/// Convert a `DiscoveryReport` to a `GateEvidence` for the evidence pipeline.
///
/// This is the integration point for OBL-006 (GateEvidence parity).
pub fn gate_evidence_from_report(report: &DiscoveryReport) -> crate::evidence::GateEvidence {
    let exit_code = if report.summary.invalid == 0 { 0 } else { 1 };

    let status = if report.summary.invalid == 0 {
        crate::evidence::GateStatus::Pass
    } else {
        crate::evidence::GateStatus::Fail
    };

    let why_failed = if report.summary.invalid > 0 {
        let error_count = report.raw_errors.len();
        Some(crate::evidence::WhyFailed {
            gate_name: "contracts".to_string(),
            hint: format!(
                "{error_count} contract validation error(s) found. Check contracts/ directory.",
            ),
            repair_command: "cargo xtask contracts --check".to_string(),
            variant: None,
            fixture_id: None,
            expected_gate: None,
        })
    } else {
        None
    };

    crate::evidence::GateEvidence {
        kind: "contract-discovery".to_string(),
        gate_name: "contracts".to_string(),
        command: "cargo xtask contracts --dir contracts".to_string(),
        exit_code,
        log: PathBuf::from(".evidence/contracts.log"),
        status,
        why_failed,
    }
}

/// Load the contract manifest from `.beads/contracts/manifest.json` if it exists.
///
/// Returns a `BTreeMap` of file path -> (version, kind).
pub fn load_manifest(
    workspace_root: &Path,
) -> ValidationResult<BTreeMap<String, (String, ContractKind)>> {
    let manifest_path = workspace_root.join(".beads/contracts/manifest.json");
    if !manifest_path.exists() {
        return Ok(BTreeMap::new());
    }

    let content = std::fs::read_to_string(&manifest_path).map_err(|e| {
        ValidationError::MissingRequiredField {
            field: format!("manifest.json: {e}"),
        }
    })?;

    let parsed: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| ValidationError::MissingRequiredField {
            field: format!("manifest.json parse: {e}"),
        })?;

    let mut map = BTreeMap::new();

    if let Some(registry) = parsed.get("contract_registry") {
        if let Some(obj) = registry.as_object() {
            for (_key, entry) in obj {
                if let (Some(path_val), Some(ver_val), Some(kind_val)) = (
                    entry.get("path"),
                    entry.get("schema_version"),
                    entry.get("kind"),
                ) {
                    if let (Some(path_str), Some(ver_str), Some(kind_str)) =
                        (path_val.as_str(), ver_val.as_str(), kind_val.as_str())
                    {
                        if let Some(kind) = ContractKind::from_str(kind_str) {
                            map.insert(path_str.to_string(), (ver_str.to_string(), kind));
                        }
                    }
                }
            }
        }
    }

    Ok(map)
}

/// Save a contract manifest to `.beads/contracts/manifest.json`.
pub fn save_manifest(workspace_root: &Path, report: &DiscoveryReport) -> ValidationResult<()> {
    let manifest_dir = workspace_root.join(".beads/contracts");
    std::fs::create_dir_all(&manifest_dir).map_err(|e| ValidationError::MissingRequiredField {
        field: format!("create manifest dir: {e}"),
    })?;

    let mut registry = serde_json::Map::new();

    for file in &report.files {
        let key = file.path.to_string_lossy().to_string();
        let mut entry = serde_json::Map::new();
        entry.insert("path".to_string(), serde_json::Value::String(key.clone()));
        entry.insert(
            "schema_version".to_string(),
            serde_json::Value::String(file.schema_version.clone()),
        );
        entry.insert(
            "kind".to_string(),
            serde_json::Value::String(file.kind.cue_str().to_string()),
        );
        entry.insert(
            "last_validated".to_string(),
            serde_json::Value::String(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        );
        registry.insert(key, serde_json::Value::Object(entry));
    }

    let mut output = serde_json::Map::new();
    output.insert(
        "contract_registry".to_string(),
        serde_json::Value::Object(registry),
    );

    let json = serde_json::to_string_pretty(&output).map_err(|e| {
        ValidationError::MissingRequiredField {
            field: format!("manifest serialization: {e}"),
        }
    })?;

    let manifest_path = manifest_dir.join("manifest.json");
    std::fs::write(&manifest_path, json).map_err(|e| ValidationError::MissingRequiredField {
        field: format!("write manifest: {e}"),
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_contract_meta_version() {
        let content = r#"package validation

#ContractMeta: {
	schema_version: string
	kind: "cli_envelope"
}
"#;
        let meta = parse_contract_meta(content);
        assert_eq!(meta.schema_version, None);
        assert_eq!(meta.kind, Some("cli_envelope".to_string()));
    }

    #[test]
    fn test_parse_contract_meta_version_literal() {
        let content = r#"package validation

#ContractMeta: {
	schema_version: "1.0.0"
	kind: "cli_envelope"
}
"#;
        let meta = parse_contract_meta(content);
        assert_eq!(meta.schema_version, Some("1.0.0".to_string()));
        assert_eq!(meta.kind, Some("cli_envelope".to_string()));
    }

    #[test]
    fn test_compare_semver_increasing() {
        assert_eq!(
            compare_semver("1.0.0", "2.0.0").unwrap(),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_semver("1.0.0", "1.1.0").unwrap(),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_semver("1.0.0", "1.0.1").unwrap(),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn test_compare_semver_equal() {
        assert_eq!(
            compare_semver("1.0.0", "1.0.0").unwrap(),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn test_compare_semver_invalid() {
        assert!(compare_semver("not-a-version", "1.0.0").is_err());
        assert!(compare_semver("1.0", "1.0.0").is_err());
    }

    #[test]
    fn test_contract_kind_from_str_all() {
        for kind in ContractKind::all_values() {
            assert_eq!(
                ContractKind::from_str(kind.cue_str()),
                Some(*kind),
                "Failed to parse {}",
                kind.cue_str()
            );
        }
        assert_eq!(ContractKind::from_str("unknown_kind"), None);
    }

    #[test]
    fn test_is_valid_semver() {
        assert!(is_valid_semver("1.0.0"));
        assert!(is_valid_semver("0.1.0"));
        assert!(is_valid_semver("10.20.30"));
        assert!(!is_valid_semver("1.0"));
        assert!(!is_valid_semver("1.0.0.0"));
        assert!(!is_valid_semver("abc"));
    }
}
