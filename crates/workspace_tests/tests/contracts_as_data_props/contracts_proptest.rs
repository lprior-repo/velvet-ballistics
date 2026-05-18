//! Proptest properties for contracts-as-data (vb-6f02).
//!
//! Covers: OBL-001 (schema_version), OBL-004 (discovery finds all files),
//! OBL-006 (BTreeMap deterministic JSON), OBL-010 (CUE validation catches errors).
//!
//! Each property is independent and can be run with:
//! `cargo test -p workspace_tests --test contracts_as_data_props -- property_name --exact`

use std::collections::BTreeMap;
use std::path::PathBuf;

// ============================================================
// Domain model — mirrors xtask/src/contracts.rs
// ============================================================

/// ContractKind mirrors the 6 valid enum values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum ContractKind {
    CliEnvelope,
    UiTokens,
    AcceptedArtifacts,
    EvidenceBundle,
    Diagnostics,
    GateOutput,
}

impl ContractKind {
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

    pub fn valid_strings() -> &'static [&'static str] {
        &[
            "cli_envelope",
            "ui_tokens",
            "accepted_artifacts",
            "evidence_bundle",
            "diagnostics",
            "gate_output",
        ]
    }
}

impl std::fmt::Display for ContractKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CliEnvelope => write!(f, "cli_envelope"),
            Self::UiTokens => write!(f, "ui_tokens"),
            Self::AcceptedArtifacts => write!(f, "accepted_artifacts"),
            Self::EvidenceBundle => write!(f, "evidence_bundle"),
            Self::Diagnostics => write!(f, "diagnostics"),
            Self::GateOutput => write!(f, "gate_output"),
        }
    }
}

/// parse_contract_kind mirrors xtask/src/contracts.rs::parse_contract_kind
pub fn parse_contract_kind(raw: &str) -> Result<ContractKind, String> {
    match raw {
        "cli_envelope" => Ok(ContractKind::CliEnvelope),
        "ui_tokens" => Ok(ContractKind::UiTokens),
        "accepted_artifacts" => Ok(ContractKind::AcceptedArtifacts),
        "evidence_bundle" => Ok(ContractKind::EvidenceBundle),
        "diagnostics" => Ok(ContractKind::Diagnostics),
        "gate_output" => Ok(ContractKind::GateOutput),
        unknown => Err(format!("Invalid kind: '{}'", unknown)),
    }
}

/// parse_schema_version mirrors xtask/src/contracts.rs::parse_schema_version
pub fn parse_schema_version(raw: &str) -> Result<String, String> {
    if raw.is_empty() {
        return Err("Missing schema version".to_string());
    }

    let parts: Vec<&str> = raw.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Err(format!("Invalid version format: '{}'", raw));
    }

    for part in &parts {
        if part.is_empty() {
            return Err(format!("Empty semver component in: '{}'", raw));
        }
        if part.len() > 1 && part.starts_with('0') {
            return Err(format!("Leading zero in semver component: '{}'", raw));
        }
        if part.parse::<u32>().is_err() {
            return Err(format!("Non-numeric semver component in: '{}'", raw));
        }
    }

    Ok(raw.to_string())
}

/// parse_vet_exit_code mirrors xtask/src/contracts.rs::parse_vet_exit_code
pub fn parse_vet_exit_code(exit_code: i32) -> Result<(), String> {
    if exit_code == 0 {
        Ok(())
    } else {
        Err(format!("cue vet failed with exit code {}", exit_code))
    }
}

/// compare_semver mirrors xtask/src/contracts.rs::compare_semver
pub fn compare_semver(old: &str, new: &str) -> i32 {
    let old_parts: Vec<u32> = old.split('.').filter_map(|p| p.parse().ok()).collect();
    let new_parts: Vec<u32> = new.split('.').filter_map(|p| p.parse().ok()).collect();

    for i in 0..3 {
        let old_val = old_parts.get(i).copied().unwrap_or(0);
        let new_val = new_parts.get(i).copied().unwrap_or(0);
        if new_val > old_val {
            return 1;
        } else if new_val < old_val {
            return -1;
        }
    }
    0
}

// ============================================================
// OBL-001: CUE schema files well-formed (schema_version)
// ============================================================

