//! Allocate declarations before lowering any source expression or scratch state.

use super::{ForEachInputs, SlotCompiler, constraint_from_input_schema, kind_from_input_schema};
use crate::{AuthorValue, CompileError, CompileErrors, WorkflowSource};
use std::collections::HashMap;
use vb_core::{InputSlotKind, SlotIdx};
use vb_expr::expr_typecheck::ExprType;

pub(super) fn prepare_source_declarations(
    source: &WorkflowSource,
    outputs: &mut HashMap<String, SlotIdx>,
    types: &mut HashMap<SlotIdx, ExprType>,
    builder: &mut SlotCompiler,
    for_each_inputs: &mut ForEachInputs,
) -> Result<Vec<Box<str>>, CompileErrors> {
    for input in source.inputs() {
        let kind = kind_from_input_schema(&input.value)?;
        let constraint = constraint_from_input_schema(&input.value)?;
        reject_dangling_schema_source(source, &input.value)?;
        let slot = builder.allocate_slot()?;
        builder.record_input_slot(slot, kind, constraint);
        bind_declaration(outputs, format!("$input.{}", input.key), slot)?;
        types.insert(slot, input_type(kind));
        for_each_inputs.record(slot, &input.key, &input.value);
    }
    for secret in source.secrets() {
        let alias = builder.intern(&secret.key)?;
        let requirement = builder.intern(&secret.value)?;
        let slot = builder.allocate_slot()?;
        builder.record_input_slot(
            slot,
            InputSlotKind::Symbol,
            vb_core::CompiledInputConstraint {
                secret: true,
                default_cold: Some(vb_core::InputDefaultValue::Text(
                    secret.value.clone().into(),
                )),
                ..vb_core::CompiledInputConstraint::required()
            },
        );
        builder.secret_requirements.push(
            vb_core::workflow::source_metadata::CompiledSecretRequirement::with_slot(
                alias,
                requirement,
                slot,
            ),
        );
        bind_declaration(outputs, format!("$secrets.{}", secret.key), slot)?;
        types.insert(slot, ExprType::Text);
    }
    let mut names = Vec::with_capacity(source.vars().len());
    for variable in source.vars() {
        let first = builder.nodes.len();
        let steps = HashMap::new();
        let scope = super::CanonicalScope {
            outputs,
            step_outputs: &steps,
            slot_types: types,
        };
        let (slot, ty) = super::canonical_values::emit(&variable.value, false, scope, builder)?;
        bind_declaration(outputs, format!("$vars.{}", variable.key), slot)?;
        types.insert(slot, ty);
        names.extend(
            builder
                .nodes
                .iter()
                .skip(first)
                .map(|_| format!("$vars.{}", variable.key).into_boxed_str()),
        );
    }
    Ok(names)
}

/// Rejects a `from` schema reference that names no declared input, variable,
/// or secret. Only provably dangling references fail closed here.
fn reject_dangling_schema_source(
    source: &WorkflowSource,
    schema: &AuthorValue,
) -> Result<(), CompileError> {
    let AuthorValue::Mapping(entries) = schema else {
        return Ok(());
    };
    let Some(target) = entries
        .iter()
        .find(|entry| entry.key == "from")
        .map(|entry| &entry.value)
    else {
        return Ok(());
    };
    let AuthorValue::Text(name) = target else {
        return Err(CompileError::InvalidInputSchema {
            field: "inputs.from",
            expected: "a non-empty string",
        });
    };
    if name.is_empty() {
        return Err(CompileError::InvalidInputSchema {
            field: "inputs.from",
            expected: "a non-empty string",
        });
    }
    let declared = source.inputs().iter().any(|field| field.key == *name)
        || source.vars().iter().any(|field| field.key == *name)
        || source.secrets().iter().any(|field| field.key == *name);
    if declared {
        Ok(())
    } else {
        Err(CompileError::InvalidInputSchema {
            field: "inputs.from",
            expected: "a declared input, variable, or secret name",
        })
    }
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
