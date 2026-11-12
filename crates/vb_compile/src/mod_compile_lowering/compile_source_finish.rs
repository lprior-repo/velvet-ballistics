#![forbid(unsafe_code)]
//! Final workflow assembly with immutable compiler symbol binding.

use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::source_symbol_binding::source_symbol_error;
use crate::source_symbols::SourceSymbolTable;
use vb_core::workflow::source_metadata::SourceMetadataError;
use vb_core::workflow::{VersionedWorkflowParts, WorkflowIrV2};
use vb_core::{
    CompiledWorkflow, ResourceContract, StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
};

pub(super) fn finalize_compiled_workflow(
    source: &crate::WorkflowSource,
    semantic_source_identity: WorkflowDigest,
    builder: SlotCompiler,
    step_names: Vec<Box<str>>,
    layout: &[CanonicalStepLayout],
) -> Result<CompiledWorkflow, CompileErrors> {
    let (parts, symbols) =
        assemble_source_parts(source, semantic_source_identity, builder, step_names)?;
    let versioned = source_version(parts, symbols)?;
    let digest = crate::compute_compiled_ir_digest(&versioned)?;
    construct_source_workflow(versioned, digest)
        .map_err(|error| CompileErrors(vec![named_workflow_error(error, source.steps(), layout)]))
}

fn assemble_source_parts(
    source: &crate::WorkflowSource,
    semantic_source_identity: WorkflowDigest,
    builder: SlotCompiler,
    step_names: Vec<Box<str>>,
) -> Result<(WorkflowParts, SourceSymbolTable), CompileErrors> {
    let parts = WorkflowParts {
        name: Box::from(source.name()),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        semantic_source_identity: Some(semantic_source_identity),
        slot_count: builder.slot_count().map_err(|e| CompileErrors(vec![e]))?,
        symbols_count: compiler_symbol_count(builder.source_symbols.len())?,
        nodes: builder.nodes.into_boxed_slice(),
        expressions: builder.expressions.into_boxed_slice(),
        accessors: builder.accessors.into_boxed_slice(),
        constants: builder.constants.into_boxed_slice(),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: step_names.into_boxed_slice(),
        input_slots: builder.input_slots.into_boxed_slice(),
    };
    Ok((parts, builder.source_symbols))
}

fn compiler_symbol_count(count: usize) -> Result<u32, CompileErrors> {
    u32::try_from(count).map_err(|_| {
        CompileErrors(vec![CompileError::Workflow(
            WorkflowError::InvalidSymbolMetadata {
                source: SourceMetadataError::SymbolIndexOverflow,
            },
        )])
    })
}

fn source_version(
    parts: WorkflowParts,
    symbols: SourceSymbolTable,
) -> Result<VersionedWorkflowParts, CompileErrors> {
    if symbols.is_empty() {
        return Ok(VersionedWorkflowParts::V1(parts));
    }
    let symbol_payloads = symbols
        .finish()
        .map_err(|error| CompileErrors(vec![source_symbol_error(error, 0)]))?;
    Ok(VersionedWorkflowParts::V2(WorkflowIrV2 {
        wire_version: 2,
        parts,
        symbol_payloads,
    }))
}

fn validated_source_parts(
    mut parts: WorkflowParts,
    digest: WorkflowDigest,
) -> Result<WorkflowParts, CompileError> {
    parts.digest = digest;
    vb_validate::shared::validate(&parts)?;
    Ok(parts)
}

fn construct_source_workflow(
    versioned: VersionedWorkflowParts,
    digest: WorkflowDigest,
) -> Result<CompiledWorkflow, CompileError> {
    match versioned {
        VersionedWorkflowParts::V1(parts) => {
            CompiledWorkflow::try_from_parts(validated_source_parts(parts, digest)?)
                .map_err(CompileError::from)
        }
        VersionedWorkflowParts::V2(ir) => CompiledWorkflow::try_from_parts_with_symbols(
            validated_source_parts(ir.parts, digest)?,
            ir.symbol_payloads,
        )
        .map_err(CompileError::from),
    }
}
