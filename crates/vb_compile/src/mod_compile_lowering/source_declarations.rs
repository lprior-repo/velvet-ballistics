//! Allocate declarations before lowering any source expression or scratch state.

use super::{ForEachInputs, SlotCompiler, kind_from_input_schema, step_idx};
use crate::{AuthorValue, CompileError, WorkflowSource};
use std::collections::HashMap;
use vb_core::{CompiledNode, CompiledNodeKind, ConstValue, InputSlotKind, SlotIdx};
use vb_expr::expr_typecheck::ExprType;

pub(super) fn prepare_source_declarations(
    source: &WorkflowSource,
    outputs: &mut HashMap<String, SlotIdx>,
    types: &mut HashMap<SlotIdx, ExprType>,
    builder: &mut SlotCompiler,
    for_each_inputs: &mut ForEachInputs,
) -> Result<Vec<Box<str>>, CompileError> {
    for input in source.inputs() {
        let kind = kind_from_input_schema(&input.value)?;
        let slot = builder.allocate_slot()?;
        builder.record_input_slot(slot, kind);
        bind_declaration(outputs, format!("$input.{}", input.key), slot)?;
        types.insert(slot, input_type(kind));
        for_each_inputs.record(slot, &input.key, &input.value);
    }
    let mut names = Vec::with_capacity(source.vars().len());
    for variable in source.vars() {
        let (value, ty) = scalar_variable(&variable.value)?;
        let slot = builder.allocate_slot()?;
        emit_variable(value, slot, builder)?;
        bind_declaration(outputs, format!("$vars.{}", variable.key), slot)?;
        types.insert(slot, ty);
        names.push(format!("$vars.{}", variable.key).into_boxed_str());
    }
    Ok(names)
}

fn bind_declaration(
    outputs: &mut HashMap<String, SlotIdx>,
    name: String,
    slot: SlotIdx,
) -> Result<(), CompileError> {
    if outputs.contains_key(&name) {
        return Err(CompileError::DuplicateOutputName { name: name.into() });
    }
    outputs.insert(name, slot);
    Ok(())
}

fn emit_variable(
    value: ConstValue,
    slot: SlotIdx,
    builder: &mut SlotCompiler,
) -> Result<(), CompileError> {
    let index = builder.nodes.len();
    let next = index
        .checked_add(1)
        .ok_or(CompileError::StepIndexOutOfRange { value: index })?;
    let constant = builder.push_constant(value)?;
    builder.push_node(CompiledNode {
        id: step_idx(index)?,
        output: Some(slot),
        next: Some(step_idx(next)?),
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::SetConst { value: constant },
    });
    Ok(())
}

fn scalar_variable(value: &AuthorValue) -> Result<(ConstValue, ExprType), CompileError> {
    match value {
        AuthorValue::Null => Ok((ConstValue::Null, ExprType::Null)),
        AuthorValue::Bool(value) => Ok((ConstValue::Bool(*value), ExprType::Bool)),
        AuthorValue::I64(value) => Ok((ConstValue::I64(*value), ExprType::I64)),
        _ => Err(CompileError::ExpressionLoweringUnsupported {
            feature: Box::from("vars currently require null, boolean, or integer static literals"),
        }),
    }
}

fn input_type(kind: InputSlotKind) -> ExprType {
    match kind {
        InputSlotKind::Null => ExprType::Null,
        InputSlotKind::Bool => ExprType::Bool,
        InputSlotKind::I64 => ExprType::I64,
        InputSlotKind::F64 => ExprType::F64,
        InputSlotKind::Symbol => ExprType::Text,
        InputSlotKind::List => ExprType::List,
        InputSlotKind::Object => ExprType::Object,
        _ => ExprType::Unknown,
    }
}