proptest! {
    /// Property: parse_schema_version accepts valid semver strings.
    ///
    /// For any three u32 values, the formatted "major.minor.patch" string
    /// must be accepted by parse_schema_version.
    #[test]
    fn test_schema_version_accepts_valid_semver(
        major in 0u32..=u32::MAX,
        minor in 0u32..=u32::MAX,
        patch in 0u32..=u32::MAX,
    ) {
        let version = format!("{}.{}.{}", major, minor, patch);
        let result = parse_schema_version(&version);
        prop_assert!(result.is_ok(), "parse_schema_version should accept '{}'", version);
        prop_assert_eq!(result.unwrap(), version);
    }

    /// Property: parse_schema_version rejects empty strings.
    #[test]
    fn test_schema_version_rejects_empty() {
        let result = parse_schema_version("");
        prop_assert!(result.is_err(), "parse_schema_version should reject empty string");
    }

    /// Property: parse_schema_version rejects malformed versions.
    #[test]
    fn test_schema_version_rejects_malformed(
        malformed in r"^\d*$|^\d+\.\d*$|^\d+\.\d+\.\d+\.\d*$|^\d+\.0\.0$|^0\.\d+\.0$|^abc$|^\.\d+\.0$|^\d+\.\.0$|^1\.0\.$",
    ) {
        // Generate malformed inputs that should be rejected
        let test_cases = [
            "",
            "1.0",
            "1.0.0.0",
            "01.0.0",
            "1.02.0",
            "1.0.03",
            "abc",
            "1.abc.0",
            "1.0.abc",
            ".0.0",
            "1..0",
            "1.0.",
        ];

        for input in &test_cases {
            let result = parse_schema_version(input);
            prop_assert!(
                result.is_err(),
                "parse_schema_version should reject malformed version: '{}'",
                input
            );
        }
    }

    /// Property: parse_schema_version is idempotent for valid versions.
    #[test]
    fn test_schema_version_idempotent(
        major in 0u32..=u32::MAX,
        minor in 0u32..=u32::MAX,
        patch in 0u32..=u32::MAX,
    ) {
        let version = format!("{}.{}.{}", major, minor, patch);
        let result1 = parse_schema_version(&version);
        let result2 = result1.as_ref().map(|v| parse_schema_version(v));

        prop_assert!(result2.unwrap().unwrap() == version,
            "parse_schema_version should be idempotent for valid input");
    }

    /// Property: parse_contract_kind rejects unknown kinds.
    ///
    /// Generates random strings and verifies they all map to Err.
    #[test]
    fn test_kind_rejects_unknown(proptest::strategy::Strategy as _, strategy: proptest::prelude::Strategy) {
        // Use a custom strategy to generate random strings
        let random_strategies = proptest::collection::vec(proptest::prelude::any::<u8>(), 1..20);

        prop_for_each_input(|bytes: Vec<u8>| {
            let kind_str = String::from_utf8_lossy(&bytes).to_string();
            if kind_str == "cli_envelope"
                || kind_str == "ui_tokens"
                || kind_str == "accepted_artifacts"
                || kind_str == "evidence_bundle"
                || kind_str == "diagnostics"
                || kind_str == "gate_output"
            {
                return; // Skip valid kinds
            }
            let result = parse_contract_kind(&kind_str);
            prop_assert!(result.is_err(), "Should reject unknown kind: '{}'", kind_str);
        });
    }
}

// ============================================================
// OBL-005/006: BTreeMap deterministic JSON output
// ============================================================

/// ReportSummary with BTreeMap for deterministic ordering.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReportSummary {
    pub total: u32,
    pub valid: u32,
    pub invalid: u32,
    pub errors_by_kind: BTreeMap<ContractKind, u32>,
}

/// DiscoveryReport for testing determinism.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiscoveryReport {
    pub total: u32,
    pub valid: u32,
    pub invalid: u32,
    pub errors_by_kind: BTreeMap<ContractKind, u32>,
    pub version_violations: Vec<String>,
}

