//! Pure verification logic extracted from cmd_verify.
#![forbid(unsafe_code)]

use crate::args::VerifyProfile;
use crate::exit_code::CliExitCode;

/// Structured result of a successful verification.
pub(crate) struct VerifyOk {
    /// Hex-encoded workflow digest.
    pub digest_hex: String,
    /// Number of compiled workflow nodes.
    pub node_count: u16,
    /// Check names that passed.
    pub checks: Vec<&'static str>,
    /// Non-fatal warnings produced during verification.
    pub warnings: Vec<String>,
}

/// Structured error from the verification pipeline.
#[allow(dead_code)]
pub(crate) enum VerifyError {
    /// YAML source could not be parsed.
    YamlParse(String),
    /// Compilation failed with one or more errors.
    Compile(Vec<String>),
    /// IR validation failed.
    IrValidation(String),
    /// Budget policy violation (fatal in full profile).
    BudgetPolicy(String),
    /// Storage operation failed.
    StorageError(String),
    /// Replay divergence detected.
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
        node_count: compiled.node_count(),
        checks,
        warnings,
    })
}

/// Map a [`VerifyError`] to a [`CliExitCode`].
pub(crate) fn exit_code_for_error(err: &VerifyError) -> CliExitCode {
    match err {
        VerifyError::YamlParse(_) => CliExitCode::ValidationFailed,
        VerifyError::Compile(_) => CliExitCode::ValidationFailed,
        VerifyError::IrValidation(_) => CliExitCode::VerificationFailed,
        VerifyError::BudgetPolicy(_) => CliExitCode::VerificationFailed,
        VerifyError::StorageError(_) => CliExitCode::StorageError,
        VerifyError::ReplayDivergence(_) => CliExitCode::ReplayDivergence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_for_yaml_parse_is_validation_failed() {
        let err = VerifyError::YamlParse("bad yaml".into());
        assert_eq!(exit_code_for_error(&err), CliExitCode::ValidationFailed);
    }

    #[test]
    fn exit_code_for_compile_is_validation_failed() {
        let err = VerifyError::Compile(vec!["err1".into()]);
        assert_eq!(exit_code_for_error(&err), CliExitCode::ValidationFailed);
    }

    #[test]
    fn exit_code_for_ir_validation_is_verification_failed() {
        let err = VerifyError::IrValidation("bad ir".into());
        assert_eq!(exit_code_for_error(&err), CliExitCode::VerificationFailed);
    }

    #[test]
    fn exit_code_for_budget_policy_is_verification_failed() {
        let err = VerifyError::BudgetPolicy("over budget".into());
        assert_eq!(exit_code_for_error(&err), CliExitCode::VerificationFailed);
    }

    #[test]
    fn exit_code_for_storage_error_is_storage_error() {
        let err = VerifyError::StorageError("disk full".into());
        assert_eq!(exit_code_for_error(&err), CliExitCode::StorageError);
    }

    #[test]
    fn exit_code_for_replay_divergence_is_replay_divergence() {
        let err = VerifyError::ReplayDivergence("diverged".into());
        assert_eq!(exit_code_for_error(&err), CliExitCode::ReplayDivergence);
    }

    #[test]
    fn verify_ok_holds_all_fields() {
        let ok = VerifyOk {
            digest_hex: "abcdef".into(),
            node_count: 5,
            checks: vec!["yaml_parse", "compilation"],
            warnings: vec!["note: old format".into()],
        };
        assert_eq!(ok.digest_hex, "abcdef");
        assert_eq!(ok.node_count, 5);
        assert_eq!(ok.checks.len(), 2);
        assert_eq!(ok.warnings.len(), 1);
    }

    #[test]
    fn verify_ok_empty_checks_and_warnings() {
        let ok = VerifyOk {
            digest_hex: "00".into(),
            node_count: 0,
            checks: vec![],
            warnings: vec![],
        };
        assert!(ok.checks.is_empty());
        assert!(ok.warnings.is_empty());
        assert_eq!(ok.node_count, 0);
    }
}
