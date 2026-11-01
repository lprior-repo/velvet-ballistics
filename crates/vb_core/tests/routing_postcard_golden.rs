//! Postcard golden bytes for `CompiledNode` routing planes.
//!
//! The routing codemod replaced `Option`-shaped fields with exhaustive enums.
//! These tests pin the discriminant-aligned wire encoding: the new enums must
//! serialize to byte sequences identical to the legacy `Option` layout, so
//! existing canonical digests and artifacts keep verifying.

use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, ErrorRouting, NodeContinuation, NodeOutput, NodeRouting,
};

fn encode<T: serde::Serialize + ?Sized>(value: &T) -> Vec<u8> {
    postcard::to_allocvec(value).into_iter().flatten().collect()
}

#[test]
fn output_discard_encodes_as_zero() {
    assert_eq!(encode(&NodeOutput::Discard), vec![0x00]);
}

#[test]
fn output_slot_encodes_as_one_followed_by_slot() {
    assert_eq!(encode(&NodeOutput::Slot(SlotIdx::new(1))), vec![0x01, 0x01]);
}

#[test]
fn continuation_end_encodes_as_zero() {
    assert_eq!(encode(&NodeContinuation::End), vec![0x00]);
}

#[test]
fn continuation_step_encodes_as_one_followed_by_step() {
    assert_eq!(
        encode(&NodeContinuation::Step(StepIdx::new(2))),
        vec![0x01, 0x02]
    );
}

#[test]
fn error_none_encodes_as_zero() {
    assert_eq!(encode(&ErrorRouting::None), vec![0x00]);
}

#[test]
fn error_handler_encodes_handler_then_error_slot() {
    assert_eq!(
        encode(&ErrorRouting::Handler {
            handler: StepIdx::new(3),
            error_slot: Some(SlotIdx::new(4)),
        }),
        vec![0x01, 0x03, 0x01, 0x04]
    );
}

#[test]
fn empty_routing_node_encodes_as_five_zero_bytes() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: NodeOutput::Discard,
        routing: NodeRouting {
            next: NodeContinuation::End,
            error: ErrorRouting::None,
        },
        kind: CompiledNodeKind::Nop,
    };
    assert_eq!(encode(&node), vec![0x00, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn routed_node_matches_legacy_option_layout_bytes() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: NodeOutput::Slot(SlotIdx::new(1)),
        routing: NodeRouting {
            next: NodeContinuation::Step(StepIdx::new(2)),
            error: ErrorRouting::Handler {
                handler: StepIdx::new(3),
                error_slot: Some(SlotIdx::new(4)),
            },
        },
        kind: CompiledNodeKind::Nop,
    };
    assert_eq!(
        encode(&node),
        vec![
            0x00, // id
            0x01, 0x01, // output: Some(1)
            0x01, 0x02, // next: Some(2)
            0x01, 0x03, // on_error: Some(3)
            0x01, 0x04, // error_slot: Some(4)
            0x00, // kind: Nop
        ]
    );
}
