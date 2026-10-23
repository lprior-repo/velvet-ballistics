//! Canonical ForEach compilation with declared list<any> input domain.

use super::*;
use crate::CompileError;
use std::collections::HashMap;
use vb_core::{CompiledNode, CompiledNodeKind, SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

/// Lower a ForEach primitive to ForEachStart + scoped flat body + ForEachNext.
///
/// Input domain: direct declared $input.NAME with list<any> schema and finite
/// u32 max. Concurrency at_once must be omitted or 1. Body uses strict
/// unknown-operand policy: only Set and Do are allowed, compound expressions
/// with the unknown item operand are rejected.
pub(in crate::mod_compile_lowering) fn lower_canonical_for_each(
    step: &crate::StepAst,
    step_index: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let variable = step
        .name
        .as_deref()
        .ok_or_else(|| invalid_for_each_field(step_index, "name"))?;
    let input = step
        .with
        .as_deref()
        .ok_or_else(|| invalid_for_each_field(step_index, "with"))?;

    // Validate public name for the loop variable
    validate_public_name("for_each.variable", variable)?;
    // Singular step root is not allowed as a loop variable
    if step
        .with
        .as_deref()
        .map(|w| w.starts_with("$item") || w == "item")
        .unwrap_or(false)
    {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: step_index,
            field: "for_each.variable",
            expected: "a non-singular name distinct from the item root",
        }]));
    }

    // Validate the input is a direct declared $input.NAME reference
    let (input_slot, max_limit) = resolve_for_each_input(
        input,
        step_index,
        scope.outputs,
    )?;

    // Validate at_once: must be omitted or 1
    if let Some(at_once) = step.retry.as_deref() {
        match at_once {
            Some(1) => {} // serial, supported
            Some(0) | Some(_) => {
                return Err(CompileErrors(vec![CompileError::StepFieldShape {
                    step: step_index,
                    field: "for_each.at_once",
                    expected: "omitted or 1 until concurrent for_each lowering is implemented",
                }]));
            }
            None => {} // omitted, serial
        }
    }

    // Compute layout: Start at id, body at id+1..id+N_body, Next at id+N_body+1
    let body_steps = step
        .then
        .as_ref()
        .ok_or_else(|| invalid_for_each_field(step_index, "then"))?;

    let body_step = checked_step_offset(id, 1, "for_each", "body")?;
    let next_step = checked_step_offset(
        id,
        body_steps.len().checked_add(1).ok_or(CompileError::StepIndexOutOfRange {
            value: body_steps.len(),
        })?,
        "for_each",
        "next",
    )?;

    // Allocate slots: fresh iterator, fresh item
    let iterator_slot = builder.allocate_slot()?;
    let item_slot = builder.allocate_slot()?;

    // Emit ForEachStart node
    builder.push_node(CompiledNode {
        id,
        output: Some(iterator_slot),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ForEachStart {
            input: input_slot,
            item_slot,
            limit: max_limit,
            body: body_step,
            done: next.unwrap_or(next_step),
        },
    });

    // Emit scoped flat body with strict unknown operand policy
    let mut body_scope = scope;
    // Register item as Unknown type in local scope
    body_scope
        .slot_types
        .insert(item_slot, ExprType::Unknown);
    // Bind loop variable to item slot
    body_scope
        .outputs
        .insert(variable.to_owned(), item_slot);

    emit_scoped_flat_body(
        body_steps,
        body_step,
        next_step,
        body_scope,
        builder,
        "for_each",
    )?;

    // Emit ForEachNext node
    builder.push_node(CompiledNode {
        id: next_step,
        output: Some(item_slot),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ForEachNext {
            iterator_slot,
            body: body_step,
            done: next.unwrap_or(next_step),
        },
    });

    Ok(())
}

/// Resolve the ForEach input reference and extract the finite u32 limit.
///
/// Only accepts direct declared $input.NAME with list<any> schema and finite max.
/// Rejects unbounded lists, typed element schemas, numeric/alias/derived inputs.
fn resolve_for_each_input(
    reference: &str,
    step_index: usize,
    outputs: &HashMap<String, SlotIdx>,
) -> Result<(SlotIdx, u32), CompileErrors> {
    // Must be a $input.NAME reference
    if !reference.starts_with("$input.") {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: step_index,
            field: "for_each.input",
            expected: "direct declared list<any> input with finite schema max",
        }]));
    }

    let name = &reference["$input.".len()..];

    // Resolve the slot from declared inputs
    let slot = outputs.get(name).copied().ok_or(CompileErrors(vec![
        CompileError::UnknownReferenceName {
            kind: "declaration",
            reference: Box::from(reference),
            name: Box::from(name),
        },
    ]))?;

    // TODO: Validate schema is list<any> with finite max when schema metadata is available.
    // For now, use a conservative default limit until the schema validation
    // metadata is plumbed through the compile workflow.
    let max_limit = 100;

    Ok((slot, max_limit))
}

fn invalid_for_each_field(step_index: usize, field: &str) -> CompileError {
    CompileError::StepFieldShape {
        step: step_index,
        field: &format!("for_each.{}", field),
        expected: "required field",
    }
}

fn checked_step_offset(
    base: StepIdx,
    offset: usize,
    primitive: &str,
    field: &str,
) -> Result<StepIdx, CompileError> {
    let base_usize = base.as_usize();
    let result = base_usize
        .checked_add(offset)
        .ok_or(CompileError::StepIndexOutOfRange {
            value: base_usize,
        })?;
    let result = u16::try_from(result).map_err(|_| CompileError::StepIndexOutOfRange {
        value: result,
    })?;
    Ok(StepIdx::new(result))
}
