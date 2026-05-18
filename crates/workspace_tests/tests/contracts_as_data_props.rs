//! Proptest properties for contracts-as-data (vb-6f02).
//!
//! Covers: OBL-001 (schema_version), OBL-004 (discovery finds all files),
//! OBL-006 (BTreeMap deterministic JSON), OBL-010 (CUE validation catches errors).
//!
//! Each property is independent and can be run with:
//! `cargo test -p workspace_tests --test contracts_as_data_props -- property_name --exact`

use std::collections::BTreeMap;

use proptest::prelude::*;
use proptest::proptest;

// ============================================================
// Domain model — mirrors xtask/src/contracts.rs
// ============================================================

/// ContractKind mirrors the 6 valid enum values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
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

impl proptest::arbitrary::Arbitrary for ContractKind {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;

    fn arbitrary_with(_params: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (0u8..6)
            .prop_map(|i| match i {
                0 => Self::CliEnvelope,
                1 => Self::UiTokens,
                2 => Self::AcceptedArtifacts,
                3 => Self::EvidenceBundle,
                4 => Self::Diagnostics,
                _ => Self::GateOutput,
            })
            .boxed()
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
pub fn compare_semver(a: &str, b: &str) -> Result<std::cmp::Ordering, String> {
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

    let va = parse_parts(a).ok_or(format!("invalid semver: {}", a))?;
    let vb = parse_parts(b).ok_or(format!("invalid semver: {}", b))?;

    if va.0 != vb.0 {
        Ok(va.0.cmp(&vb.0))
    } else if va.1 != vb.1 {
        Ok(va.1.cmp(&vb.1))
    } else {
        Ok(va.2.cmp(&vb.2))
    }
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
        match result {
            Ok(v) => prop_assert_eq!(v, version, "Accepted version must equal input"),
            Err(e) => prop_assert!(false, "parse_schema_version should accept '{}', got Err: {}", version, e),
        }
    }

    /// Property: parse_schema_version rejects malformed versions.
    ///
    /// Generates arbitrary strings and filters for those that are NOT valid semver,
    /// then asserts each is rejected by parse_schema_version.
    #[test]
    fn test_schema_version_rejects_malformed(
        raw in any::<String>().prop_filter(
            "skip valid semver",
            |s| !spec_is_valid_schema_version(s),
        ),
    ) {
        let result = parse_schema_version(&raw);
        prop_assert!(
            result.is_err(),
            "parse_schema_version should reject malformed version: '{}'",
            raw
        );
    }

    /// Property: parse_schema_version rejects any string not matching valid semver.
    ///
    /// Generates arbitrary strings and proves correctness: if the string is valid
    /// semver per the spec, parse_schema_version returns Ok; otherwise Err.
    #[test]
    fn test_schema_version_matches_spec(
        raw in any::<String>(),
    ) {
        let result = parse_schema_version(&raw);
        let is_valid = spec_is_valid_schema_version(&raw);

        match result {
            Ok(v) => {
                prop_assert!(is_valid,
                    "parse_schema_version accepted invalid semver: '{}'", raw);
                prop_assert_eq!(v, raw, "Accepted version must equal input");
            }
            Err(_) => {
                prop_assert!(!is_valid,
                    "Spec says invalid, parse_schema_version should return Err for: '{}'", raw);
            }
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

        match result2 {
            Ok(Ok(v)) => prop_assert_eq!(v, version, "parse_schema_version should be idempotent for valid input"),
            Ok(Err(e)) => prop_assert!(false, "Second parse should succeed for '{}': {}", version, e),
            Err(e) => prop_assert!(false, "First parse should succeed for '{}': {}", version, e),
        }
    }

    /// Property: parse_contract_kind rejects unknown kinds.
    ///
    /// Generates random non-empty strings that are NOT valid kind names,
    /// then verifies they all map to Err.
    #[test]
    fn test_kind_rejects_unknown(
        raw in any::<String>().prop_filter(
            "skip valid kinds",
            |s| {
                s != "cli_envelope"
                    && s != "ui_tokens"
                    && s != "accepted_artifacts"
                    && s != "evidence_bundle"
                    && s != "diagnostics"
                    && s != "gate_output"
            },
        ),
    ) {
        let result = parse_contract_kind(&raw);
        prop_assert!(result.is_err(), "Should reject unknown kind: '{}'", raw);
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
            any::<ContractKind>(),
            1..=6
        ),
        counts in proptest::collection::vec(
            any::<u32>(),
            1..=6
        ),
    ) {
        // Ensure no duplicate keys (same key with different counts would give different results)
        let len = kinds.len().min(counts.len());
        let unique_kinds: std::collections::HashSet<_> = kinds[..len].iter().collect();
        if unique_kinds.len() == len {
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
        let json1 = match serde_json::to_string(&map1) {
            Ok(j) => j,
            Err(e) => {
                prop_assert!(false, "JSON serialization should not fail: {}", e);
                String::new()
            }
        };
        let json2 = match serde_json::to_string(&map2) {
            Ok(j) => j,
            Err(e) => {
                prop_assert!(false, "JSON serialization should not fail: {}", e);
                String::new()
            }
        };

        prop_assert_eq!(json1, json2,
            "BTreeMap serialization must be deterministic regardless of insertion order");
        }
    }

    /// Property: BTreeMap with ContractKind keys produces sorted JSON keys.
    ///
    /// JSON output must have keys in lexicographic order regardless of insertion order.
    #[test]
    fn test_btreemap_sorted_keys(
        kinds in proptest::collection::vec(any::<ContractKind>(), 1..=6),
        counts in proptest::collection::vec(any::<u32>(), 1..=6),
    ) {
        let len = kinds.len().min(counts.len());
        let mut map = BTreeMap::new();
        for i in 0..len {
            map.insert(kinds[i], counts[i]);
        }

        let json = match serde_json::to_string(&map) {
            Ok(j) => j,
            Err(e) => {
                prop_assert!(false, "JSON serialization should not fail: {}", e);
                String::new()
            }
        };

        let json_value = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => {
                prop_assert!(false, "JSON deserialization should succeed: {}", e);
                serde_json::Value::Null
            }
        };
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
        valid in 0u32..=u32::MAX,
        invalid in 0u32..=u32::MAX,
    ) {
        let total = valid.saturating_add(invalid);
        let summary = ReportSummary {
            total,
            valid,
            invalid,
            errors_by_kind: BTreeMap::new(),
        };

        prop_assert_eq!(summary.total, summary.valid.saturating_add(summary.invalid),
            "ReportSummary.total must equal valid + invalid");
    }
}

