use vb_core::{ids, workflow};

#[path = "../src/verification/kani/kani_workflow_budget_partition/word.rs"]
mod word;

use ids::{ActionId, SlotIdx, StepIdx};
use workflow::{
    CompiledNode, CompiledNodeKind, CompiledRetryPolicy, ErrorRouting, NodeContinuation,
    NodeOutput, NodeRouting,
};

fn node(kind: CompiledNodeKind) -> CompiledNode {
    CompiledNode {
        id: StepIdx::ZERO,
        output: NodeOutput::Discard,
        routing: NodeRouting {
            next: NodeContinuation::End,
            error: ErrorRouting::None,
        },
        kind,
        retry: None,
    }
}

fn check(nodes: &[CompiledNode], expected: u8) {
    assert_eq!(word::index(nodes), Some(expected));
    for candidate in u8::MIN..=u8::MAX {
        assert_eq!(word::matches(nodes, candidate), candidate == expected);
    }
}

#[test]
fn all_ordered_words_have_exactly_one_partition() -> Result<(), String> {
    let zero = SlotIdx::new(0);
    let kinds = [
        CompiledNodeKind::Nop,
        CompiledNodeKind::Do {
            action: ActionId::new(u16::MAX),
            input: zero,
        },
        CompiledNodeKind::WaitUntil {
            deadline_slot: zero,
        },
        CompiledNodeKind::WaitEvent {
            event: zero,
            timeout_slot: Some(zero),
        },
        CompiledNodeKind::Ask {
            prompt: zero,
            timeout_slot: None,
        },
        CompiledNodeKind::Finish { result: zero },
    ];
    let mut expected = 0_u8;
    check(&[], expected);
    for kind in &kinds {
        expected = expected.checked_add(1).ok_or("word index")?;
        check(&[node(kind.clone())], expected);
    }
    for left in &kinds {
        for right in &kinds {
            expected = expected.checked_add(1).ok_or("word index")?;
            check(&[node(left.clone()), node(right.clone())], expected);
        }
    }
    assert_eq!(expected, 42);
    Ok(())
}

#[test]
fn membership_does_not_filter_invalid_non_shape_fields() {
    let mut malformed = node(CompiledNodeKind::Nop);
    malformed.id = StepIdx::new(u16::MAX);
    malformed.output = NodeOutput::Slot(SlotIdx::new(u16::MAX));
    malformed.routing = NodeRouting {
        next: NodeContinuation::Step(StepIdx::new(u16::MAX)),
        error: ErrorRouting::Handler {
            handler: StepIdx::new(u16::MAX),
            error_slot: Some(SlotIdx::new(u16::MAX)),
        },
    };
    malformed.retry = Some(CompiledRetryPolicy {
        max_attempts: 0,
        delay_ms: u64::MAX,
    });
    check(&[malformed], 1);
}

#[test]
fn unsupported_kinds_and_extra_nodes_are_unclassified() {
    let unsupported = [node(CompiledNodeKind::Copy {
        source: SlotIdx::new(0),
    })];
    let extra = [
        node(CompiledNodeKind::Nop),
        node(CompiledNodeKind::Nop),
        node(CompiledNodeKind::Nop),
    ];
    for nodes in [unsupported.as_slice(), extra.as_slice()] {
        assert_eq!(word::index(nodes), None);
        for candidate in u8::MIN..=u8::MAX {
            assert!(!word::matches(nodes, candidate));
        }
    }
}
