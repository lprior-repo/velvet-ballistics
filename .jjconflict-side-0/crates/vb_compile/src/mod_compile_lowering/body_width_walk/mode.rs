use super::{Action, CompileError, Frame, StepPrimitive, add_width, unsupported_body};

#[derive(Clone, Copy)]
pub(super) enum WidthMode {
    Legacy,
    Canonical,
}

pub(super) fn primitive_action<'a>(
    primitive: &'a StepPrimitive,
    width: &mut usize,
    mode: WidthMode,
) -> Result<Action<'a>, CompileError> {
    match mode {
        WidthMode::Legacy => legacy_action(primitive, width),
        WidthMode::Canonical => canonical_action(primitive, width),
    }
}

fn legacy_action<'a>(
    primitive: &'a StepPrimitive,
    width: &mut usize,
) -> Result<Action<'a>, CompileError> {
    match primitive {
        StepPrimitive::Set { .. } | StepPrimitive::Do { .. } => {
            add_width(width, 1).map(|()| Action::Idle)
        }
        StepPrimitive::ForEach { body, .. } => Ok(Action::Push(Frame::body(body, 2))),
        StepPrimitive::Collect { body, .. } | StepPrimitive::Repeat { body, .. } => {
            Ok(Action::Push(Frame::body(body, 3)))
        }
        StepPrimitive::Choose { branches, .. } => Ok(Action::Push(Frame::choose(branches))),
        other => unsupported_body(other),
    }
}

fn canonical_action<'a>(
    primitive: &'a StepPrimitive,
    width: &mut usize,
) -> Result<Action<'a>, CompileError> {
    match primitive {
        StepPrimitive::Set { .. }
        | StepPrimitive::Save { .. }
        | StepPrimitive::Do { .. }
        | StepPrimitive::Wait { .. }
        | StepPrimitive::Finish { .. } => add_width(width, 1).map(|()| Action::Idle),
        StepPrimitive::Ask { .. } => add_width(width, 2).map(|()| Action::Idle),
        _ => container_action(primitive, *width),
    }
}

fn container_action(
    primitive: &StepPrimitive,
    parent_width: usize,
) -> Result<Action<'_>, CompileError> {
    match primitive {
        StepPrimitive::ForEach { .. } => for_each_action(primitive, parent_width),
        StepPrimitive::Collect { .. }
        | StepPrimitive::Repeat { .. }
        | StepPrimitive::Aggregate { .. } => loop_action(primitive),
        StepPrimitive::Choose { branches, .. } => Ok(Action::Push(Frame::choose(branches))),
        StepPrimitive::Together { branches } => Ok(Action::Push(Frame::together(branches))),
        other => unsupported_body(other),
    }
}

fn for_each_action(
    primitive: &StepPrimitive,
    parent_width: usize,
) -> Result<Action<'_>, CompileError> {
    let StepPrimitive::ForEach { input, body, .. } = primitive else {
        return unsupported_body(primitive);
    };
    let tracked = crate::mod_compile_lowering::part_14::iteration_body::needs_tracking(body);
    let overhead = 6usize
        .checked_add(crate::mod_compile_lowering::list_input::source_width(input))
        .and_then(|width| width.checked_add(usize::from(tracked)))
        .ok_or(CompileError::StepIndexOutOfRange {
            value: parent_width,
        })?;
    Ok(Action::Push(Frame::tracked_body(body, overhead, tracked)))
}

fn loop_action(primitive: &StepPrimitive) -> Result<Action<'_>, CompileError> {
    let (body, overhead) = match primitive {
        StepPrimitive::Collect { body, .. } => (body, 3),
        StepPrimitive::Repeat { body, .. } => (body, 4),
        StepPrimitive::Aggregate { body, .. } => (
            body,
            if crate::mod_compile_lowering::reduce_direct_body(body) {
                4
            } else {
                5
            },
        ),
        other => return unsupported_body(other),
    };
    Ok(Action::Push(Frame::body(body, overhead)))
}
