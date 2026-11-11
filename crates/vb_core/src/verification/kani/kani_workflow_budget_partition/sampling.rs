use crate::ids::StepIdx;
use crate::workflow::{NodeContinuation, NodeOutput, NodeRouting};

use super::super::{
    bounded_error_routing, bounded_slot, bounded_two_node_step, maybe_slot, maybe_step,
};
use super::constructors::Fields;
use super::witness::{Kind, Node, Nodes, Witness};

pub(super) fn fields(count: usize) -> Fields {
    Fields {
        output: NodeOutput::from(maybe_slot()),
        routing: NodeRouting {
            next: NodeContinuation::from(maybe_step(count)),
            error: bounded_error_routing(count),
        },
    }
}

#[derive(Clone, Copy)]
enum Tag {
    Nop,
    Do,
    WaitUntil,
    WaitEvent,
    Ask,
    Finish,
}

fn node(tag: Tag, count: usize) -> Node {
    let fields = fields(count);
    let kind = match tag {
        Tag::Nop => Kind::Nop,
        Tag::Do => Kind::Do {
            action: crate::ids::ActionId::new(kani::any()),
            input: bounded_slot(),
        },
        Tag::WaitUntil => Kind::WaitUntil {
            deadline_slot: bounded_slot(),
        },
        Tag::WaitEvent => Kind::WaitEvent {
            event: bounded_slot(),
            timeout_slot: maybe_slot(),
        },
        Tag::Ask => Kind::Ask {
            prompt: bounded_slot(),
            timeout_slot: maybe_slot(),
        },
        Tag::Finish => Kind::Finish {
            result: bounded_slot(),
        },
    };
    Node { fields, kind }
}

fn one(tag: Tag) -> Witness {
    Witness {
        nodes: Nodes::One(node(tag, 1)),
        entry: StepIdx::new(0),
        contract: kani::any(),
    }
}

fn two(first: Tag, second: Tag) -> Witness {
    Witness {
        nodes: Nodes::Two(node(first, 2), node(second, 2)),
        entry: bounded_two_node_step(),
        contract: kani::any(),
    }
}

pub(super) fn case_witness(case: u8) -> Witness {
    match case {
        0 => Witness {
            nodes: Nodes::Empty,
            entry: StepIdx::new(0),
            contract: kani::any(),
        },
        1 => one(Tag::Nop),
        2 => one(Tag::Do),
        3 => one(Tag::WaitUntil),
        4 => one(Tag::WaitEvent),
        5 => one(Tag::Ask),
        6 => one(Tag::Finish),
        7 => two(Tag::Nop, Tag::Nop),
        8 => two(Tag::Nop, Tag::Do),
        9 => two(Tag::Nop, Tag::WaitUntil),
        10 => two(Tag::Nop, Tag::WaitEvent),
        11 => two(Tag::Nop, Tag::Ask),
        12 => two(Tag::Nop, Tag::Finish),
        13 => two(Tag::Do, Tag::Nop),
        14 => two(Tag::Do, Tag::Do),
        15 => two(Tag::Do, Tag::WaitUntil),
        16 => two(Tag::Do, Tag::WaitEvent),
        17 => two(Tag::Do, Tag::Ask),
        18 => two(Tag::Do, Tag::Finish),
        19 => two(Tag::WaitUntil, Tag::Nop),
        20 => two(Tag::WaitUntil, Tag::Do),
        21 => two(Tag::WaitUntil, Tag::WaitUntil),
        22 => two(Tag::WaitUntil, Tag::WaitEvent),
        23 => two(Tag::WaitUntil, Tag::Ask),
        24 => two(Tag::WaitUntil, Tag::Finish),
        25 => two(Tag::WaitEvent, Tag::Nop),
        26 => two(Tag::WaitEvent, Tag::Do),
        27 => two(Tag::WaitEvent, Tag::WaitUntil),
        28 => two(Tag::WaitEvent, Tag::WaitEvent),
        29 => two(Tag::WaitEvent, Tag::Ask),
        30 => two(Tag::WaitEvent, Tag::Finish),
        31 => two(Tag::Ask, Tag::Nop),
        32 => two(Tag::Ask, Tag::Do),
        33 => two(Tag::Ask, Tag::WaitUntil),
        34 => two(Tag::Ask, Tag::WaitEvent),
        35 => two(Tag::Ask, Tag::Ask),
        36 => two(Tag::Ask, Tag::Finish),
        37 => two(Tag::Finish, Tag::Nop),
        38 => two(Tag::Finish, Tag::Do),
        39 => two(Tag::Finish, Tag::WaitUntil),
        40 => two(Tag::Finish, Tag::WaitEvent),
        41 => two(Tag::Finish, Tag::Ask),
        _ => two(Tag::Finish, Tag::Finish),
    }
}
