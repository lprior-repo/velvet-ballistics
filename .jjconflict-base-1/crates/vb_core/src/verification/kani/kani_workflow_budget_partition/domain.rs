use crate::ids::{SlotIdx, StepIdx};
use crate::workflow::{CompiledNode, CompiledNodeKind};

pub(super) fn original_projection(nodes: &[CompiledNode], entry: StepIdx) -> bool {
    match nodes {
        [] => entry == StepIdx::new(0),
        [first] => entry == StepIdx::new(0) && node(first, 0, 1),
        [first, second] => entry.as_usize() < 2 && node(first, 0, 2) && node(second, 1, 2),
        _ => false,
    }
}

fn node(value: &CompiledNode, position: u16, count: usize) -> bool {
    value.id == StepIdx::new(position)
        && value.retry.is_none()
        && optional_slot(value.output.slot())
        && optional_step(value.routing.next.step(), count)
        && optional_step(value.routing.error.handler(), count)
        && optional_slot(value.routing.error.error_slot())
        && kind(&value.kind)
}

fn optional_step(value: Option<StepIdx>, count: usize) -> bool {
    value.is_none_or(|value| value.as_usize() < count)
}

fn slot(value: SlotIdx) -> bool {
    value.as_usize() <= 3
}

fn optional_slot(value: Option<SlotIdx>) -> bool {
    value.is_none_or(slot)
}

fn kind(value: &CompiledNodeKind) -> bool {
    match *value {
        CompiledNodeKind::Nop => true,
        CompiledNodeKind::Do { input, .. } => slot(input),
        CompiledNodeKind::WaitUntil { deadline_slot } => slot(deadline_slot),
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => slot(event) && optional_slot(timeout_slot.slot()),
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => slot(prompt) && optional_slot(timeout_slot.slot()),
        CompiledNodeKind::Finish { result } => slot(result),
        _ => false,
    }
}
