//! Pure verification logic extracted from cmd_verify.
#![forbid(unsafe_code)]

use crate::args::{DurabilityMode, VerifyProfile};
use crate::exit_code::CliExitCode;
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

/// Structured result of a successful verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VerifyOk {
    /// Hex-encoded workflow digest.
    pub digest_hex: String,
    /// Number of compiled workflow nodes.
    pub node_count: u16,
    /// Master §63 gate statuses in canonical order.
    pub checks: Vec<&'static str>,
    /// Non-fatal warnings produced during verification.
    pub warnings: Vec<String>,
    /// Durability mode the verify call was tagged with (e.g. strict, journaled, none).
    pub durability_mode: DurabilityMode,
}

impl VerifyOk {
    pub(crate) fn all_gates_closed(&self) -> bool {
        self.checks
            .iter()
            .all(|check| !is_deferred_gate_status(check))
    }

    pub(crate) fn passed_gates(&self) -> Vec<&'static str> {
        self.checks
            .iter()
            .copied()
            .filter(|check| !is_deferred_gate_status(check))
            .collect()
    }

    pub(crate) fn deferred_gates(&self) -> Vec<&'static str> {
        self.checks
            .iter()
            .copied()
            .filter_map(|check| {
                if is_deferred_gate_status(check) {
                    Some(canonical_gate_name(check))
                } else {
                    None
                }
            })
            .collect()
    }
}

fn is_deferred_gate_status(status: &str) -> bool {
    status.ends_with(":deferred")
}

fn canonical_gate_name(status: &'static str) -> &'static str {
    if let Some(name) = status.strip_suffix(":deferred") {
        name
    } else {
        status
    }
}

/// Canonical verification gate outcomes reported by the CLI layer.
///
/// Gates backed directly by the local parse/compile/validate pipeline are
/// emitted as bare gate names. Gates that still rely on external registries,
/// runtime admission, or release evidence stay suffixed with `:deferred` so
/// the output stays faithful to master §63 without inventing replacement names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VerificationGateOutcomes {
    bounded: bool,
    budgets: bool,
    contracts: bool,
    taint: bool,
    idempotency: bool,
    durability: bool,
    capabilities: bool,
    evidence: bool,
}

impl VerificationGateOutcomes {
    const fn baseline_success() -> Self {
        Self {
            bounded: false,
            budgets: false,
            contracts: false,
            taint: false,
            idempotency: false,
            durability: false,
            capabilities: false,
            evidence: false,
        }
    }

    fn to_checks(self) -> [&'static str; 15] {
        [
            "profile",
            "shape",
            "names",
            "references",
            "expressions",
            "CFG",
            if self.bounded {
                "bounded"
            } else {
                "bounded:deferred"
            },
            if self.budgets {
                "budgets"
            } else {
                "budgets:deferred"
            },
            if self.contracts {
                "contracts"
            } else {
                "contracts:deferred"
            },
            if self.taint {
                "taint"
            } else {
                "taint:deferred"
            },
            if self.idempotency {
                "idempotency"
            } else {
                "idempotency:deferred"
            },
            if self.durability {
                "durability"
            } else {
                "durability:deferred"
            },
            if self.capabilities {
                "capabilities"
            } else {
                "capabilities:deferred"
            },
            "results",
            if self.evidence {
                "evidence"
            } else {
                "evidence:deferred"
            },
        ]
    }
}

/// Structured error from the verification pipeline.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// Full verification cannot pass while any canonical gate remains deferred.
    DeferredGates(VerifyOk),
}

