#![forbid(unsafe_code)]
//! Final workflow assembly with compiler symbol binding and YAML source metadata.
//!
//! Combines core V2 symbol payload binding with YAML WorkflowSourceMetadata
//! carrying slot-bearing secret requirements. The kernel owner will preserve
//! metadata/absence/roundtrip through validated construction.

use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::source_symbol_binding::source_symbol_error;
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
    let (parts, symbol_payloads) =
        assemble_source_parts(source, semantic_source_identity, builder, step_names)?;
    let versioned = source_version(parts, symbol_payloads)?;
    let digest = crate::compute_compiled_ir_digest(&versioned)?;
    construct_source_workflow(versioned, digest)
        .map_err(|error| CompileErrors(vec![named_workflow_error(error, source.steps(), layout)]))
}

fn assemble_source_parts(
    source: &crate::WorkflowSource,
    semantic_source_identity: WorkflowDigest,
    mut builder: SlotCompiler,
    mut step_names: Vec<Box<str>>,
) -> Result<(WorkflowParts, Box<[Box<str>]>), CompileErrors> {
    // Apply YAML display name overrides
    for (id, display) in &builder.display_names {
        if let Some(name) = step_names.get_mut(id.as_usize()) {
            *name = display.clone();
        }
    }

    // Finish the unified source_symbols table into ordered payloads
    let count = builder.source_symbols.len();
    let symbol_payloads = builder
        .source_symbols
        .finish()
        .map_err(|error| CompileErrors(vec![source_symbol_error(error, 0)]))?;

    // Sort secret requirements by alias for deterministic metadata
    builder
        .secret_requirements
        .sort_unstable_by_key(|binding| binding.alias());

    // Build YAML WorkflowSourceMetadata with slot-bearing secret requirements
    // The core V2 constructor only accepts payloads, so we carry requirements
    // through parts.source_metadata while providing payloads to the constructor.
    let source_metadata = if count > 0 {
        let metadata =
            vb_core::workflow::source_metadata::WorkflowSourceMetadata::try_new(
                symbol_payloads,
                builder
                    .secret_requirements
                    .into_boxed_slice(),
            )
            .map_err(|error| CompileErrors(vec![
                CompileError::Workflow(WorkflowError::InvalidSymbolMetadata { source: error }),
            ]))?;
        // Validate measurements match what the interner reported
        if metadata.symbols_count() != count {
            return Err(CompileErrors(vec![
                CompileError::Workflow(WorkflowError::InvalidSymbolMetadata {
                    source: SourceMetadataError::SymbolCountMismatch {
                        actual: metadata.symbols_count(),
                        expected: count,
                    },
                }),
            ]));
        }
        Some(Box::new(metadata))
    } else {
        builder
            .secret_requirements
            .into_boxed_slice();
        None
    };

    let parts = WorkflowParts {
        name: Box::from(source.name()),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        semantic_source_identity: Some(semantic_source_identity),
        source_metadata,
        slot_count: builder
            .slot_count()
            .map_err(|e| CompileErrors(vec![e]))?,
        symbols_count: count
            .try_into()
            .map_err(|_| {
                CompileErrors(vec![CompileError::Workflow(
                    WorkflowError::InvalidSymbolMetadata {
                        source: SourceMetadataError::SymbolIndexOverflow,
                    },
                )])
            })?,
        nodes: builder.nodes.into_boxed_slice(),
        expressions: builder.expressions.into_boxed_slice(),
        accessors: builder.accessors.into_boxed_slice(),
        constants: builder.constants.into_boxed_slice(),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: step_names.into_boxed_slice(),
        input_slots: builder.input_slots.into_boxed_slice(),
    };

    Ok((parts, symbol_payloads))
}

fn source_version(
    parts: WorkflowParts,
    symbol_payloads: Box<[Box<str>]>,
) -> Result<VersionedWorkflowParts, CompileErrors> {
    if symbol_payloads.is_empty() {
        return Ok(VersionedWorkflowParts::V1(parts));
    }
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
