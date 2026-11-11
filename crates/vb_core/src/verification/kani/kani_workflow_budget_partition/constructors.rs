use crate::ids::{ActionId, SlotIdx, StepIdx};
use crate::workflow::{CompiledNode, CompiledNodeKind, NodeOutput, NodeRouting};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Fields {
    pub(super) output: NodeOutput,
    pub(super) routing: NodeRouting,
}

fn node(position: u16, fields: Fields, kind: CompiledNodeKind) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(position),
        output: fields.output,
        routing: fields.routing,
        kind,
        retry: None,
    }
}

pub(super) fn nop(position: u16, fields: Fields) -> CompiledNode {
    node(position, fields, CompiledNodeKind::Nop)
}

pub(super) fn do_node(
    position: u16,
    fields: Fields,
    action: ActionId,
    input: SlotIdx,
) -> CompiledNode {
    node(position, fields, CompiledNodeKind::Do { action, input })
}

pub(super) fn wait_until(position: u16, fields: Fields, deadline_slot: SlotIdx) -> CompiledNode {
    node(
        position,
        fields,
        CompiledNodeKind::WaitUntil { deadline_slot },
    )
}

pub(super) fn wait_event(
    position: u16,
    fields: Fields,
    event: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) -> CompiledNode {
    node(
        position,
        fields,
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        },
    )
}

pub(super) fn ask(
    position: u16,
    fields: Fields,
    prompt: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) -> CompiledNode {
    node(
        position,
        fields,
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        },
    )
}

pub(super) fn finish(position: u16, fields: Fields, result: SlotIdx) -> CompiledNode {
    node(position, fields, CompiledNodeKind::Finish { result })
}
