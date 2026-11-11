#![forbid(unsafe_code)]
//! Final workflow assembly for `compile_source` (part_01).
//!
//! The top-level compile pipeline ends by encoding the lowered parts,
//! stamping the artifact digest, and validating the result; this helper
//! is a verbatim relocation of that final segment. The slot compiler
//! and step names are moved in by value because the pipeline owns them
//! through the end of compilation.

use super::*;
use crate::mod_compile_errors::CompileErrors;
use vb_core::{CompiledWorkflow, ResourceContract, StepIdx, WorkflowDigest, WorkflowParts};

pub(super) fn finalize_compiled_workflow(
    source: &crate::WorkflowSource,
    semantic_source_identity: WorkflowDigest,
    builder: SlotCompiler,
    step_names: Vec<Box<str>>,
    steps: &[crate::StepAst],
    layout: &[CanonicalStepLayout],
) -> Result<CompiledWorkflow, CompileErrors> {
    let mut parts = WorkflowParts {
        name: Box::from(source.name()),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        semantic_source_identity: Some(semantic_source_identity),
        slot_count: builder.slot_count().map_err(|e| CompileErrors(vec![e]))?,
        symbols_count: 0,
        nodes: builder.nodes.into_boxed_slice(),
        expressions: builder.expressions.into_boxed_slice(),
        accessors: builder.accessors.into_boxed_slice(),
        constants: builder.constants.into_boxed_slice(),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: step_names.into_boxed_slice(),
        input_slots: builder.input_slots.into_boxed_slice(),
    };
    parts.digest = crate::compute_compiled_ir_digest(
        &vb_core::workflow::VersionedWorkflowParts::V1(parts.clone()),
    )?;
    vb_validate::shared::validate(&parts)
        .map_err(|e| CompileErrors(vec![named_workflow_error(e.into(), steps, layout)]))?;
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|e| CompileErrors(vec![named_workflow_error(e.into(), steps, layout)]))
}
