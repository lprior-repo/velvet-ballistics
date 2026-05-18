#![forbid(unsafe_code)]
//! Verification certificate panel -- UI view over IPC verification results.
//!
//! Transforms `vb_ipc::VerificationResult` into structured certificate entries
//! with severity classification for display in the verification view.

/// Severity category derived from gate identifiers.
///
/// Maps gate kinds to user-facing severity buckets:
/// - **Structural**: gate_07 through gate_11 and gate_13 (IR structure, stack, accessor, slot, node, loop, cycle)
/// - **Taint**: gate_14 (slot type consistency / taint flow)
/// - **Durability**: gate_15 (determinism proof)
/// - **Resource**: any gate not in the known mapping (future resource-bound gates)
/// - **Policy**: any gate whose name contains "action" or "policy"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[non_exhaustive]
pub enum CertificateSeverity {
    /// IR structure, stack, accessor, slot, node, loop, or cycle checks.
    Structural,
    /// Resource bound checks.
    Resource,
    /// Taint / slot type consistency checks.
    Taint,
    /// Determinism and durability proof checks.
    Durability,
    /// Action policy checks.
    Policy,
}

impl std::fmt::Display for CertificateSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Structural => write!(f, "Structural"),
            Self::Resource => write!(f, "Resource"),
            Self::Taint => write!(f, "Taint"),
            Self::Durability => write!(f, "Durability"),
            Self::Policy => write!(f, "Policy"),
        }
    }
}

/// One gate-check certificate entry for UI display.
#[derive(Debug, Clone)]
pub struct CertificateEntry {
    /// Gate identifier string (e.g., `"gate_07_expression_stack_depth"`).
    pub kind: String,
    /// Whether this gate check passed.
    pub passed: bool,
    /// Human-readable details (may be empty on pass).
    pub details: String,
    /// Severity category derived from the gate kind.
    pub severity: CertificateSeverity,
}

/// Aggregated pass/fail counts for the certificate panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateSummary {
    /// Total number of gate checks.
    pub total: u32,
    /// Number of gate checks that passed.
    pub passed: u32,
    /// Number of gate checks that failed.
    pub failed: u32,
    /// Per-severity breakdown: `(severity, passed_count, failed_count)`.
    pub by_severity: Vec<(CertificateSeverity, u32, u32)>,
}

/// Panel of verification certificates built from an IPC verification result.
#[derive(Debug, Clone)]
pub struct CertificatePanel {
    entries: Vec<CertificateEntry>,
}

impl CertificatePanel {
    /// Construct a certificate panel from an IPC verification result.
    ///
    /// Maps each `CertificateWire` entry to a `CertificateEntry` with
    /// severity derived from the gate kind string.
    #[must_use]
    pub fn from_verification_result(result: &vb_ipc::VerificationResult) -> Self {
        let entries: Vec<CertificateEntry> = result
            .certificates
            .iter()
            .map(|wire| {
                let passed = wire.status == "Pass";
                CertificateEntry {
                    kind: wire.kind.clone(),
                    passed,
                    details: wire.details.clone(),
                    severity: classify_gate_kind(&wire.kind),
                }
            })
            .collect();

        Self { entries }
    }

    /// Returns all certificate entries in order.
    #[must_use]
    pub fn certificates(&self) -> &[CertificateEntry] {
        &self.entries
    }

