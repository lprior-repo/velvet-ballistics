#![allow(unused_imports)]
use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors, non_string_key_error};
use crate::mod_compile_validation::{
    reject_unsupported_for_each_fields, validate_canonical_compile_scope,
};
use crate::{AuthorValue, InputField, WorkflowSource};
use saphyr::Yaml;
use std::collections::HashMap;
use vb_core::{
    AccessorProgram, CompiledInputSlot, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx,
    ConstValue, ExprIdx, ExprProgram, InputSlotKind, ResourceContract, SlotBranch, SlotIdx,
    StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
};
use vb_expr::expr_typecheck::ExprType;

#[path = "finish_inputs.rs"]
mod finish_inputs;
pub(crate) use finish_inputs::canonical_finish_slot_with_step_outputs;
#[path = "for_each_inputs.rs"]
mod for_each_inputs;
pub(in crate::mod_compile_lowering) use for_each_inputs::ForEachInputs;
#[path = "source_declarations.rs"]
mod source_declarations;
use super::source_slot_reservation::reserve_source_slots;
use source_declarations::prepare_source_declarations;

/// Compile the canonical cold YAML authoring AST into numeric runtime IR.
pub fn compile_source(source: &WorkflowSource) -> Result<CompiledWorkflow, CompileErrors> {
    validate_canonical_compile_scope(source)?;
    validate_branch_counts(source)?;
    let semantic_source_identity = canonical_digest(source)?;
    let steps = source.steps();
    let last = steps
        .len()
        .checked_sub(1)
        .ok_or(CompileErrors(vec![CompileError::EmptySteps]))?;
    let mut builder = SlotCompiler::new();
    reserve_source_slots(source, &mut builder).map_err(|e| CompileErrors(vec![e]))?;
    super::numeric_source_inputs::record_numeric_inputs(source, &mut builder)
        .map_err(|e| CompileErrors(vec![e]))?;
    let mut outputs: HashMap<String, SlotIdx> = HashMap::new();
    let mut step_outputs: HashMap<String, SlotIdx> = HashMap::new();
    let mut slot_types: HashMap<SlotIdx, ExprType> = HashMap::new();
    let mut for_each_inputs = ForEachInputs::new();
    let mut step_names = prepare_source_declarations(
        source,
        &mut outputs,
        &mut slot_types,
        &mut builder,
        &mut for_each_inputs,
    )?;
    let layout =
        canonical_layout(steps, builder.nodes.len()).map_err(|e| CompileErrors(vec![e]))?;
    step_names.extend(canonical_step_names(steps, &layout).map_err(|e| CompileErrors(vec![e]))?);
    for (index, step) in steps.iter().enumerate() {
        let id = layout_start(&layout, index).map_err(|e| CompileErrors(vec![e]))?;
        let mut next = next_layout_start(&layout, index).map_err(|e| CompileErrors(vec![e]))?;
        if index == last && source.result().is_some() {
            if matches!(step.primitive, crate::StepPrimitive::Finish { .. }) {
                return Err(CompileErrors(vec![CompileError::StepFieldShape {
                    step: index,
                    field: "finish",
                    expected: "either finish or a top-level result mapping",
                }]));
            }
            let width = canonical_step_width_with_condition(step)?;
            let end = id
                .as_usize()
                .checked_add(width)
                .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
            next = Some(step_idx(end)?);
        }
        lower_canonical_step(
            step,
            index,
            last,
            id,
            next,
            &mut outputs,
            &mut step_outputs,
            &mut step_names,
            &mut slot_types,
            &for_each_inputs,
            &mut builder,
        )?;
    }
    if let Some(result) = source.result() {
        let first = builder.nodes.len();
        let scope = CanonicalScope {
            outputs: &outputs,
            step_outputs: &step_outputs,
            slot_types: &slot_types,
        };
        let (slot, _) = super::canonical_values::emit(
            &AuthorValue::Mapping(result.fields.clone()),
            true,
            scope,
            &mut builder,
        )?;
        let id = step_idx(builder.nodes.len())?;
        builder.push_node(vb_core::workflow::node(
            id,
            CompiledNodeKind::Finish { result: slot },
            None,
            None,
        ));
        step_names.extend(
            builder
                .nodes
                .iter()
                .skip(first)
                .map(|_| Box::from("$result")),
        );
    }
    super::skipped_outputs::PossiblySkippedOutputs::from_source(source)
        .and_then(|outputs| outputs.validate(&builder))
        .map_err(|error| CompileErrors(vec![error]))?;
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
        .map_err(|e| CompileErrors(vec![named_workflow_error(e.into(), steps, &layout)]))?;
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|e| CompileErrors(vec![named_workflow_error(e.into(), steps, &layout)]))
}

#[path = "input_schema.rs"]
mod input_schema;
use input_schema::{constraint_from_input_schema, kind_from_input_schema};

#[path = "canonical_layout.rs"]
mod layout;
pub(super) use layout::*;

/// Converts a bounded cold author literal into an arena-independent payload.
pub fn author_value_to_owned(
    value: &AuthorValue,
) -> Result<vb_core::InputDefaultValue, CompileError> {
    input_schema::defaults::owned(value)
}
