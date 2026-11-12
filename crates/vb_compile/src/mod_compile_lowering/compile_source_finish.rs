#![forbid(unsafe_code)]
//! Finalize source metadata, encode the artifact digest and validate the lowered workflow.

use super::*;
use crate::{CompileError, CompileErrors};
use vb_core::{CompiledWorkflow, ResourceContract, StepIdx, WorkflowDigest, WorkflowParts};

pub(super) fn finalize_compiled_workflow(
    source: &crate::WorkflowSource,
    semantic_source_identity: WorkflowDigest,
    mut builder: SlotCompiler,
    mut step_names: Vec<Box<str>>,
    steps: &[crate::StepAst],
    layout: &[CanonicalStepLayout],
) -> Result<CompiledWorkflow, CompileErrors> {
    for (id, display) in &builder.display_names {
        if let Some(name) = step_names.get_mut(id.as_usize()) {
            *name = display.clone();
        }
    }
    let symbols = std::mem::take(&mut builder.symbols);
    builder
        .secret_requirements
        .sort_unstable_by_key(|binding| binding.alias());
    let metadata = if symbols.is_empty() {
        None
    } else {
        let count = symbols.len();
        let bytes = symbols.symbol_bytes();
        let symbols =
            symbols
                .finish()
                .map_err(|error| CompileError::ExpressionLoweringUnsupported {
                    feature: error.to_string().into(),
                })?;
        let metadata = vb_core::workflow::source_metadata::WorkflowSourceMetadata::try_new(
            symbols,
            std::mem::take(&mut builder.secret_requirements).into_boxed_slice(),
        )
        .map_err(|error| CompileError::ExpressionLoweringUnsupported {
            feature: error.to_string().into(),
        })?;
        if usize::try_from(metadata.symbols_count()).ok() != Some(count)
            || metadata.symbol_bytes() != bytes
        {
            return Err(CompileError::ExpressionLoweringUnsupported {
                feature: "symbol measurements".into(),
            }
            .into());
        }
        Some(Box::new(metadata))
    };
    let mut parts = WorkflowParts {
        name: Box::from(source.name()),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        semantic_source_identity: Some(semantic_source_identity),
        slot_count: builder.slot_count().map_err(|e| CompileErrors(vec![e]))?,
        symbols_count: metadata.as_ref().map_or(0, |value| value.symbols_count()),
        source_metadata: metadata,
        nodes: builder.nodes.into_boxed_slice(),
        expressions: builder.expressions.into_boxed_slice(),
        accessors: builder.accessors.into_boxed_slice(),
        constants: builder.constants.into_boxed_slice(),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: step_names.into_boxed_slice(),
        input_slots: builder.input_slots.into_boxed_slice(),
    };
    let encoded_parts = postcard::to_allocvec(&parts)
        .map_err(|error| CompileErrors(vec![CompileError::CompiledIrEncode(error)]))?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&encoded_parts).into());
    vb_validate::shared::validate(&parts)
        .map_err(|e| CompileErrors(vec![named_workflow_error(e.into(), steps, layout)]))?;
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|e| CompileErrors(vec![named_workflow_error(e.into(), steps, layout)]))
}
