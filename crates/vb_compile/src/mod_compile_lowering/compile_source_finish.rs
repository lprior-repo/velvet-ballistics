#![forbid(unsafe_code)]
//! Final workflow assembly with one ordered source symbol table.

use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::source_symbol_binding::source_symbol_error;
use crate::source_symbols::SourceSymbolError;
use vb_core::workflow::source_metadata::{SourceMetadataError, WorkflowSourceMetadata};
use vb_core::workflow::{VersionedWorkflowParts, WorkflowIrV2};
use vb_core::{
    CompiledWorkflow, ResourceContract, StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
};

type SourceParts = (WorkflowParts, Box<[Box<str>]>);

struct Symbols {
    metadata: Option<Box<WorkflowSourceMetadata>>,
    payloads: Box<[Box<str>]>,
    count: u32,
}

struct PartsAssembly<'a> {
    source: &'a crate::WorkflowSource,
    identity: WorkflowDigest,
    builder: SlotCompiler,
    names: Vec<Box<str>>,
    metadata: Option<Box<WorkflowSourceMetadata>>,
    count: u32,
}

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
    identity: WorkflowDigest,
    mut builder: SlotCompiler,
    mut names: Vec<Box<str>>,
) -> Result<SourceParts, CompileErrors> {
    apply_display_names(&builder, &mut names);
    let Symbols {
        metadata,
        payloads,
        count,
    } = finish_symbols(&mut builder)?;
    let parts = build_parts(PartsAssembly {
        source,
        identity,
        builder,
        names,
        metadata,
        count,
    })?;
    Ok((parts, payloads))
}

fn apply_display_names(builder: &SlotCompiler, names: &mut [Box<str>]) {
    for (id, display) in &builder.display_names {
        if let Some(name) = names.get_mut(id.as_usize()) {
            *name = display.clone();
        }
    }
}

fn finish_symbols(builder: &mut SlotCompiler) -> Result<Symbols, CompileError> {
    let count = checked_symbol_count(builder.source_symbols.len())?;
    let bytes = builder.source_symbols.symbol_bytes();
    let empty = builder.source_symbols.is_empty();
    let table = std::mem::take(&mut builder.source_symbols);
    let payloads = table
        .finish()
        .map_err(|error| source_symbol_error(error, 0))?;
    let mut requirements = std::mem::take(&mut builder.secret_requirements);
    requirements.sort_unstable_by_key(|binding| binding.alias());
    let metadata = WorkflowSourceMetadata::try_new(payloads, requirements.into_boxed_slice())
        .map_err(metadata_error)?;
    validate_measurements(&metadata, count, bytes)?;
    let payloads = copy_payloads(metadata.symbols())?;
    let metadata = (!empty).then(|| Box::new(metadata));
    Ok(Symbols {
        metadata,
        payloads,
        count,
    })
}

fn checked_symbol_count(count: usize) -> Result<u32, CompileError> {
    u32::try_from(count).map_err(|_| source_symbol_error(SourceSymbolError::SymbolIndexOverflow, 0))
}

fn metadata_error(source: SourceMetadataError) -> CompileError {
    CompileError::Workflow(WorkflowError::InvalidSymbolMetadata { source })
}

fn validate_measurements(
    metadata: &WorkflowSourceMetadata,
    count: u32,
    bytes: u64,
) -> Result<(), CompileError> {
    if metadata.symbols_count() != count || metadata.symbol_bytes() != bytes {
        return Err(CompileError::ExpressionLoweringUnsupported {
            feature: "symbol measurements".into(),
        });
    }
    Ok(())
}

fn copy_payloads(payloads: &[Box<str>]) -> Result<Box<[Box<str>]>, CompileError> {
    let mut copied = Vec::new();
    copied
        .try_reserve_exact(payloads.len())
        .map_err(allocation_error)?;
    for payload in payloads {
        copied.push(copy_payload(payload)?);
    }
    Ok(copied.into_boxed_slice())
}

fn copy_payload(payload: &str) -> Result<Box<str>, CompileError> {
    let mut copied = String::new();
    copied
        .try_reserve_exact(payload.len())
        .map_err(allocation_error)?;
    copied.push_str(payload);
    Ok(copied.into_boxed_str())
}

fn allocation_error(_: std::collections::TryReserveError) -> CompileError {
    source_symbol_error(SourceSymbolError::AllocationFailed, 0)
}

fn build_parts(state: PartsAssembly<'_>) -> Result<WorkflowParts, CompileError> {
    let slot_count = state.builder.slot_count()?;
    Ok(WorkflowParts {
        name: Box::from(state.source.name()),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        semantic_source_identity: Some(state.identity),
        source_metadata: state.metadata,
        slot_count,
        symbols_count: state.count,
        nodes: state.builder.nodes.into_boxed_slice(),
        expressions: state.builder.expressions.into_boxed_slice(),
        accessors: state.builder.accessors.into_boxed_slice(),
        constants: state.builder.constants.into_boxed_slice(),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: state.names.into_boxed_slice(),
        input_slots: state.builder.input_slots.into_boxed_slice(),
    })
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
