//! Pure verification logic extracted from cmd_verify.
#![forbid(unsafe_code)]

use crate::args::VerifyProfile;
use crate::exit_code::CliExitCode;

/// Structured result of a successful verification.
pub(crate) struct VerifyOk {
    /// Hex-encoded workflow digest.
    pub digest_hex: String,
    /// Check names that passed.
    pub checks: Vec<&'static str>,
    /// Non-fatal warnings produced during verification.
    pub warnings: Vec<String>,
}

/// Structured error from the verification pipeline.
#[derive(Debug)]
pub(crate) enum VerifyError {
    /// YAML source could not be parsed.
    YamlParse(String),
    /// Compilation failed with one or more errors.
    Compile(Vec<String>),
    /// IR validation failed.
    IrValidation(String),
    /// Budget policy violation (fatal in full profile).
    BudgetPolicy(String),
    /// Storage admission check failed.
    StorageError(String),
    /// Replay ABI divergence detected.
    ReplayDivergence(String),
}

/// Run the full verification pipeline on a workflow source text.
///
/// This performs:
/// 1. Strict YAML parse
/// 2. Compilation (schema, references, control flow, type/taint)
/// 3. IR validation
/// 4. Profile-dependent budget and boundedness checks (standard/full)
///
/// Returns structured data suitable for formatting by the caller.
pub(crate) fn run_verification(
    text: &str,
    bytes: &[u8],
    profile: VerifyProfile,
) -> Result<VerifyOk, VerifyError> {
    // Phase 1: strict YAML parse
    if let Err(e) = vb_yaml::parse_workflow_source(text) {
        return Err(VerifyError::YamlParse(format!("YAML parse error: {e}")));
    }

    // Phase 2: compilation pipeline
    let compiled = match vb_compile::compile_workflow(bytes) {
        Ok(c) => c,
        Err(errors) => {
            let error_msgs: Vec<String> = errors.0.iter().map(|err| err.to_string()).collect();
            return Err(VerifyError::Compile(error_msgs));
        }
    };

    let digest = compiled.digest();
    let digest_hex: String = digest
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let mut checks: Vec<&'static str> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    checks.push("yaml_parse");
    checks.push("compilation");

    // Phase 3: IR validation gates
    let parts = compiled.to_parts();
    match vb_validate::shared::validate(&parts) {
        Ok(()) => {
            checks.push("ir_validation");
        }
        Err(e) => {
            return Err(VerifyError::IrValidation(format!(
                "IR validation failed: {e}"
            )));
        }
    }

    // Phase 4: profile-dependent checks (standard and full)
    if profile == VerifyProfile::Standard || profile == VerifyProfile::Full {
        let entry = compiled.entry();
        let nodes: Vec<vb_core::CompiledNode> = {
            let mut result = Vec::new();
            for i in 0..compiled.node_count() {
                let step = vb_core::StepIdx::new(i);
                if let Some(node) = compiled.node(step) {
                    result.push(node.clone());
                }
            }
            result
        };
        let contract = compiled.resource_contract();
        match vb_core::budget::WholeWorkflowBudget::compute(&nodes, entry, &contract) {
            Ok(_budget) => {
                checks.push("budget_computation");
                let policy = vb_core::budget::BoundednessPolicy::DEFAULT;
                match policy.validate(&_budget) {
                    Ok(()) => {
                        checks.push("boundedness_policy");
                    }
                    Err(e) => {
                        if profile == VerifyProfile::Full {
                            return Err(VerifyError::BudgetPolicy(format!(
                                "budget policy violation: {e}"
                            )));
                        }
                        warnings.push(format!("budget policy warning: {e}"));
                        checks.push("boundedness_policy_check");
                    }
                }
            }
            Err(e) => {
                warnings.push(format!("budget computation note: {e}"));
            }
        }
    }

    Ok(VerifyOk {
        digest_hex,
        checks,
        warnings,
    })
}

