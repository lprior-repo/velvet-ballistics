use vb_core::{ids, workflow};

#[path = "../src/verification/kani/kani_workflow_budget_partition/node_equality.rs"]
mod node_equality;

use ids::{ActionId, SlotIdx, StepIdx};
use node_equality::focused_nodes_equal;
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

fn variants() -> Vec<CompiledNode> {
    use CompiledNodeKind::{Ask, Do, Finish, Nop, WaitEvent, WaitUntil};
    let zero = SlotIdx::new(0);
    let high = SlotIdx::new(u16::MAX);
    [
        Nop,
        Do {
            action: ActionId::new(0),
            input: zero,
        },
        Do {
            action: ActionId::new(u16::MAX),
            input: zero,
        },
        Do {
            action: ActionId::new(0),
            input: high,
        },
        WaitUntil {
            deadline_slot: zero,
        },
        WaitUntil {
            deadline_slot: high,
        },
        WaitEvent {
            event: zero,
            timeout_slot: None,
        },
        WaitEvent {
            event: high,
            timeout_slot: None,
        },
        WaitEvent {
            event: zero,
            timeout_slot: Some(zero),
        },
        WaitEvent {
            event: zero,
            timeout_slot: Some(high),
        },
        Ask {
            prompt: zero,
            timeout_slot: None,
        },
        Ask {
            prompt: high,
            timeout_slot: None,
        },
        Ask {
            prompt: zero,
            timeout_slot: Some(zero),
        },
        Ask {
            prompt: zero,
            timeout_slot: Some(high),
        },
        Finish { result: zero },
        Finish { result: high },
    ]
    .into_iter()
    .map(node)
    .collect()
}

fn rejects_difference(left: &CompiledNode, right: &CompiledNode) {
    assert_ne!(left, right);
    assert!(!focused_nodes_equal(
        std::slice::from_ref(left),
        std::slice::from_ref(right)
    ));
    assert!(!focused_nodes_equal(
        std::slice::from_ref(right),
        std::slice::from_ref(left)
    ));
}

#[test]
fn every_supported_payload_matches_actual_derived_equality() {
    let variants = variants();
    for left in &variants {
        for right in &variants {
            assert_eq!(
                focused_nodes_equal(std::slice::from_ref(left), std::slice::from_ref(right)),
                left == right,
            );
        }
    }
}

#[test]
fn all_node_fields_and_routing_retry_payloads_remain_observable() {
    let original = node(CompiledNodeKind::Nop);
    let mutations: [fn(&mut CompiledNode); 6] = [
        |n| n.id = StepIdx::new(1),
        |n| n.output = NodeOutput::Slot(SlotIdx::new(0)),
        |n| n.routing.next = NodeContinuation::Step(StepIdx::new(0)),
        |n| {
            n.routing.error = ErrorRouting::Handler {
                handler: StepIdx::new(0),
                error_slot: None,
            }
        },
        |n| {
            n.retry = Some(CompiledRetryPolicy {
                max_attempts: 1,
                delay_ms: 0,
            })
        },
        |n| {
            n.kind = CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            }
        },
    ];
    for mutate in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        rejects_difference(&original, &changed);
    }
    let mut output = original.clone();
    output.output = NodeOutput::Slot(SlotIdx::new(0));
    let mut changed = output.clone();
    changed.output = NodeOutput::Slot(SlotIdx::new(1));
    rejects_difference(&output, &changed);

    let mut routed = original.clone();
    routed.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(0),
        error_slot: Some(SlotIdx::new(0)),
    };
    let mut changed = routed.clone();
    changed.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(1),
        error_slot: Some(SlotIdx::new(0)),
    };
    rejects_difference(&routed, &changed);
    changed.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(0),
        error_slot: Some(SlotIdx::new(1)),
    };
    rejects_difference(&routed, &changed);
    changed.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(0),
        error_slot: None,
    };
    rejects_difference(&routed, &changed);

    let mut retried = original.clone();
    retried.retry = Some(CompiledRetryPolicy {
        max_attempts: 1,
        delay_ms: 0,
    });
    let mut changed = retried.clone();
    changed.retry = Some(CompiledRetryPolicy {
        max_attempts: 2,
        delay_ms: 0,
    });
    rejects_difference(&retried, &changed);
    changed.retry = Some(CompiledRetryPolicy {
        max_attempts: 1,
        delay_ms: 1,
    });
    rejects_difference(&retried, &changed);
}

#[test]
fn length_order_and_unsupported_controls_preserve_the_stronger_predicate() {
    let first = node(CompiledNodeKind::Nop);
    let mut second = first.clone();
    second.id = StepIdx::new(1);
    assert!(focused_nodes_equal(&[], &[]));
    assert!(!focused_nodes_equal(&[], std::slice::from_ref(&first)));
    let forward = [first.clone(), second.clone()];
    let reverse = [second, first.clone()];
    assert!(focused_nodes_equal(&forward, &forward));
    assert!(!focused_nodes_equal(&forward, &reverse));
    assert!(!focused_nodes_equal(std::slice::from_ref(&first), &forward));
    let three = [first.clone(), first.clone(), first];
    assert!(!focused_nodes_equal(&three, &three));
    let unsupported = node(CompiledNodeKind::Copy {
        source: SlotIdx::new(0),
    });
    assert_eq!(unsupported, unsupported.clone());
    assert!(!focused_nodes_equal(
        std::slice::from_ref(&unsupported),
        std::slice::from_ref(&unsupported)
    ));
}
