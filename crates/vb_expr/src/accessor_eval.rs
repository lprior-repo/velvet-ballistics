#![forbid(unsafe_code)]
//! Shared accessor resolution for expression evaluators.

use vb_core::value_store::ValueStore;
use vb_core::{AccessorIdx, AccessorProgram, PathSegment, SlotValue};

use crate::{ExprError, ExprResult};

pub(crate) fn eval_load_accessor_from_slots(
    slots: &[Option<SlotValue>],
    accessors: &[AccessorProgram],
    store: &ValueStore,
    accessor: AccessorIdx,
) -> ExprResult<SlotValue> {
    let program = accessors
        .get(accessor.as_usize())
        .ok_or_else(|| invalid_accessor_reference(accessor))?;
    eval_accessor_program(slots, store, program)
}

fn eval_accessor_program(
    slots: &[Option<SlotValue>],
    store: &ValueStore,
    program: &AccessorProgram,
) -> ExprResult<SlotValue> {
    let mut current = slots
        .get(program.root.as_usize())
        .and_then(|slot| *slot)
        .ok_or(ExprError::StackUnderflow)?;

    let mut index = 0usize;
    while index < program.path.len() {
        let segment = program
            .path
            .get(index)
            .copied()
            .ok_or(ExprError::UnexpectedEof)?;
        current = traverse_accessor_segment(store, current, segment)?;
        index = next_index(index)?;
    }

    Ok(current)
}

fn traverse_accessor_segment(
    store: &ValueStore,
    current: SlotValue,
    segment: PathSegment,
) -> ExprResult<SlotValue> {
    match (current, segment) {
        (SlotValue::Object(object), PathSegment::Field(field)) => store
            .object_field(object, field)
            .map_err(|_| invalid_path_segment(segment)),
        (SlotValue::List(list), PathSegment::Index(index)) => store
            .list_item(list, index)
            .map_err(|_| invalid_path_segment(segment)),
        (value, segment) => Err(ExprError::TypeMismatch {
            expected: segment_expected_type(segment).into(),
            found: value.type_name().into(),
        }),
    }
}

fn next_index(index: usize) -> ExprResult<usize> {
    index.checked_add(1).ok_or(ExprError::UnexpectedEof)
}

fn invalid_accessor_reference(accessor: AccessorIdx) -> ExprError {
    ExprError::InvalidReference {
        reference: format!("accessor:{accessor:?}"),
    }
}

fn invalid_path_segment(segment: PathSegment) -> ExprError {
    ExprError::InvalidReference {
        reference: format!("accessor segment {segment:?}"),
    }
}

const fn segment_expected_type(segment: PathSegment) -> &'static str {
    match segment {
        PathSegment::Field(_) => "object",
        PathSegment::Index(_) => "list",
        _ => "supported accessor target",
    }
}