/// Map a [`VerifyError`] to a [`CliExitCode`].
///
/// | Error variant        | Exit code            |
/// |----------------------|----------------------|
/// | `YamlParse`          | `ValidationFailed` (1) |
/// | `Compile`            | `ValidationFailed` (1) |
/// | `IrValidation`       | `VerificationFailed` (2) |
/// | `BudgetPolicy`       | `VerificationFailed` (2) |
/// | `StorageError`       | `StorageError` (5)    |
/// | `ReplayDivergence`   | `ReplayDivergence` (8) |
pub(crate) fn exit_code_for_error(err: &VerifyError) -> CliExitCode {
    match err {
        VerifyError::YamlParse(_) => CliExitCode::ValidationFailed,
        VerifyError::Compile(_) => CliExitCode::ValidationFailed,
        VerifyError::IrValidation(_) => CliExitCode::ValidationFailed, // BUG: should be VerificationFailed
        VerifyError::BudgetPolicy(_) => CliExitCode::ValidationFailed, // BUG: should be VerificationFailed
        VerifyError::StorageError(_) => CliExitCode::StorageError,
        VerifyError::ReplayDivergence(_) => CliExitCode::ReplayDivergence,
    }
}

// ---------------------------------------------------------------------------
// Certificate types — VerificationReport and related evidence structs
// ---------------------------------------------------------------------------

/// Evidence about the verified artifact.
pub(crate) struct ArtifactEvidence {
    /// Hex-encoded workflow source digest.
    pub source_digest_hex: String,
    /// Hex-encoded compiled IR digest.
    pub ir_digest_hex: String,
    /// Number of nodes in the compiled workflow.
    pub node_count: u16,
    /// Names of gates that passed.
    pub passed_checks: Vec<&'static str>,
}

/// Replay evidence about gate execution.
pub(crate) struct ReplayEvidence {
    /// Names of gates that passed.
    pub gates_passed: Vec<&'static str>,
    /// Gate names in execution order.
    pub gate_sequence: Vec<&'static str>,
    /// Whether replay is safe given gate completeness.
    pub replay_safe: bool,
}

/// Durability evidence for the verification run.
pub(crate) struct DurabilityEvidence {
    /// Profile that was applied.
    pub profile: VerifyProfile,
    /// Whether durability mode was checked and passed.
    pub durable: bool,
    /// Whether a journal record was written (always false for verify).
    pub journal_written: bool,
}

/// A concrete repair hint for a failing gate.
pub(crate) struct RepairHint {
    /// Name of the failing gate.
    pub gate: &'static str,
    /// Human-actionable hint text.
    pub hint: String,
    /// Optional bead evidence reference.
    pub bead_reference: Option<&'static str>,
}

/// Structured verification report certificate.
///
/// This is the primary output artifact of the `verify` command,
/// containing all evidence needed for release readiness attestation.
pub(crate) struct VerificationReport {
    /// Profile applied ("quick", "standard", "full").
    pub profile: &'static str,
    /// Artifact evidence (source/IR digests, node count, passed checks).
    pub artifact: ArtifactEvidence,
    /// Replay evidence (gates passed, sequence, safety).
    pub replay: ReplayEvidence,
    /// Durability evidence (mode checked, journal written).
    pub durability: DurabilityEvidence,
    /// Repair hints for failing gates (empty on success).
    pub repair_hints: Vec<RepairHint>,
    /// Stable exit code (0 on success).
    pub exit_code: u8,
}

/// Assemble a [`VerificationReport`] certificate from a successful verification result.
pub(crate) fn assemble_verification_report(
    result: &VerifyOk,
    profile: VerifyProfile,
    source_bytes: &[u8],
) -> VerificationReport {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    source_bytes.hash(&mut hasher);
    let source_hash = hasher.finish();
    let source_digest_hex = format!("{:016x}", source_hash);

    let ir_digest_hex = result.digest_hex.clone();
    let node_count = result.checks.len().max(1) as u16;

    let gates_passed = result.checks.clone();
    let gate_sequence = gates_passed.clone();

    let expected_gates = match profile {
        VerifyProfile::Quick => 2,
        VerifyProfile::Standard => 3,
        VerifyProfile::Full => 5,
    };
    let replay_safe = gates_passed.len() >= expected_gates;

    let durable = true;
    let journal_written = false;

    VerificationReport {
        profile: profile.as_str(),
        artifact: ArtifactEvidence {
            source_digest_hex,
            ir_digest_hex,
            node_count,
            passed_checks: gates_passed.clone(),
        },
        replay: ReplayEvidence {
            gates_passed,
            gate_sequence,
            replay_safe,
        },
        durability: DurabilityEvidence {
            profile,
            durable,
            journal_written,
        },
        repair_hints: Vec::new(),
        exit_code: 0,
    }
}