/// Run the full verification pipeline on a workflow source text.
///
/// This performs:
/// 1. Strict YAML parse
/// 2. Compilation (schema, references, control flow, type/taint)
/// 3. IR validation
/// 4. Profile-dependent local checks that can upgrade master §63 gate statuses
///    from `:deferred` to fully checked at this CLI layer
///
/// The `durability_mode` is the durability profile the runtime intends to use
/// when this workflow is later executed. It is recorded in [`VerifyOk`] so the
/// emitted `VerificationReport` can populate its `durability` block
/// accurately rather than emitting a hard-coded placeholder.
///
/// Returns structured data suitable for formatting by the caller.
pub(crate) fn run_verification(
    text: &str,
    bytes: &[u8],
    profile: VerifyProfile,
    durability_mode: DurabilityMode,
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
    let mut gate_outcomes = VerificationGateOutcomes::baseline_success();
    let mut warnings: Vec<String> = Vec::new();

    // Phase 3: IR validation gates
    let parts = compiled.to_parts();
    match vb_validate::shared::validate(&parts) {
        Ok(()) => {}
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
                let policy = vb_core::budget::BoundednessPolicy::DEFAULT;
                match policy.validate(&_budget) {
                    Ok(()) => {
                        gate_outcomes.bounded = true;
                        gate_outcomes.budgets = true;
                    }
                    Err(e) => {
                        if profile == VerifyProfile::Full {
                            return Err(VerifyError::BudgetPolicy(format!(
                                "budget policy violation: {e}"
                            )));
                        }
                        warnings.push(format!("budget policy warning: {e}"));
                    }
                }
            }
            Err(e) => {
                warnings.push(format!("budget computation note: {e}"));
            }
        }

        // Phase 5: advisory structural checks.
        // These surface additional local warnings, but only promote a master
        // gate out of `:deferred` when this layer truly closes that gate.
        let _ = run_advisory_gate(&parts, "slot_bounds", check_slot_bounds, &mut warnings);
        let _ = run_advisory_gate(&parts, "taint", check_taint_propagation, &mut warnings);
        let _ = run_advisory_gate(&parts, "contracts", check_action_contracts, &mut warnings);
        let _ = run_advisory_gate(
            &parts,
            "capabilities",
            check_capability_requirements,
            &mut warnings,
        );
        let _ = run_advisory_gate(
            &parts,
            "determinism",
            check_replay_determinism,
            &mut warnings,
        );
        let _ = run_advisory_gate(&parts, "idempotency", check_idempotency, &mut warnings);
    }

    let checks = gate_outcomes.to_checks().to_vec();

    let result = VerifyOk {
        digest_hex,
        node_count: compiled.node_count(),
        checks,
        warnings,
        durability_mode,
    };

    if profile == VerifyProfile::Full && !result.all_gates_closed() {
        return Err(VerifyError::DeferredGates(result));
    }

    Ok(result)
}