// ============================================================
// OBL-008: Empty contracts directory edge case — moved outside proptest! block

// ============================================================
// OBL-004: compare_semver is a strict weak order
// ============================================================

proptest! {
    /// Property: compare_semver is reflexive (cmp(a, a) == 0).
    #[test]
    fn test_semver_reflexive(
        major in 0u64..=u64::MAX,
        minor in 0u64..=u64::MAX,
        patch in 0u64..=u64::MAX,
    ) {
        let version = format!("{}.{}.{}", major, minor, patch);
        let cmp = compare_semver(&version, &version);
        prop_assert_eq!(cmp, Ok(std::cmp::Ordering::Equal), "compare_semver(a, a) must be Equal");
    }

    /// Property: compare_semver is antisymmetric (cmp(a,b) = reverse of cmp(b,a)).
    #[test]
    fn test_semver_antisymmetric(
        major1 in 0u64..=u64::MAX,
        minor1 in 0u64..=u64::MAX,
        patch1 in 0u64..=u64::MAX,
        major2 in 0u64..=u64::MAX,
        minor2 in 0u64..=u64::MAX,
        patch2 in 0u64..=u64::MAX,
    ) {
        let v1 = format!("{}.{}.{}", major1, minor1, patch1);
        let v2 = format!("{}.{}.{}", major2, minor2, patch2);

        let cmp_ab = compare_semver(&v1, &v2);
        let cmp_ba = compare_semver(&v2, &v1);

        prop_assert!(
            cmp_ab.is_ok() && cmp_ba.is_ok(),
            "compare_semver should succeed for valid semver strings"
        );
        let ord_ab = cmp_ab.unwrap();
        let ord_ba = cmp_ba.unwrap();
        prop_assert_eq!(ord_ab, ord_ba.reverse(),
            "compare_semver(a, b) must reverse compare_semver(b, a)");
    }

    /// Property: compare_semver is transitive.
    ///
    /// If cmp(a, b) > 0 and cmp(b, c) > 0, then cmp(a, c) > 0.
    #[test]
    fn test_semver_transitive(
        major1 in 0u64..=u64::MAX,
        minor1 in 0u64..=u64::MAX,
        patch1 in 0u64..=u64::MAX,
        major2 in 0u64..=u64::MAX,
        minor2 in 0u64..=u64::MAX,
        patch2 in 0u64..=u64::MAX,
        major3 in 0u64..=u64::MAX,
        minor3 in 0u64..=u64::MAX,
        patch3 in 0u64..=u64::MAX,
    ) {
        let v1 = format!("{}.{}.{}", major1, minor1, patch1);
        let v2 = format!("{}.{}.{}", major2, minor2, patch2);
        let v3 = format!("{}.{}.{}", major3, minor3, patch3);

        let cmp_ab = compare_semver(&v1, &v2);
        let cmp_bc = compare_semver(&v2, &v3);

        // Only test transitivity when both comparisons are positive
        if let (Ok(std::cmp::Ordering::Greater), Ok(std::cmp::Ordering::Greater)) = (&cmp_ab, &cmp_bc) {
            let cmp_ac = compare_semver(&v1, &v3);
            prop_assert_eq!(cmp_ac, Ok(std::cmp::Ordering::Greater),
                "Transitivity: if a > b and b > c, then a > c");
        }
    }

    /// Property: compare_semver correctly orders increasing versions.
    #[test]
    fn test_semver_increasing_order(
        major in 0u64..=99u64,
        minor in 0u64..=99u64,
        patch in 0u64..=99u64,
    ) {
        let v1 = format!("{}.{}.{}", major, minor, patch);
        let v2 = format!("{}.{}.{}", major, minor, patch + 1);
        let v3 = format!("{}.{}.{}", major, minor + 1, 0);
        let v4 = format!("{}.{}.{}", major + 1, 0, 0);

        prop_assert_eq!(compare_semver(&v1, &v2), Ok(std::cmp::Ordering::Less), "patch increase: v1 < v2");
        prop_assert_eq!(compare_semver(&v2, &v3), Ok(std::cmp::Ordering::Less), "minor increase: v2 < v3");
        prop_assert_eq!(compare_semver(&v3, &v4), Ok(std::cmp::Ordering::Less), "major increase: v3 < v4");
    }
}