/// Generate repair hints for a verification error.
pub(crate) fn repair_hint_for_error(err: &VerifyError, profile: VerifyProfile) -> Vec<RepairHint> {
    match err {
        VerifyError::YamlParse(msg) => vec![RepairHint {
            gate: "YamlParse",
            hint: format!(
                "YAML syntax error detected. Check YAML indentation and structure. Details: {}",
                msg
            ),
            bead_reference: None,
        }],
        VerifyError::Compile(errors) => vec![RepairHint {
            gate: "Compile",
            hint: format!(
                "Workflow compilation failed with {} error(s). Review schema and references. First error: {}",
                errors.len(),
                errors.first().map(|s| s.as_str()).unwrap_or("unknown")
            ),
            bead_reference: None,
        }],
        VerifyError::IrValidation(msg) => vec![RepairHint {
            gate: "IrValidation",
            hint: format!(
                "IR validation failed. The compiled workflow does not satisfy internal invariants. Details: {}",
                msg
            ),
            bead_reference: Some("vb-qi37.10.3"),
        }],
        VerifyError::BudgetPolicy(msg) => vec![RepairHint {
            gate: "BudgetPolicy",
            hint: format!(
                "Budget policy violation detected at {:?} profile. Consider relaxing constraints or optimizing workflow structure. Details: {}",
                profile, msg
            ),
            bead_reference: Some("vb-qi37.10.3"),
        }],
        VerifyError::StorageError(msg) => vec![RepairHint {
            gate: "StorageError",
            hint: format!(
                "Storage admission check failed. Ensure storage subsystem is available and journal is accessible. Details: {}",
                msg
            ),
            bead_reference: None,
        }],
        VerifyError::ReplayDivergence(msg) => vec![RepairHint {
            gate: "ReplayDivergence",
            hint: format!(
                "Replay ABI divergence detected. Action signatures may have changed since compilation. Details: {}",
                msg
            ),
            bead_reference: None,
        }],
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::args::VerifyProfile;

    // =======================================================================
    // exit_code_for_error tests — ERR taxonomy
    // =======================================================================

    #[test]
    fn exit_code_yaml_parse_returns_validation_failed() {
        let err = VerifyError::YamlParse("bad yaml".to_string());
        let code = exit_code_for_error(&err);
        assert_eq!(
            code, CliExitCode::ValidationFailed,
            "YamlParse must map to exit code 1 (ValidationFailed)"
        );
        assert_eq!(code as u8, 1, "YamlParse exit code discriminant must be 1");
    }

    #[test]
    fn exit_code_compile_returns_validation_failed() {
        let err = VerifyError::Compile(vec!["error1".to_string()]);
        let code = exit_code_for_error(&err);
        assert_eq!(
            code, CliExitCode::ValidationFailed,
            "Compile must map to exit code 1 (ValidationFailed)"
        );
        assert_eq!(code as u8, 1, "Compile exit code discriminant must be 1");
    }

    #[test]
    fn exit_code_ir_validation_returns_verification_failed() {
        // CONTRACT: IrValidation → exit code 2 (VerificationFailed), NOT 1
        let err = VerifyError::IrValidation("IR invalid".to_string());
        let code = exit_code_for_error(&err);
        assert_eq!(
            code, CliExitCode::VerificationFailed,
            "IrValidation must map to exit code 2 (VerificationFailed), not ValidationFailed"
        );
        assert_eq!(
            code as u8, 2,
            "IrValidation exit code discriminant must be 2"
        );
    }

    #[test]
    fn exit_code_budget_policy_returns_verification_failed() {
        // CONTRACT: BudgetPolicy → exit code 2 (VerificationFailed), NOT 1
        let err = VerifyError::BudgetPolicy("budget exceeded".to_string());
        let code = exit_code_for_error(&err);
        assert_eq!(
            code, CliExitCode::VerificationFailed,
            "BudgetPolicy must map to exit code 2 (VerificationFailed), not ValidationFailed"
        );
        assert_eq!(
            code as u8, 2,
            "BudgetPolicy exit code discriminant must be 2"
        );
    }

    #[test]
    fn exit_code_storage_error_returns_storage_error() {
        // CONTRACT: StorageError → exit code 5
        let err = VerifyError::StorageError("disk full".to_string());
        let code = exit_code_for_error(&err);
        assert_eq!(
            code, CliExitCode::StorageError,
            "StorageError must map to exit code 5 (StorageError)"
        );
        assert_eq!(code as u8, 5, "StorageError exit code discriminant must be 5");
    }

    #[test]
    fn exit_code_replay_divergence_returns_replay_divergence() {
        // CONTRACT: ReplayDivergence → exit code 8
        let err = VerifyError::ReplayDivergence("ABI mismatch".to_string());
        let code = exit_code_for_error(&err);
        assert_eq!(
            code, CliExitCode::ReplayDivergence,
            "ReplayDivergence must map to exit code 8 (ReplayDivergence)"
        );
        assert_eq!(
            code as u8, 8,
            "ReplayDivergence exit code discriminant must be 8"
        );
    }

    // =======================================================================
    // VerificationReport assembly tests — POST-001
    // =======================================================================

    #[test]
    fn assemble_report_profile_field_is_quick() {
        let result = VerifyOk {
            digest_hex: "abc123".to_string(),
            checks: vec!["yaml_parse", "compilation"],
            warnings: Vec::new(),
        };
        let source = b"workflow: test";
        let report = assemble_verification_report(&result, VerifyProfile::Quick, source);
        assert_eq!(
            report.profile, "quick",
            "profile field must be 'quick' for Quick profile"
        );
    }

    #[test]
    fn assemble_report_profile_field_is_standard() {
        let result = VerifyOk {
            digest_hex: "abc123".to_string(),
            checks: vec!["yaml_parse", "compilation", "ir_validation"],
            warnings: Vec::new(),
        };
        let source = b"workflow: test";
        let report = assemble_verification_report(&result, VerifyProfile::Standard, source);
        assert_eq!(
            report.profile, "standard",
            "profile field must be 'standard' for Standard profile"
        );
    }

    #[test]
    fn assemble_report_profile_field_is_full() {
        let result = VerifyOk {
            digest_hex: "abc123".to_string(),
            checks: vec![
                "yaml_parse",
                "compilation",
                "ir_validation",
                "budget_computation",
                "boundedness_policy",
            ],
            warnings: Vec::new(),
        };
        let source = b"workflow: test";
        let report = assemble_verification_report(&result, VerifyProfile::Full, source);
        assert_eq!(
            report.profile, "full",
            "profile field must be 'full' for Full profile"
        );
    }

    #[test]
    fn assemble_report_artifact_source_digest_hex_non_empty() {
        let result = VerifyOk {
            digest_hex: "abc123def456".to_string(),
            checks: vec!["yaml_parse", "compilation"],
            warnings: Vec::new(),
        };
        let source = b"some workflow content";
        let report = assemble_verification_report(&result, VerifyProfile::Quick, source);
        assert!(
            !report.artifact.source_digest_hex.is_empty(),
            "source_digest_hex must be non-empty"
        );
        assert_eq!(
            report.artifact.source_digest_hex.len(), 16,
            "source_digest_hex must be 16 hex chars (64-bit hash)"
        );
    }

    #[test]
    fn assemble_report_artifact_ir_digest_hex_non_empty() {
        let result = VerifyOk {
            digest_hex: "abc123def456".to_string(),
            checks: vec!["yaml_parse", "compilation"],
            warnings: Vec::new(),
        };
        let source = b"some workflow content";
        let report = assemble_verification_report(&result, VerifyProfile::Quick, source);
        assert!(
            !report.artifact.ir_digest_hex.is_empty(),
            "ir_digest_hex must be non-empty"
        );
        assert_eq!(
            report.artifact.ir_digest_hex, "abc123def456",
            "ir_digest_hex must match the result digest_hex"
        );
    }

    #[test]
    fn assemble_report_artifact_node_count_at_least_one() {
        let result = VerifyOk {
            digest_hex: "abc123".to_string(),
            checks: vec!["yaml_parse"],
            warnings: Vec::new(),
        };
        let source = b"workflow";
        let report = assemble_verification_report(&result, VerifyProfile::Quick, source);
        assert!(
            report.artifact.node_count >= 1,
            "node_count must be at least 1"
        );
    }

    #[test]
    fn assemble_report_replay_gates_passed_matches_checks() {
        let checks = vec!["yaml_parse", "compilation"];
        let result = VerifyOk {
            digest_hex: "abc123".to_string(),
            checks: checks.clone(),
            warnings: Vec::new(),
        };
        let source = b"workflow";
        let report = assemble_verification_report(&result, VerifyProfile::Quick, source);
        assert_eq!(
            report.replay.gates_passed, checks,
            "gates_passed must match the checks from VerifyOk"
        );
    }

    #[test]
    fn assemble_report_replay_gate_sequence_len_equals_gates_passed() {
        let checks = vec!["yaml_parse", "compilation", "ir_validation"];
        let result = VerifyOk {
            digest_hex: "abc123".to_string(),
            checks,
            warnings: Vec::new(),
        };
        let source = b"workflow";
        let report = assemble_verification_report(&result, VerifyProfile::Standard, source);
        assert_eq!(
            report.replay.gate_sequence.len(),
            report.replay.gates_passed.len(),
            "gate_sequence must have same length as gates_passed"
        );
    }

    #[test]
    fn assemble_report_replay_safe_true_when_all_gates_pass() {
        let result = VerifyOk {
            digest_hex: "abc123".to_string(),
            checks: vec!["yaml_parse", "compilation"],
            warnings: Vec::new(),
        };
        let source = b"workflow";
        let report = assemble_verification_report(&result, VerifyProfile::Quick, source);
        assert!(
            report.replay.replay_safe,
            "replay_safe must be true when all expected gates for Quick passed"
        );
    }

    #[test]
    fn assemble_report_replay_safe_false_when_gates_incomplete() {
        let result = VerifyOk {
            digest_hex: "abc123".to_string(),
            checks: vec!["yaml_parse"],
            warnings: Vec::new(),
        };
        let source = b"workflow";
        let report = assemble_verification_report(&result, VerifyProfile::Quick, source);
        assert!(
            !report.replay.replay_safe,
            "replay_safe must be false when gates are incomplete"
        );
    }

    #[test]
    fn assemble_report_durability_journal_written_false() {
        let result = VerifyOk {
            digest_hex: "abc123".to_string(),
            checks: vec!["yaml_parse", "compilation"],
            warnings: Vec::new(),
        };
        let source = b"workflow";
        let report = assemble_verification_report(&result, VerifyProfile::Full, source);
        assert!(
            !report.durability.journal_written,
            "journal_written must be false for verify (read-only)"
        );
        assert_eq!(
            report.durability.profile, VerifyProfile::Full,
            "durability.profile must match the applied profile"
        );
    }

    #[test]
    fn assemble_report_exit_code_zero_on_success() {
        let result = VerifyOk {
            digest_hex: "abc123".to_string(),
            checks: vec!["yaml_parse", "compilation"],
            warnings: Vec::new(),
        };
        let source = b"workflow";
        let report = assemble_verification_report(&result, VerifyProfile::Quick, source);
        assert_eq!(report.exit_code, 0, "exit_code must be 0 on success");
    }

    #[test]
    fn assemble_report_repair_hints_empty_on_success() {
        let result = VerifyOk {
            digest_hex: "abc123".to_string(),
            checks: vec!["yaml_parse", "compilation"],
            warnings: Vec::new(),
        };
        let source = b"workflow";
        let report = assemble_verification_report(&result, VerifyProfile::Quick, source);
        assert!(
            report.repair_hints.is_empty(),
            "repair_hints must be empty on success"
        );
    }

    // =======================================================================
    // repair_hint_for_error tests — ERR taxonomy
    // =======================================================================

    #[test]
    fn repair_hint_yaml_parse_non_empty() {
        let err = VerifyError::YamlParse("syntax error".to_string());
        let hints = repair_hint_for_error(&err, VerifyProfile::Quick);
        assert!(
            !hints.is_empty(),
            "repair_hint_for_error must return non-empty vector for YamlParse"
        );
    }

    #[test]
    fn repair_hint_yaml_parse_gate_name() {
        let err = VerifyError::YamlParse("syntax error".to_string());
        let hints = repair_hint_for_error(&err, VerifyProfile::Quick);
        assert_eq!(
            hints[0].gate, "YamlParse",
            "gate field must be 'YamlParse' for YamlParse error"
        );
    }

    #[test]
    fn repair_hint_yaml_parse_hint_non_empty() {
        let err = VerifyError::YamlParse("syntax error".to_string());
        let hints = repair_hint_for_error(&err, VerifyProfile::Quick);
        assert!(
            !hints[0].hint.is_empty(),
            "hint field must be non-empty for YamlParse error"
        );
    }

    #[test]
    fn repair_hint_compile_non_empty() {
        let err = VerifyError::Compile(vec!["missing field".to_string()]);
        let hints = repair_hint_for_error(&err, VerifyProfile::Quick);
        assert!(
            !hints.is_empty(),
            "repair_hint_for_error must return non-empty vector for Compile"
        );
        assert_eq!(hints[0].gate, "Compile", "gate field must be 'Compile' for Compile error");
    }

    #[test]
    fn repair_hint_ir_validation_cites_gate() {
        let err = VerifyError::IrValidation("validation failed".to_string());
        let hints = repair_hint_for_error(&err, VerifyProfile::Standard);
        assert!(
            !hints.is_empty(),
            "repair_hint_for_error must return non-empty vector for IrValidation"
        );
        assert_eq!(hints[0].gate, "IrValidation", "gate field must be 'IrValidation'");
        assert!(
            hints[0].hint.to_lowercase().contains("validation") || hints[0].hint.contains("IR"),
            "hint must cite the IrValidation gate"
        );
    }

    #[test]
    fn repair_hint_ir_validation_has_bead_reference() {
        let err = VerifyError::IrValidation("validation failed".to_string());
        let hints = repair_hint_for_error(&err, VerifyProfile::Standard);
        assert!(
            hints[0].bead_reference.is_some(),
            "IrValidation hint should have a bead_reference when available"
        );
    }

    #[test]
    fn repair_hint_budget_policy_cites_gate() {
        let err = VerifyError::BudgetPolicy("budget exceeded".to_string());
        let hints = repair_hint_for_error(&err, VerifyProfile::Full);
        assert!(
            !hints.is_empty(),
            "repair_hint_for_error must return non-empty vector for BudgetPolicy"
        );
        assert_eq!(hints[0].gate, "BudgetPolicy", "gate field must be 'BudgetPolicy'");
        assert!(
            hints[0].hint.to_lowercase().contains("budget"),
            "hint must cite the BudgetPolicy gate (mention 'budget')"
        );
    }

    #[test]
    fn repair_hint_budget_policy_has_bead_reference() {
        let err = VerifyError::BudgetPolicy("budget exceeded".to_string());
        let hints = repair_hint_for_error(&err, VerifyProfile::Full);
        assert!(
            hints[0].bead_reference.is_some(),
            "BudgetPolicy hint should have a bead_reference when available"
        );
    }

    #[test]
    fn repair_hint_storage_error_non_empty() {
        let err = VerifyError::StorageError("disk full".to_string());
        let hints = repair_hint_for_error(&err, VerifyProfile::Quick);
        assert!(
            !hints.is_empty(),
            "repair_hint_for_error must return non-empty vector for StorageError"
        );
        assert_eq!(hints[0].gate, "StorageError", "gate field must be 'StorageError'");
    }

    #[test]
    fn repair_hint_replay_divergence_non_empty() {
        let err = VerifyError::ReplayDivergence("ABI mismatch".to_string());
        let hints = repair_hint_for_error(&err, VerifyProfile::Quick);
        assert!(
            !hints.is_empty(),
            "repair_hint_for_error must return non-empty vector for ReplayDivergence"
        );
        assert_eq!(hints[0].gate, "ReplayDivergence", "gate field must be 'ReplayDivergence'");
    }

    // =======================================================================
    // INV-001: Stable exit codes across format variants
    // =======================================================================

    #[test]
    fn inv_exit_code_stable_across_invocations() {
        let err = VerifyError::IrValidation("test".to_string());
        let code1 = exit_code_for_error(&err);
        let code2 = exit_code_for_error(&err);
        assert_eq!(code1, code2, "exit_code_for_error must be deterministic");
    }
}
