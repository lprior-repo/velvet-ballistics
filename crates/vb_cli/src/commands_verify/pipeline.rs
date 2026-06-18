//! The verification pipeline — the five-phase static-analysis workflow.
//!
//! Phases:
//! 1. Strict YAML parse
//! 2. Compilation (schema, references, control flow, type/taint)
//! 3. IR validation
//! 4. Profile-dependent local checks (standard and full)
//! 5. Advisory structural checks that may upgrade master §63 gate statuses

#![forbid(unsafe_code)]

use crate::args::{DurabilityMode, VerifyProfile};
use crate::commands_verify::advisory::{
    check_action_contracts, check_capability_requirements, check_idempotency,
    check_replay_determinism, check_slot_bounds, check_taint_propagation, run_advisory_gate,
};
use crate::commands_verify::types::{VerificationGateOutcomes, VerifyError, VerifyOk};
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

/// Run the full verification pipeline on a workflow source text.
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
    let digest_hex = crate::commands_verify::types::workflow_digest_hex(digest);
    let mut gate_outcomes = VerificationGateOutcomes::baseline_success();
    let mut warnings: Vec<String> = Vec::new();

    // Phase 3: IR validation gates
    let parts = compiled.to_parts();
    let ir_bytes = match postcard::to_allocvec(&parts) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Err(VerifyError::IrValidation(format!(
                "compiled artifact serialization failed: {error}"
            )));
        }
    };
    let ir_digest_hex = crate::commands_verify::types::bytes_digest_hex(&ir_bytes);
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
        ir_digest_hex,
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