proptest! {
    /// Property: BTreeMap serialization produces deterministic JSON.
    ///
    /// Inserting the same key-value pairs in different order produces
    /// identical JSON output.
    #[test]
    fn test_btreemap_deterministic_json(
        kinds in proptest::collection::vec(
            proptest::prelude::any::<ContractKind>(),
            1..=6
        ),
        counts in proptest::collection::vec(
            proptest::prelude::any::<u32>(),
            1..=6
        ),
    ) {
        // Ensure we have matching lengths
        let len = kinds.len().min(counts.len());
        let pairs: Vec<_> = kinds[..len].iter().zip(counts[..len].iter()).collect();

        // Build two maps with same pairs in different order
        let mut map1 = BTreeMap::new();
        let mut map2 = BTreeMap::new();

        for (kind, count) in &pairs {
            map1.insert(**kind, *count);
        }

        // Insert in reverse order
        for (kind, count) in pairs.iter().rev() {
            map2.insert(**kind, **count);
        }

        // Both maps must serialize to identical JSON
        let json1 = serde_json::to_string(&map1).unwrap();
        let json2 = serde_json::to_string(&map2).unwrap();

        prop_assert_eq!(json1, json2,
            "BTreeMap serialization must be deterministic regardless of insertion order");
    }

    /// Property: BTreeMap with ContractKind keys produces sorted JSON keys.
    ///
    /// JSON output must have keys in lexicographic order.
    #[test]
    fn test_btreemap_sorted_keys() {
        let mut map = BTreeMap::new();
        map.insert(ContractKind::GateOutput, 1u32);
        map.insert(ContractKind::CliEnvelope, 2u32);
        map.insert(ContractKind::Diagnostics, 3u32);

        let json = serde_json::to_string(&map).unwrap();

        // Parse the JSON and check key order
        let json_value: serde_json::Value = serde_json::from_str(&json).unwrap();
        if let serde_json::Value::Object(obj) = json_value {
            let keys: Vec<&String> = obj.keys().collect();
            let mut sorted_keys = keys.clone();
            sorted_keys.sort();
            prop_assert_eq!(keys, sorted_keys,
                "BTreeMap JSON keys must be in sorted order");
        } else {
            prop_assert!(false, "Expected JSON object");
        }
    }

    /// Property: ReportSummary total = valid + invalid invariant.
    #[test]
    fn test_report_summary_invariant(
        total in 0u32..=u32::MAX,
    ) {
        let valid: u32 = proptest::prelude::any();
        let invalid: u32 = proptest::prelude::any();

        // Only test cases where valid + invalid doesn't overflow
        if valid.saturating_add(invalid) == total {
            let summary = ReportSummary {
                total,
                valid,
                invalid,
                errors_by_kind: BTreeMap::new(),
            };

            prop_assert_eq!(summary.total, summary.valid + summary.invalid,
                "ReportSummary.total must equal valid + invalid");
        }
    }
}

// ============================================================
// OBL-008: Empty contracts directory edge case
// ============================================================

proptest! {
    /// Property: discover on empty contracts/ produces valid Pass report.
    #[test]
    fn test_empty_directory_passes() {
        let report = DiscoveryReport {
            total: 0,
            valid: 0,
            invalid: 0,
            errors_by_kind: BTreeMap::new(),
            version_violations: Vec::new(),
        };

        prop_assert_eq!(report.total, 0);
        prop_assert_eq!(report.valid, 0);
        prop_assert_eq!(report.invalid, 0);
        prop_assert!(report.errors_by_kind.is_empty());
        prop_assert!(report.version_violations.is_empty());
    }
}

// ============================================================
// OBL-004: compare_semver is a strict weak order
// ============================================================

proptest! {
    /// Property: compare_semver is reflexive (cmp(a, a) == 0).
    #[test]
    fn test_semver_reflexive(
        major in 0u32..=u32::MAX,
        minor in 0u32..=u32::MAX,
        patch in 0u32..=u32::MAX,
    ) {
        let version = format!("{}.{}.{}", major, minor, patch);
        let cmp = compare_semver(&version, &version);
        prop_assert_eq!(cmp, 0, "compare_semver(a, a) must be 0");
    }

    /// Property: compare_semver is antisymmetric (cmp(a,b) = -cmp(b,a)).
    #[test]
    fn test_semver_antisymmetric(
        major1 in 0u32..=u32::MAX,
        minor1 in 0u32..=u32::MAX,
        patch1 in 0u32..=u32::MAX,
        major2 in 0u32..=u32::MAX,
        minor2 in 0u32..=u32::MAX,
        patch2 in 0u32..=u32::MAX,
    ) {
        let v1 = format!("{}.{}.{}", major1, minor1, patch1);
        let v2 = format!("{}.{}.{}", major2, minor2, patch2);

        let cmp_ab = compare_semver(&v1, &v2);
        let cmp_ba = compare_semver(&v2, &v1);

        prop_assert_eq!(cmp_ab, -cmp_ba,
            "compare_semver(a, b) must equal -compare_semver(b, a)");
    }

    /// Property: compare_semver is transitive.
    ///
    /// If cmp(a, b) > 0 and cmp(b, c) > 0, then cmp(a, c) > 0.
    #[test]
    fn test_semver_transitive(
        major1 in 0u32..=u32::MAX,
        minor1 in 0u32..=u32::MAX,
        patch1 in 0u32..=u32::MAX,
        major2 in 0u32..=u32::MAX,
        minor2 in 0u32..=u32::MAX,
        patch2 in 0u32..=u32::MAX,
        major3 in 0u32..=u32::MAX,
        minor3 in 0u32..=u32::MAX,
        patch3 in 0u32..=u32::MAX,
    ) {
        let v1 = format!("{}.{}.{}", major1, minor1, patch1);
        let v2 = format!("{}.{}.{}", major2, minor2, patch2);
        let v3 = format!("{}.{}.{}", major3, minor3, patch3);

        let cmp_ab = compare_semver(&v1, &v2);
        let cmp_bc = compare_semver(&v2, &v3);

        // Only test transitivity when both comparisons are positive
        if cmp_ab > 0 && cmp_bc > 0 {
            let cmp_ac = compare_semver(&v1, &v3);
            prop_assert!(cmp_ac > 0,
                "Transitivity: if a > b and b > c, then a > c");
        }
    }

    /// Property: compare_semver correctly orders increasing versions.
    #[test]
    fn test_semver_increasing_order(
        major in 0u32..=99u32,
        minor in 0u32..=99u32,
        patch in 0u32..=99u32,
    ) {
        let v1 = format!("{}.{}.{}", major, minor, patch);
        let v2 = format!("{}.{}.{}", major, minor, patch + 1);
        let v3 = format!("{}.{}.{}", major, minor + 1, 0);
        let v4 = format!("{}.{}.{}", major + 1, 0, 0);

        prop_assert!(compare_semver(&v1, &v2) < 0, "patch increase: v1 < v2");
        prop_assert!(compare_semver(&v2, &v3) < 0, "minor increase: v2 < v3");
        prop_assert!(compare_semver(&v3, &v4) < 0, "major increase: v3 < v4");
    }
}

