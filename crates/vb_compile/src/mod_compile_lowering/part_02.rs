use super::part_01::ForEachInputs;
use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use std::collections::HashMap;
use vb_core::{CompiledNodeKind, SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

#[cfg(test)]
pub(crate) use super::part_14::lower_canonical_choose;

#[path = "canonical_scope.rs"]
mod canonical_scope;
pub(super) use canonical_scope::{CanonicalScope, validate_output_alias};

#[path = "canonical_references.rs"]
mod canonical_references;
pub(super) use canonical_references::resolve_slot_reference;

#[path = "canonical_types.rs"]
mod canonical_types;

#[path = "canonical_scalar.rs"]
mod canonical_scalar;
pub(super) use canonical_scalar::{
    compile_canonical_expression, lower_canonical_set_expression,
    lower_canonical_set_expression_with_binding,
};

#[path = "canonical_do.rs"]
mod canonical_do;

#[path = "canonical_controls.rs"]
pub(super) mod canonical_controls;

#[path = "canonical_guarded_do.rs"]
mod canonical_guarded_do;
use canonical_guarded_do::lower_top_level_do;

#[path = "bounded_for_each.rs"]
mod bounded_for_each;
use bounded_for_each::{ForEachLowering, lower_bounded_for_each};

#[path = "bounded_collect.rs"]
pub(super) mod bounded_collect;
use bounded_collect::{CollectLowering, lower_bounded_collect};

#[path = "for_each_strict.rs"]
mod for_each_strict;
pub(super) use for_each_strict::validate_for_each_expression;

#[path = "canonical_step.rs"]
mod step;
pub(super) use step::lower_canonical_step;

pub(super) fn extend_step_names_for_generated(
    names: &mut Vec<Box<str>>,
    step_id: &str,
    node_count: usize,
) {
    while names.len() < node_count {
        names.push(Box::from(step_id));
    }
}

pub(super) fn lower_canonical_finish(
    index: usize,
    last: usize,
    id: StepIdx,
    result: &crate::ScalarValue,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    if index != last {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "finish",
            expected: "the last step",
        }]));
    }
    let slot = super::canonical_finish_slot_with_step_outputs(result, outputs, step_outputs)?;
    let node = lower_finish(id, slot, builder);
    builder.push_node(node);
    Ok(())
}
