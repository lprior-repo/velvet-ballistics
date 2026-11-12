use crate::ids::{ActionId, SlotIdx, StepIdx};
use crate::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};

use super::constructors::{self, Fields};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Nop,
    Do {
        action: ActionId,
        input: SlotIdx,
    },
    WaitUntil {
        deadline_slot: SlotIdx,
    },
    WaitEvent {
        event: SlotIdx,
        timeout_slot: Option<SlotIdx>,
    },
    Ask {
        prompt: SlotIdx,
        timeout_slot: Option<SlotIdx>,
    },
    Finish {
        result: SlotIdx,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Node {
    pub(super) fields: Fields,
    pub(super) kind: Kind,
}

impl Node {
    pub(super) fn encode(self, position: u16) -> CompiledNode {
        let fields = self.fields;
        match self.kind {
            Kind::Nop => constructors::nop(position, fields),
            Kind::Do { action, input } => constructors::do_node(position, fields, action, input),
            Kind::WaitUntil { deadline_slot } => {
                constructors::wait_until(position, fields, deadline_slot)
            }
            Kind::WaitEvent {
                event,
                timeout_slot,
            } => constructors::wait_event(position, fields, event, timeout_slot),
            Kind::Ask {
                prompt,
                timeout_slot,
            } => constructors::ask(position, fields, prompt, timeout_slot),
            Kind::Finish { result } => constructors::finish(position, fields, result),
        }
    }

    fn decode(node: &CompiledNode) -> Option<Self> {
        let kind = match node.kind {
            CompiledNodeKind::Nop => Kind::Nop,
            CompiledNodeKind::Do { action, input } => Kind::Do { action, input },
            CompiledNodeKind::WaitUntil { deadline_slot } => Kind::WaitUntil { deadline_slot },
            CompiledNodeKind::WaitEvent {
                event,
                timeout_slot,
            } => Kind::WaitEvent {
                event,
                timeout_slot: timeout_slot.slot(),
            },
            CompiledNodeKind::Ask {
                prompt,
                timeout_slot,
            } => Kind::Ask {
                prompt,
                timeout_slot: timeout_slot.slot(),
            },
            CompiledNodeKind::Finish { result } => Kind::Finish { result },
            _ => return None,
        };
        Some(Self {
            fields: Fields {
                output: node.output,
                routing: node.routing,
            },
            kind,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Nodes {
    Empty,
    One(Node),
    Two(Node, Node),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Witness {
    pub(super) nodes: Nodes,
    pub(super) entry: StepIdx,
    pub(super) contract: ResourceContract,
}

impl Witness {
    pub(super) fn decode(
        nodes: &[CompiledNode],
        entry: StepIdx,
        contract: &ResourceContract,
    ) -> Option<Self> {
        let nodes = match nodes {
            [] => Nodes::Empty,
            [first] => Nodes::One(Node::decode(first)?),
            [first, second] => Nodes::Two(Node::decode(first)?, Node::decode(second)?),
            _ => return None,
        };
        Some(Self {
            nodes,
            entry,
            contract: *contract,
        })
    }

    pub(super) fn with_nodes(
        self,
        visit: impl FnOnce(&[CompiledNode], StepIdx, &ResourceContract),
    ) {
        let entry = self.entry;
        let contract = self.contract;
        match self.nodes {
            Nodes::Empty => visit(&[], entry, &contract),
            Nodes::One(first) => visit(&[first.encode(0)], entry, &contract),
            Nodes::Two(first, second) => {
                visit(&[first.encode(0), second.encode(1)], entry, &contract)
            }
        }
    }
}