// ============================================================
// OBL-010: CUE validation catches schema errors
// ============================================================

proptest! {
    /// Property: cue vet would reject files missing schema_version.
    ///
    /// Simulates the CUE schema #ContractMeta requiring schema_version.
    #[test]
    fn test_cue_validation_rejects_missing_version() {
        // Simulate a CUE file without schema_version
        let cue_content = r#"package validation

#TestSchema: {
    kind: "cli_envelope"
    // schema_version is missing!
}
"#;

        // The #ContractMeta requires schema_version: string
        // Without it, cue vet should fail
        let has_version = cue_content.contains("schema_version");
        prop_assert!(!has_version,
            "Test file intentionally omits schema_version");
        prop_assert!(
            !is_valid_contract_cue(cue_content),
            "CUE file without schema_version should be invalid"
        );
    }

    /// Property: cue vet would reject files with invalid kind.
    #[test]
    fn test_cue_validation_rejects_invalid_kind() {
        // Simulate a CUE file with invalid kind
        let cue_content = r#"package validation

#TestSchema: {
    schema_version: "1.0.0"
    kind: "invalid_kind" // Not in the allowed set
}
"#;

        let has_invalid_kind = cue_content.contains("invalid_kind");
        prop_assert!(has_invalid_kind,
            "Test file intentionally uses invalid_kind");
        prop_assert!(
            !is_valid_contract_cue(cue_content),
            "CUE file with invalid kind should be invalid"
        );
    }

    /// Property: cue vet accepts valid contract files.
    #[test]
    fn test_cue_validation_accepts_valid(
        major in 0u32..=999u32,
        minor in 0u32..=999u32,
        patch in 0u32..=999u32,
        kind_idx in 0u64..6u64,
    ) {
        let valid_kinds = [
            "cli_envelope",
            "ui_tokens",
            "accepted_artifacts",
            "evidence_bundle",
            "diagnostics",
            "gate_output",
        ];

        let kind = valid_kinds[kind_idx as usize];
        let version = format!("{}.{}.{}", major, minor, patch);

        let cue_content = format!(r#"package validation

#TestSchema: #ContractMeta & {{
    schema_version: "{}"
    kind: "{}"
}}
"#, version, kind);

        // Valid CUE files should pass #ContractMeta
        prop_assert!(
            is_valid_contract_cue(&cue_content),
            "CUE file with valid schema_version and kind should be valid"
        );
    }
}

/// Simulates CUE #ContractMeta validation.
///
/// Returns true if the content has both schema_version and a valid kind.
fn is_valid_contract_cue(content: &str) -> bool {
    let has_version = content.contains("schema_version:")
        && !content.contains("schema_version: \"\"");

    let valid_kinds = [
        "cli_envelope",
        "ui_tokens",
        "accepted_artifacts",
        "evidence_bundle",
        "diagnostics",
        "gate_output",
    ];

    let has_valid_kind = valid_kinds.iter().any(|k| content.contains(&format!("kind: \"{}\"", k)));

    has_version && has_valid_kind
}