    /// Returns `true` when every gate check passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.entries.iter().all(|e| e.passed)
    }

    /// Returns the number of gate checks that passed.
    #[must_use]
    pub fn pass_count(&self) -> u32 {
        self.entries
            .iter()
            .filter(|e| e.passed)
            .count()
            .try_into()
            .unwrap_or(u32::MAX)
    }

    /// Returns the number of gate checks that failed.
    #[must_use]
    pub fn fail_count(&self) -> u32 {
        self.entries
            .iter()
            .filter(|e| !e.passed)
            .count()
            .try_into()
            .unwrap_or(u32::MAX)
    }

    /// Returns only the certificate entries that failed.
    #[must_use]
    pub fn failed_certs(&self) -> Vec<&CertificateEntry> {
        self.entries.iter().filter(|e| !e.passed).collect()
    }

    /// Returns the aggregated gate summary with per-severity breakdown.
    #[must_use]
    pub fn gate_summary(&self) -> GateSummary {
        let total = u32::try_from(self.entries.len()).unwrap_or(u32::MAX);
        let passed = self.pass_count();
        let failed = self.fail_count();

        // Collect unique severities in a deterministic order.
        let mut severity_order: Vec<CertificateSeverity> = Vec::new();
        for sev in &[
            CertificateSeverity::Structural,
            CertificateSeverity::Resource,
            CertificateSeverity::Taint,
            CertificateSeverity::Durability,
            CertificateSeverity::Policy,
        ] {
            let has_entries = self.entries.iter().any(|e| e.severity == *sev);
            if has_entries {
                severity_order.push(*sev);
            }
        }

        let by_severity: Vec<(CertificateSeverity, u32, u32)> = severity_order
            .into_iter()
            .map(|sev| {
                let matching: Vec<&CertificateEntry> =
                    self.entries.iter().filter(|e| e.severity == sev).collect();
                let p = matching.iter().filter(|e| e.passed).count();
                let f = matching.iter().filter(|e| !e.passed).count();
                (
                    sev,
                    u32::try_from(p).unwrap_or(u32::MAX),
                    u32::try_from(f).unwrap_or(u32::MAX),
                )
            })
            .collect();

        GateSummary {
            total,
            passed,
            failed,
            by_severity,
        }
    }
}

/// Classify a gate kind string into a severity category.
///
/// Mapping:
/// - `gate_07`, `gate_08`, `gate_09`, `gate_10`, `gate_11`, `gate_13` -> Structural
/// - `gate_14` -> Taint
/// - `gate_15` -> Durability
/// - Names containing `"action"` or `"policy"` -> Policy
/// - Everything else -> Resource
fn classify_gate_kind(kind: &str) -> CertificateSeverity {
    // Extract the gate number prefix if present.
    let gate_num = extract_gate_number(kind);

    match gate_num {
        Some(7..=11) | Some(13) => CertificateSeverity::Structural,
        Some(14) => CertificateSeverity::Taint,
        Some(15) => CertificateSeverity::Durability,
        _ => {
            // Fallback: keyword-based classification.
            let lower = kind.to_ascii_lowercase();
            if lower.contains("action") || lower.contains("policy") {
                CertificateSeverity::Policy
            } else {
                CertificateSeverity::Resource
            }
        }
    }
}