/// Run a single local advisory gate.
///
/// A returned `true` means the local structural check passed. Callers may
/// still keep the corresponding master gate in `:deferred` state when the full
/// gate requires evidence or runtime inputs outside this CLI layer.
fn run_advisory_gate(
    parts: &WorkflowParts,
    gate_name: &'static str,
    check: fn(&WorkflowParts) -> Result<(), String>,
    warnings: &mut Vec<String>,
) -> bool {
    match check(parts) {
        Ok(()) => true,
        Err(message) => {
            warnings.push(format!("{gate_name} warning: {message}"));
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Extended gate implementations.
//
// Each function performs a meaningful static check against the compiled IR.
// Where the check would need runtime or external-registry data we do not
// have at verify time, the function returns a `not_implemented` warning
// string instead of silently passing. This is the policy demanded by
// `High #7` in the bead log: real work where possible, honest stubs
// otherwise.
// ---------------------------------------------------------------------------

/// Gate: every slot reference is within the declared `slot_count`.
fn check_slot_bounds(parts: &WorkflowParts) -> Result<(), String> {
    vb_validate::shared::validate_gate_09_slot_references(parts)
        .map_err(|err| format!("slot reference out of bounds: {err}"))
}

/// Gate: taint propagation.
///
/// No compiled-form taint validator over `WorkflowParts` exists in `vb_validate`
/// today. The compile pipeline's AST taint pass is useful upstream, but it does
/// not close the master §63 compiled-IR taint gate for this CLI layer.
fn check_taint_propagation(parts: &WorkflowParts) -> Result<(), String> {
    let _ = parts;
    Err(
        "compiled-form WorkflowParts taint validation is not implemented; AST validation alone does not close this gate"
            .to_string(),
    )
}

/// Gate: every `Do` node has a structurally valid `ActionId` and a valid
/// input slot. This is the structural portion of the action-contract gate;
/// the contract-completeness portion is enforced at runtime by the action
/// registry.
fn check_action_contracts(parts: &WorkflowParts) -> Result<(), String> {
    let slot_count = usize::from(parts.slot_count);
    let mut do_count: u32 = 0;
    for node in parts.nodes.iter() {
        if let CompiledNodeKind::Do { input, .. } = &node.kind {
            if input.as_usize() >= slot_count {
                return Err(format!(
                    "Do node at step {:?} has out-of-bounds input slot {:?}",
                    node.id, input
                ));
            }
            do_count = do_count.saturating_add(1);
        }
    }
    // `do_count <= u16::MAX` is a hard cap on action tickets for v1.
    if do_count > u32::from(u16::MAX) {
        return Err(format!("action ticket count {do_count} exceeds u16::MAX"));
    }
    Ok(())
}

/// Gate: every required capability on a `Do` node is within the
/// `ResourceContract` cap. Without an external capability registry the
/// strongest honest check is a structural one: every `Do` node must have a
/// valid input slot and the action ID must be in range.
fn check_capability_requirements(parts: &WorkflowParts) -> Result<(), String> {
    let slot_count = usize::from(parts.slot_count);
    for node in parts.nodes.iter() {
        if let CompiledNodeKind::Do { input, .. } = &node.kind
            && input.as_usize() >= slot_count
        {
            return Err(format!(
                "Do node at step {:?} cannot satisfy capability check: out-of-bounds input slot",
                node.id
            ));
        }
    }
    Ok(())
}

/// Gate: deterministic replay. Runs the existing `validate_gate_15_determinism_proof`
/// against the compiled parts.
fn check_replay_determinism(parts: &WorkflowParts) -> Result<(), String> {
    vb_validate::shared::validate_gate_15_determinism_proof(parts)
        .map_err(|err| format!("replay determinism check failed: {err}"))
}

/// Gate: idempotency. The structural check ensures every `Do` node has a
/// valid input slot (the same precondition required to derive a stable
/// idempotency key). Without an external action contract registry the
/// strongest honest verdict is a structural one.
fn check_idempotency(parts: &WorkflowParts) -> Result<(), String> {
    let slot_count = usize::from(parts.slot_count);
    for node in parts.nodes.iter() {
        if let CompiledNodeKind::Do { input, .. } = &node.kind
            && input.as_usize() >= slot_count
        {
            return Err(format!(
                "Do node at step {:?} cannot satisfy idempotency check: out-of-bounds input slot",
                node.id
            ));
        }
    }
    Ok(())
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
        VerifyError::DeferredGates(_) => CliExitCode::VerificationFailed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_WORKFLOW_YAML: &str =
        include_str!("../../workspace_tests/tests/fixtures/valid/minimal.yaml");
    const UNSUPPORTED_TOP_LEVEL_INPUTS_YAML: &str = r#"version: velvet-ballistics/v1
name: compile_scope_failure
when:
  manual: {}
inputs:
  count:
    type: u32
steps:
  - id: done
    finish:
      result: 0
"#;
    const QUICK_PROFILE_EXPECTED_CHECKS: [&str; 15] = [
        "profile",
        "shape",
        "names",
        "references",
        "expressions",
        "CFG",
        "bounded:deferred",
        "budgets:deferred",
        "contracts:deferred",
        "taint:deferred",
        "idempotency:deferred",
        "durability:deferred",
        "capabilities:deferred",
        "results",
        "evidence:deferred",
    ];

    fn expect_success(result: Result<VerifyOk, VerifyError>) -> VerifyOk {
        match result {
            Ok(ok) => ok,
            Err(err) => panic!("expected verification success, got {err:?}"),
        }
    }

    fn expect_deferred_failure(result: Result<VerifyOk, VerifyError>) -> VerifyOk {
        match result {
            Err(VerifyError::DeferredGates(ok)) => ok,
            Err(err) => panic!("expected deferred-gates failure, got {err:?}"),
            Ok(ok) => panic!("expected deferred-gates failure, got success {ok:?}"),
        }
    }
    const FULL_PROFILE_EXPECTED_CHECKS: [&str; 15] = [
        "profile",
        "shape",
        "names",
        "references",
        "expressions",
        "CFG",
        "bounded",
        "budgets",
        "contracts:deferred",
        "taint:deferred",
        "idempotency:deferred",
        "durability:deferred",
        "capabilities:deferred",
        "results",
        "evidence:deferred",
    ];

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
    fn exit_code_for_deferred_gates_is_verification_failed() {
        let err = VerifyError::DeferredGates(VerifyOk {
            digest_hex: "deadbeef".into(),
            node_count: 2,
            checks: FULL_PROFILE_EXPECTED_CHECKS.to_vec(),
            warnings: Vec::new(),
            durability_mode: DurabilityMode::None,
        });

        assert_eq!(exit_code_for_error(&err), CliExitCode::VerificationFailed);
    }

    #[test]
    fn verify_ok_holds_all_fields() {
        let ok = VerifyOk {
            digest_hex: "abcdef".into(),
            node_count: 5,
            checks: vec!["profile", "shape"],
            warnings: vec!["note: old format".into()],
            durability_mode: DurabilityMode::None,
        };
        assert_eq!(ok.digest_hex, "abcdef");
        assert_eq!(ok.node_count, 5);
        assert_eq!(ok.checks.len(), 2);
        assert_eq!(ok.warnings.len(), 1);
        assert_eq!(ok.durability_mode, DurabilityMode::None);
    }

    #[test]
    fn verify_ok_empty_checks_and_warnings() {
        let ok = VerifyOk {
            digest_hex: "00".into(),
            node_count: 0,
            checks: vec![],
            warnings: vec![],
            durability_mode: DurabilityMode::None,
        };
        assert!(ok.checks.is_empty());
        assert!(ok.warnings.is_empty());
        assert_eq!(ok.node_count, 0);
        assert_eq!(ok.durability_mode, DurabilityMode::None);
    }

    #[test]
    fn verify_ok_records_durability_mode() {
        let ok = VerifyOk {
            digest_hex: "deadbeef".into(),
            node_count: 3,
            checks: vec!["profile"],
            warnings: vec![],
            durability_mode: DurabilityMode::Strict,
        };
        assert_eq!(ok.durability_mode, DurabilityMode::Strict);
    }

    #[test]
    fn malformed_yaml_returns_yaml_parse_error() {
        let result = run_verification(
            "version: [",
            b"version: [",
            VerifyProfile::Quick,
            DurabilityMode::None,
        );

        match result {
            Err(VerifyError::YamlParse(message)) => {
                assert!(message.contains("YAML parse error"));
            }
            Err(err) => panic!("expected YAML parse error, got {err:?}"),
            Ok(ok) => panic!("expected YAML parse error, got success {ok:?}"),
        }
    }

    #[test]
    fn invalid_workflow_returns_compile_error() {
        let result = run_verification(
            UNSUPPORTED_TOP_LEVEL_INPUTS_YAML,
            UNSUPPORTED_TOP_LEVEL_INPUTS_YAML.as_bytes(),
            VerifyProfile::Quick,
            DurabilityMode::None,
        );

        match result {
            Err(VerifyError::Compile(errors)) => {
                assert!(!errors.is_empty());
                assert!(errors.iter().any(|error| error.contains("inputs")));
            }
            Err(err) => panic!("expected compile error, got {err:?}"),
            Ok(ok) => panic!("expected compile error, got success {ok:?}"),
        }
    }

    #[test]
    fn quick_profile_reports_master_gate_names_in_order() {
        let ok = expect_success(run_verification(
            MINIMAL_WORKFLOW_YAML,
            MINIMAL_WORKFLOW_YAML.as_bytes(),
            VerifyProfile::Quick,
            DurabilityMode::None,
        ));

        assert_eq!(ok.checks, QUICK_PROFILE_EXPECTED_CHECKS);
    }

    #[test]
    fn standard_profile_succeeds_with_deferred_gates_and_warnings() {
        let ok = expect_success(run_verification(
            MINIMAL_WORKFLOW_YAML,
            MINIMAL_WORKFLOW_YAML.as_bytes(),
            VerifyProfile::Standard,
            DurabilityMode::None,
        ));

        assert_eq!(ok.checks, FULL_PROFILE_EXPECTED_CHECKS);
        assert!(!ok.all_gates_closed());
        assert_eq!(
            ok.deferred_gates(),
            vec![
                "contracts",
                "taint",
                "idempotency",
                "durability",
                "capabilities",
                "evidence",
            ]
        );
        assert!(ok.warnings.iter().any(|warning| warning.contains(
            "compiled-form WorkflowParts taint validation is not implemented"
        )));
    }

    #[test]
    fn full_profile_fails_closed_when_deferred_gates_remain() {
        let ok = expect_deferred_failure(run_verification(
            MINIMAL_WORKFLOW_YAML,
            MINIMAL_WORKFLOW_YAML.as_bytes(),
            VerifyProfile::Full,
            DurabilityMode::None,
        ));

        assert_eq!(ok.checks, FULL_PROFILE_EXPECTED_CHECKS);
        assert!(!ok.all_gates_closed());
        assert!(ok.passed_gates().contains(&"bounded"));
        assert!(ok.deferred_gates().contains(&"evidence"));
    }

    #[test]
    fn success_path_records_digest_node_count_and_durability() {
        let compiled = match vb_compile::compile_workflow(MINIMAL_WORKFLOW_YAML.as_bytes()) {
            Ok(compiled) => compiled,
            Err(err) => panic!("expected fixture to compile, got {err:?}"),
        };
        let expected_digest: String = compiled
            .digest()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let ok = expect_success(run_verification(
            MINIMAL_WORKFLOW_YAML,
            MINIMAL_WORKFLOW_YAML.as_bytes(),
            VerifyProfile::Quick,
            DurabilityMode::Strict,
        ));

        assert_eq!(ok.digest_hex, expected_digest);
        assert_eq!(ok.node_count, compiled.node_count());
        assert_eq!(ok.durability_mode, DurabilityMode::Strict);
    }

    #[test]
    fn deferred_profile_omits_fabricated_gate_names() {
        let forbidden = [
            "digest_stability",
            "resource_contract_validation",
            "error_handler_completeness",
            "taint_boundary",
            "input_purity",
            "expression_complexity",
            "cycle_detection",
            "determinism_seed",
            "replay_round_trip",
        ];
        let ok = expect_deferred_failure(run_verification(
            MINIMAL_WORKFLOW_YAML,
            MINIMAL_WORKFLOW_YAML.as_bytes(),
            VerifyProfile::Full,
            DurabilityMode::None,
        ));

        for forbidden_gate in forbidden {
            assert!(!ok.checks.contains(&forbidden_gate));
        }
    }
}
