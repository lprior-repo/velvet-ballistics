use super::{VerifyError, record_budget_result};
use crate::args::VerifyProfile;
use vb_core::CompiledWorkflow;

pub(super) fn compile_registered(bytes: &[u8]) -> Result<CompiledWorkflow, VerifyError> {
    let compiled = vb_compile::compile_workflow(bytes).map_err(|errors| {
        VerifyError::Compile(errors.0.iter().map(ToString::to_string).collect())
    })?;
    if let Some(id) = crate::action_specs::unknown_action_id(&compiled) {
        return Err(VerifyError::Compile(vec![format!(
            "unknown action id: {id} (not in the CLI action registry)"
        )]));
    }
    Ok(compiled)
}

pub(super) fn checks(
    source: &vb_compile::WorkflowSource,
    compiled: &CompiledWorkflow,
    profile: VerifyProfile,
) -> Result<(Vec<&'static str>, Vec<String>), VerifyError> {
    let parts = compiled.to_parts();
    vb_validate::shared::validate(&parts)
        .map_err(|error| VerifyError::IrValidation(format!("IR validation failed: {error}")))?;
    let mut checks = vec!["yaml_parse", "compilation", "ir_validation"];
    let mut warnings = Vec::new();
    if matches!(profile, VerifyProfile::Standard | VerifyProfile::Full) {
        let budget = vb_core::budget::WholeWorkflowBudget::compute(
            &parts.nodes,
            parts.entry,
            &parts.resource_contract,
        );
        record_budget_result(profile, budget, &mut checks, &mut warnings)?;
    }
    verify_examples(source, compiled, &mut checks)?;
    Ok((checks, warnings))
}

fn verify_examples(
    source: &vb_compile::WorkflowSource,
    compiled: &CompiledWorkflow,
    checks: &mut Vec<&'static str>,
) -> Result<(), VerifyError> {
    if !source.examples().is_empty() {
        crate::workflow_examples::verify(source, compiled)
            .map_err(|error| VerifyError::Compile(vec![error]))?;
        checks.push("examples");
    }
    Ok(())
}

pub(super) fn ir_digest(compiled: &CompiledWorkflow) -> Result<String, VerifyError> {
    let bytes = postcard::to_allocvec(&compiled.to_parts())
        .map_err(|error| VerifyError::IrValidation(format!("IR serialization error: {error}")))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}
