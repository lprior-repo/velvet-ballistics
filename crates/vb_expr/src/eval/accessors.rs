#![forbid(unsafe_code)]
//! Accessor resolution for the active expression evaluator.

use arrayvec::ArrayVec;
use vb_core::limits::MAX_EXPRESSION_STACK_USIZE;
use vb_core::value_store::ValueStore;
use vb_core::{AccessorIdx, AccessorProgram, PathSegment, SlotValue};

use crate::{AccessorContext, ExprError, ExprResult};

pub(super) fn eval_load_accessor(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    slots: &[Option<SlotValue>],
    accessors: AccessorContext<'_>,
    store: &ValueStore,
    accessor: AccessorIdx,
) -> ExprResult<()> {
    let value = resolve_load_accessor(slots, accessors, store, accessor)?;
    super::stack::push_value(stack, value)
}

fn resolve_load_accessor(
    slots: &[Option<SlotValue>],
    accessors: AccessorContext<'_>,
    store: &ValueStore,
    accessor: AccessorIdx,
) -> ExprResult<SlotValue> {
    let table = match accessors {
        AccessorContext::Present(table) => table,
        AccessorContext::Absent(absence) => {
            return Err(ExprError::MissingAccessorContext { absence });
        }
    };
    let program = table
        .get(accessor.as_usize())
        .ok_or(ExprError::AccessorOutOfBounds { accessor })?;
    resolve_accessor_program(slots, store, program)
}

fn resolve_accessor_program(
    slots: &[Option<SlotValue>],
    store: &ValueStore,
    program: &AccessorProgram,
) -> ExprResult<SlotValue> {
    let depth = program.path.len();
    if depth > vb_core::limits::MAX_PATH_DEPTH {
        return Err(ExprError::AccessorPathTooDeep {
            depth,
            max: vb_core::limits::MAX_PATH_DEPTH,
        });
    }

    let mut current = read_accessor_root(slots, program.root)?;
    for segment in program.path.iter().copied() {
        current = traverse_accessor_segment(store, current, segment)?;
    }
    Ok(current)
}

fn read_accessor_root(
    slots: &[Option<SlotValue>],
    root: vb_core::SlotIdx,
) -> ExprResult<SlotValue> {
    let value = slots
        .get(root.as_usize())
        .copied()
        .ok_or(ExprError::AccessorRootOutOfBounds { root })?;
    value.ok_or(ExprError::AccessorRootUninitialized { root })
}

fn traverse_accessor_segment(
    store: &ValueStore,
    current: SlotValue,
    segment: PathSegment,
) -> ExprResult<SlotValue> {
    match (current, segment) {
        (SlotValue::Object(object), PathSegment::Field(field)) => {
            store.object_field(object, field).map_err(ExprError::from)
        }
        (SlotValue::List(list), PathSegment::Index(index)) => {
            store.list_item(list, index).map_err(ExprError::from)
        }
        (value, segment) => Err(ExprError::UnsupportedAccessorTraversal {
            segment: path_segment_name(segment),
            found: value.type_name(),
        }),
    }
}

const fn path_segment_name(segment: PathSegment) -> &'static str {
    match segment {
        PathSegment::Field(_) => "field",
        PathSegment::Index(_) => "index",
        _ => "unknown",
    }
}