/// Extract the gate number from a kind string like `"gate_07_expression_stack_depth"`.
/// Returns `None` if the string does not start with `gate_` followed by digits.
fn extract_gate_number(kind: &str) -> Option<u32> {
    let rest = kind.strip_prefix("gate_")?;
    let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(certs: Vec<(&str, &str, &str)>) -> vb_ipc::VerificationResult {
        let certificates: Vec<vb_ipc::CertificateWire> = certs
            .into_iter()
            .map(|(kind, status, details)| vb_ipc::CertificateWire {
                kind: String::from(kind),
                status: String::from(status),
                details: String::from(details),
            })
            .collect();
        let total = u32::try_from(certificates.len()).unwrap_or(u32::MAX);
        let pass_count = certificates
            .iter()
            .filter(|c| c.status == "Pass")
            .count()
            .try_into()
            .unwrap_or(u32::MAX);
        let fail_count = certificates
            .iter()
            .filter(|c| c.status == "Fail")
            .count()
            .try_into()
            .unwrap_or(u32::MAX);
        vb_ipc::VerificationResult {
            certificates,
            total_checks: total,
            pass_count,
            fail_count,
        }
    }

    // -------------------------------------------------------------------------
    // classify_gate_kind / extract_gate_number tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_extract_gate_number_valid() {
        assert_eq!(extract_gate_number("gate_07_expression_stack_depth"), Some(7));
        assert_eq!(extract_gate_number("gate_08_accessor_path_segments"), Some(8));
        assert_eq!(extract_gate_number("gate_09_slot_references"), Some(9));
        assert_eq!(extract_gate_number("gate_10_node_kind_specific"), Some(10));
        assert_eq!(extract_gate_number("gate_11_loop_body_graph"), Some(11));
        assert_eq!(extract_gate_number("gate_13_no_slot_cycles"), Some(13));
        assert_eq!(extract_gate_number("gate_14_slot_type_consistency"), Some(14));
        assert_eq!(extract_gate_number("gate_15_determinism_proof"), Some(15));
    }

    #[test]
    fn test_extract_gate_number_invalid() {
        assert_eq!(extract_gate_number("unknown_gate"), None);
        assert_eq!(extract_gate_number("gate_"), None);
        assert_eq!(extract_gate_number(""), None);
        assert_eq!(extract_gate_number("Gate_07_uppercase"), None);
    }

    #[test]
    fn test_severity_mapping_structural_gates() {
        assert_eq!(
            classify_gate_kind("gate_07_expression_stack_depth"),
            CertificateSeverity::Structural
        );
        assert_eq!(
            classify_gate_kind("gate_08_accessor_path_segments"),
            CertificateSeverity::Structural
        );
        assert_eq!(
            classify_gate_kind("gate_09_slot_references"),
            CertificateSeverity::Structural
        );
        assert_eq!(
            classify_gate_kind("gate_10_node_kind_specific"),
            CertificateSeverity::Structural
        );
        assert_eq!(
            classify_gate_kind("gate_11_loop_body_graph"),
            CertificateSeverity::Structural
        );
        assert_eq!(
            classify_gate_kind("gate_13_no_slot_cycles"),
            CertificateSeverity::Structural
        );
    }

    #[test]
    fn test_severity_mapping_taint() {
        assert_eq!(
            classify_gate_kind("gate_14_slot_type_consistency"),
            CertificateSeverity::Taint
        );
    }

    #[test]
    fn test_severity_mapping_durability() {
        assert_eq!(
            classify_gate_kind("gate_15_determinism_proof"),
            CertificateSeverity::Durability
        );
    }

    #[test]
    fn test_severity_mapping_policy_fallback() {
        assert_eq!(
            classify_gate_kind("action_policy_check"),
            CertificateSeverity::Policy
        );
        assert_eq!(
            classify_gate_kind("some_policy_gate"),
            CertificateSeverity::Policy
        );
    }

    #[test]
    fn test_severity_mapping_resource_fallback() {
        assert_eq!(
            classify_gate_kind("resource_bound_check"),
            CertificateSeverity::Resource
        );
        assert_eq!(
            classify_gate_kind("gate_99_unknown"),
            CertificateSeverity::Resource
        );
    }

    // -------------------------------------------------------------------------
    // CertificatePanel construction tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_from_verification_result_all_pass() {
        let result = make_result(vec![
            ("gate_07_expression_stack_depth", "Pass", ""),
            ("gate_08_accessor_path_segments", "Pass", ""),
            ("gate_09_slot_references", "Pass", ""),
            ("gate_10_node_kind_specific", "Pass", ""),
            ("gate_11_loop_body_graph", "Pass", ""),
            ("gate_13_no_slot_cycles", "Pass", ""),
            ("gate_14_slot_type_consistency", "Pass", ""),
            ("gate_15_determinism_proof", "Pass", ""),
        ]);
        let panel = CertificatePanel::from_verification_result(&result);
        assert!(panel.passed());
        assert_eq!(panel.pass_count(), 8);
        assert_eq!(panel.fail_count(), 0);
        assert!(panel.failed_certs().is_empty());
        assert_eq!(panel.certificates().len(), 8);
    }

    #[test]
    fn test_from_verification_result_all_fail() {
        let result = make_result(vec![
            ("gate_07_expression_stack_depth", "Fail", "stack mismatch"),
            ("gate_14_slot_type_consistency", "Fail", "type error"),
        ]);
        let panel = CertificatePanel::from_verification_result(&result);
        assert!(!panel.passed());
        assert_eq!(panel.pass_count(), 0);
        assert_eq!(panel.fail_count(), 2);
        assert_eq!(panel.failed_certs().len(), 2);
    }

    #[test]
    fn test_from_verification_result_mixed() {
        let result = make_result(vec![
            ("gate_07_expression_stack_depth", "Pass", ""),
            ("gate_08_accessor_path_segments", "Pass", ""),
            ("gate_09_slot_references", "Fail", "slot out of range"),
            ("gate_14_slot_type_consistency", "Pass", ""),
            ("gate_15_determinism_proof", "Fail", "non-deterministic"),
        ]);
        let panel = CertificatePanel::from_verification_result(&result);
        assert!(!panel.passed());
        assert_eq!(panel.pass_count(), 3);
        assert_eq!(panel.fail_count(), 2);
        assert_eq!(panel.certificates().len(), 5);

        let failed = panel.failed_certs();
        assert_eq!(failed.len(), 2);
        assert_eq!(failed[0].kind, "gate_09_slot_references");
        assert_eq!(failed[1].kind, "gate_15_determinism_proof");
        assert!(!failed[0].passed);
        assert_eq!(failed[0].details, "slot out of range");
    }

    #[test]
    fn test_from_verification_result_empty() {
        let result = make_result(vec![]);
        let panel = CertificatePanel::from_verification_result(&result);
        assert!(panel.passed());
        assert_eq!(panel.pass_count(), 0);
        assert_eq!(panel.fail_count(), 0);
        assert_eq!(panel.certificates().len(), 0);
        assert!(panel.failed_certs().is_empty());
    }

    // -------------------------------------------------------------------------
    // GateSummary tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_gate_summary_all_pass() {
        let result = make_result(vec![
            ("gate_07_expression_stack_depth", "Pass", ""),
            ("gate_08_accessor_path_segments", "Pass", ""),
            ("gate_14_slot_type_consistency", "Pass", ""),
            ("gate_15_determinism_proof", "Pass", ""),
        ]);
        let panel = CertificatePanel::from_verification_result(&result);
        let summary = panel.gate_summary();

        assert_eq!(summary.total, 4);
        assert_eq!(summary.passed, 4);
        assert_eq!(summary.failed, 0);

        // Should have 3 severity groups: Structural(2), Taint(1), Durability(1)
        assert_eq!(summary.by_severity.len(), 3);
        assert_eq!(summary.by_severity[0], (CertificateSeverity::Structural, 2, 0));
        assert_eq!(summary.by_severity[1], (CertificateSeverity::Taint, 1, 0));
        assert_eq!(summary.by_severity[2], (CertificateSeverity::Durability, 1, 0));
    }

    #[test]
    fn test_gate_summary_mixed_failures() {
        let result = make_result(vec![
            ("gate_07_expression_stack_depth", "Pass", ""),
            ("gate_10_node_kind_specific", "Fail", "bad node"),
            ("gate_14_slot_type_consistency", "Fail", "taint issue"),
            ("gate_15_determinism_proof", "Pass", ""),
        ]);
        let panel = CertificatePanel::from_verification_result(&result);
        let summary = panel.gate_summary();

        assert_eq!(summary.total, 4);
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.failed, 2);

        // Structural: gate_07 pass, gate_10 fail -> (1, 1)
        // Taint: gate_14 fail -> (0, 1)
        // Durability: gate_15 pass -> (1, 0)
        assert_eq!(summary.by_severity.len(), 3);
        assert_eq!(summary.by_severity[0], (CertificateSeverity::Structural, 1, 1));
        assert_eq!(summary.by_severity[1], (CertificateSeverity::Taint, 0, 1));
        assert_eq!(summary.by_severity[2], (CertificateSeverity::Durability, 1, 0));
    }

    #[test]
    fn test_gate_summary_empty() {
        let result = make_result(vec![]);
        let panel = CertificatePanel::from_verification_result(&result);
        let summary = panel.gate_summary();

        assert_eq!(summary.total, 0);
        assert_eq!(summary.passed, 0);
        assert_eq!(summary.failed, 0);
        assert!(summary.by_severity.is_empty());
    }

    #[test]
    fn test_gate_summary_all_structural() {
        let result = make_result(vec![
            ("gate_07_expression_stack_depth", "Pass", ""),
            ("gate_08_accessor_path_segments", "Fail", "bad accessor"),
            ("gate_09_slot_references", "Pass", ""),
            ("gate_10_node_kind_specific", "Pass", ""),
            ("gate_11_loop_body_graph", "Pass", ""),
            ("gate_13_no_slot_cycles", "Fail", "cycle detected"),
        ]);
        let panel = CertificatePanel::from_verification_result(&result);
        let summary = panel.gate_summary();

        assert_eq!(summary.total, 6);
        assert_eq!(summary.passed, 4);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.by_severity.len(), 1);
        assert_eq!(summary.by_severity[0], (CertificateSeverity::Structural, 4, 2));
    }

    // -------------------------------------------------------------------------
    // CertificateEntry detail preservation tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_entry_details_preserved_on_pass() {
        let result = make_result(vec![
            ("gate_07_expression_stack_depth", "Pass", "all expressions valid"),
        ]);
        let panel = CertificatePanel::from_verification_result(&result);
        assert_eq!(panel.certificates().len(), 1);
        assert_eq!(panel.certificates()[0].details, "all expressions valid");
        assert!(panel.certificates()[0].passed);
    }

    #[test]
    fn test_entry_details_preserved_on_fail() {
        let result = make_result(vec![
            ("gate_14_slot_type_consistency", "Fail", "slot 3 has inconsistent type"),
        ]);
        let panel = CertificatePanel::from_verification_result(&result);
        assert_eq!(panel.certificates()[0].details, "slot 3 has inconsistent type");
        assert!(!panel.certificates()[0].passed);
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", CertificateSeverity::Structural), "Structural");
        assert_eq!(format!("{}", CertificateSeverity::Resource), "Resource");
        assert_eq!(format!("{}", CertificateSeverity::Taint), "Taint");
        assert_eq!(format!("{}", CertificateSeverity::Durability), "Durability");
        assert_eq!(format!("{}", CertificateSeverity::Policy), "Policy");
    }

    #[test]
    fn test_full_ipc_verification_result_mapping() {
        // Simulate a realistic IPC verification result with all 8 gates.
        let result = make_result(vec![
            ("gate_07_expression_stack_depth", "Pass", ""),
            ("gate_08_accessor_path_segments", "Pass", ""),
            ("gate_09_slot_references", "Pass", ""),
            ("gate_10_node_kind_specific", "Pass", ""),
            ("gate_11_loop_body_graph", "Pass", ""),
            ("gate_13_no_slot_cycles", "Fail", "cycle between slots 2 and 5"),
            ("gate_14_slot_type_consistency", "Pass", ""),
            ("gate_15_determinism_proof", "Fail", "non-deterministic action detected"),
        ]);
        let panel = CertificatePanel::from_verification_result(&result);

        assert!(!panel.passed());
        assert_eq!(panel.pass_count(), 6);
        assert_eq!(panel.fail_count(), 2);
        assert_eq!(panel.certificates().len(), 8);

        let failed = panel.failed_certs();
        assert_eq!(failed.len(), 2);

        // gate_13 is Structural
        assert_eq!(failed[0].severity, CertificateSeverity::Structural);
        // gate_15 is Durability
        assert_eq!(failed[1].severity, CertificateSeverity::Durability);

        let summary = panel.gate_summary();
        assert_eq!(summary.total, 8);
        assert_eq!(summary.passed, 6);
        assert_eq!(summary.failed, 2);

        // Structural: 5 pass, 1 fail (gate_07-11 pass, gate_13 fail)
        assert_eq!(summary.by_severity[0], (CertificateSeverity::Structural, 5, 1));
        // Taint: 1 pass, 0 fail (gate_14)
        assert_eq!(summary.by_severity[1], (CertificateSeverity::Taint, 1, 0));
        // Durability: 0 pass, 1 fail (gate_15)
        assert_eq!(summary.by_severity[2], (CertificateSeverity::Durability, 0, 1));
    }
}
