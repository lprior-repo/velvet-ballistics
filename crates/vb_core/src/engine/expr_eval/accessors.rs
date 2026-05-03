//! Accessor evaluation.

use crate::errors::EngineError;
use crate::ids::AccessorIdx;
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::{AccessorProgram, CompiledWorkflow};

fn eval_accessor_program_without_store(
    run: &crate::RunFrame,
    program: &AccessorProgram,
) -> Result<SlotValue, EngineError> {
    let current = *run.read_slot(program.root)?;
    if program.path.is_empty() {
        return Ok(current);
    }
    let segment = program.path.first().copied().ok_or({
        EngineError::InternalInvariantViolation {
            reason: "accessor path checked non-empty",
        }
    })?;
    Err(EngineError::UnsupportedAccessorTraversal {
        segment: path_segment_name(segment),
        found: current.type_name(),
    })
}

pub(super) fn eval_accessor_program(
    run: &crate::RunFrame,
    store: &mut ValueStore,
    program: &AccessorProgram,
) -> Result<SlotValue, EngineError> {
    let mut current = *run.read_slot(program.root)?;
    if program.path.is_empty() {
        return Ok(current);
    }

    let mut index = 0usize;
    while index < program.path.len() {
        let segment = program.path.get(index).copied().ok_or({
            EngineError::InternalInvariantViolation {
                reason: "accessor path index checked by loop bound",
            }
        })?;
        current = traverse_accessor_segment(store, current, segment)?;
        index = index
            .checked_add(1)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "accessor path index overflow",
            })?;
    }
    Ok(current)
}

fn traverse_accessor_segment(
    store: &ValueStore,
    current: SlotValue,
    segment: crate::workflow::PathSegment,
) -> Result<SlotValue, EngineError> {
    match (current, segment) {
        (SlotValue::Object(object), crate::workflow::PathSegment::Field(field)) => {
            store.object_field(object, field)
        }
        (SlotValue::List(list), crate::workflow::PathSegment::Index(index)) => {
            store.list_item(list, index)
        }
        (value, segment) => Err(EngineError::UnsupportedAccessorTraversal {
            segment: path_segment_name(segment),
            found: value.type_name(),
        }),
    }
}

const fn path_segment_name(segment: crate::workflow::PathSegment) -> &'static str {
    match segment {
        crate::workflow::PathSegment::Field(_) => "field",
        crate::workflow::PathSegment::Index(_) => "index",
    }
}

// ===== Public API =====

pub(super) fn eval_accessor_inner(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    accessor: AccessorIdx,
) -> Result<SlotValue, EngineError> {
    let program = plan
        .accessor(accessor)
        .ok_or(EngineError::InvalidCompiledWorkflow {
            reason: "accessor index out of bounds",
        })?;
    eval_accessor_program(run, store, program)
}

pub(super) fn eval_accessor_with_taint_inner(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    accessor: AccessorIdx,
) -> Result<(SlotValue, crate::value::Taint), EngineError> {
    let program = plan
        .accessor(accessor)
        .ok_or(EngineError::InvalidCompiledWorkflow {
            reason: "accessor index out of bounds",
        })?;
    let root_taint = run.read_taint(program.root)?;
    let value = eval_accessor_program(run, store, program)?;
    Ok((value, root_taint))
}

pub(super) fn eval_load_accessor(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    stack: &mut super::stack::ExprStack,
    accessor: AccessorIdx,
    taint_accum: &mut crate::value::Taint,
) -> Result<(), EngineError> {
    let (value, accessor_taint) = eval_accessor_with_taint_inner(plan, run, store, accessor)?;
    *taint_accum = crate::value::join_taint(*taint_accum, accessor_taint);
    super::stack::push_value(stack, value)
}

pub fn eval_accessor(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    accessor: AccessorIdx,
) -> Result<SlotValue, EngineError> {
    let program = plan
        .accessor(accessor)
        .ok_or(EngineError::InvalidCompiledWorkflow {
            reason: "accessor index out of bounds",
        })?;
    eval_accessor_program_without_store(run, program)
}

pub fn eval_accessor_with_store(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    accessor: AccessorIdx,
) -> Result<SlotValue, EngineError> {
    eval_accessor_inner(plan, run, store, accessor)
}