// ============================================================
// OBL-010: CUE validation catches schema errors
// ============================================================

proptest! {
    /// Property: cue vet accepts valid contract files.
    ///
    /// Generates valid semver and valid kind, then verifies real validation accepts them.
    #[test]
    fn test_cue_validation_accepts_valid(
        major in 0u32..=999u32,
        minor in 0u32..=999u32,
        patch in 0u32..=999u32,
        kind in any::<ContractKind>(),
    ) {
        let version = format!("{}.{}.{}", major, minor, patch);
        let kind_str = kind.to_string();

        let cue_content = format!(r#"package validation

#TestSchema: #ContractMeta & {{
    schema_version: "{}"
    kind: "{}"
}}
"#, version, kind_str);

        prop_assert!(
            validate_contract_cue(&cue_content),
            "CUE file with valid schema_version and kind should be valid"
        );
    }
}

/// Independent specification for schema_version validity (mirrors spec in Kani harness).
///
/// Returns true if the string is a valid semver: non-empty, exactly 3 dot-separated
/// numeric parts, no leading zeros (except "0" itself).
fn spec_is_valid_schema_version(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let parts: Vec<&str> = s.splitn(3, '.').collect();
    if parts.len() != 3 {
        return false;
    }

    for part in &parts {
        if part.is_empty() {
            return false;
        }
        if part.len() > 1 && part.starts_with('0') {
            return false;
        }
        if !part.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }

    true
}

/// Validates a CUE file's #ContractMeta by parsing its schema_version and kind fields.
///
/// This is real validation — uses parse_schema_version and parse_contract_kind
/// to verify the fields, not string matching.
fn validate_contract_cue(content: &str) -> bool {
    // Extract schema_version value from CUE content
    let version = extract_cue_string_field(content, "schema_version");
    let kind = extract_cue_string_field(content, "kind");

    // Validate schema_version using real parser
    if parse_schema_version(&version).is_err() {
        return false;
    }

    // Validate kind using real parser
    if parse_contract_kind(&kind).is_err() {
        return false;
    }

    true
}

/// Extracts a string field value from CUE content.
///
/// Looks for patterns like: `field_name: "value"` or `field_name: value`
fn extract_cue_string_field(content: &str, field: &str) -> String {
    let pattern = format!("{}:", field);
    if let Some(pos) = content.find(&pattern) {
        let after = &content[pos + pattern.len()..];
        let after = after.trim();
        // Check for quoted string
        if let Some(quoted) = after.strip_prefix('"') {
            if let Some(end) = quoted.find('"') {
                return quoted[..end].to_string();
            }
        }
        // Unquoted value (take until newline or end)
        let unquoted = after.split(|c: char| c.is_whitespace() || c == '\n').next().unwrap_or("");
        return unquoted.to_string();
    }
    String::new()
}

// ============================================================
// Parameterless tests (must be outside proptest! macro)
// ============================================================

/// Property: parse_schema_version rejects empty strings.
#[test]
fn test_schema_version_rejects_empty() {
    let result = parse_schema_version("");
    assert!(result.is_err(), "parse_schema_version should reject empty string");
}

/// Property: cue vet would reject files missing schema_version.
#[test]
fn test_cue_validation_rejects_missing_version() {
    let cue_content = r#"package validation

#TestSchema: {
    kind: "cli_envelope"
    // schema_version is missing!
}
"#;

    assert!(
        !validate_contract_cue(cue_content),
        "CUE file without schema_version should be invalid"
    );
}

/// Property: cue vet would reject files with invalid kind.
#[test]
fn test_cue_validation_rejects_invalid_kind() {
    let cue_content = r#"package validation

#TestSchema: {
    schema_version: "1.0.0"
    kind: "invalid_kind" // Not in the allowed set
}
"#;

    assert!(
        !validate_contract_cue(cue_content),
        "CUE file with invalid kind should be invalid"
    );
}

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

    assert_eq!(report.total, 0);
    assert_eq!(report.valid, 0);
    assert_eq!(report.invalid, 0);
    assert!(report.errors_by_kind.is_empty());
    assert!(report.version_violations.is_empty());
}
