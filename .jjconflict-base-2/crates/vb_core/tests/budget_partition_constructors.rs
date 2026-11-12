use vb_core::{ids, workflow};

#[path = "../src/verification/kani/kani_workflow_budget_partition/constructors.rs"]
mod constructors;
#[path = "../src/verification/kani/kani_workflow_budget_partition/domain.rs"]
mod domain;
#[path = "../src/verification/kani/kani_workflow_budget_partition/witness.rs"]
mod witness;
#[path = "../src/verification/kani/kani_workflow_budget_partition/word.rs"]
mod word;

use constructors::Fields;
use ids::{ActionId, SlotIdx, StepIdx};
use vb_core::budget::WholeWorkflowBudget;
use witness::{Kind, Node, Nodes, Witness};
use workflow::{ErrorRouting, NodeContinuation, NodeOutput, NodeRouting, ResourceContract};

fn contract(high: bool) -> ResourceContract {
    use workflow::SecretResultPolicy;
    if high {
        ResourceContract {
            max_steps: u16::MAX,
            max_slots: u16::MAX,
            max_constants: u16::MAX,
            max_accessors: u16::MAX,
            max_expressions: u16::MAX,
            max_expr_stack: u8::MAX,
            max_step_budget_per_tick: u64::MAX,
            max_transitions_per_tick: u64::MAX,
            max_input_bytes: u32::MAX,
            max_output_bytes: u32::MAX,
            max_blob_bytes: u64::MAX,
            max_ipc_payload_bytes: u32::MAX,
            max_retry_attempts: u16::MAX,
            max_fanout: u16::MAX,
            max_collect_items: u32::MAX,
            max_queue_depth: u32::MAX,
            max_journal_batch_bytes: u32::MAX,
            allows_secret_results: SecretResultPolicy::Allow,
        }
    } else {
        ResourceContract {
            max_steps: 0,
            max_slots: 0,
            max_constants: 0,
            max_accessors: 0,
            max_expressions: 0,
            max_expr_stack: 0,
            max_step_budget_per_tick: 0,
            max_transitions_per_tick: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            max_blob_bytes: 0,
            max_ipc_payload_bytes: 0,
            max_retry_attempts: 0,
            max_fanout: 0,
            max_collect_items: 0,
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
            allows_secret_results: SecretResultPolicy::Deny,
        }
    }
}

fn fields(high: bool) -> Fields {
    Fields {
        output: if high {
            NodeOutput::Slot(SlotIdx::new(3))
        } else {
            NodeOutput::Discard
        },
        routing: NodeRouting {
            next: if high {
                NodeContinuation::Step(StepIdx::new(0))
            } else {
                NodeContinuation::End
            },
            error: if high {
                ErrorRouting::Handler {
                    handler: StepIdx::new(0),
                    error_slot: Some(SlotIdx::new(3)),
                }
            } else {
                ErrorRouting::None
            },
        },
    }
}

fn check(witness: Witness) {
    witness.with_nodes(|nodes, entry, resource| {
        assert!(domain::original_projection(nodes, entry));
        assert_eq!(Witness::decode(nodes, entry, resource), Some(witness));
        let result = WholeWorkflowBudget::compute(nodes, entry, resource);
        let expected = match witness.nodes {
            Nodes::Empty => Err(workflow::WorkflowError::EntryOutOfBounds { entry }),
            _ => Ok(()),
        };
        assert_eq!(result.as_ref().map(|_| ()), expected.as_ref().map(|_| ()));
        match result {
            Ok(budget) => {
                assert_eq!(budget.max_total_slots, u64::from(resource.max_slots));
                assert_eq!(budget.max_result_bytes, resource.max_output_bytes);
            }
            Err(error) => {
                assert_eq!(error, workflow::WorkflowError::EntryOutOfBounds { entry });
            }
        }
    });
}

#[test]
fn all_ordered_words_preserve_fields_and_execute_at_contract_extremes() {
    let kinds = [
        Kind::Nop,
        Kind::Do {
            action: ActionId::new(u16::MAX),
            input: SlotIdx::new(3),
        },
        Kind::WaitUntil {
            deadline_slot: SlotIdx::new(3),
        },
        Kind::WaitEvent {
            event: SlotIdx::new(3),
            timeout_slot: Some(SlotIdx::new(0)),
        },
        Kind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None,
        },
        Kind::Finish {
            result: SlotIdx::new(3),
        },
    ];
    for high in [false, true] {
        let resource = contract(high);
        check(Witness {
            nodes: Nodes::Empty,
            entry: StepIdx::new(0),
            contract: resource,
        });
        for kind in kinds {
            let first = Node {
                fields: fields(high),
                kind,
            };
            check(Witness {
                nodes: Nodes::One(first),
                entry: StepIdx::new(0),
                contract: resource,
            });
            for second_kind in kinds {
                let second = Node {
                    fields: fields(!high),
                    kind: second_kind,
                };
                for entry in [StepIdx::new(0), StepIdx::new(1)] {
                    check(Witness {
                        nodes: Nodes::Two(first, second),
                        entry,
                        contract: resource,
                    });
                }
            }
        }
    }
}

#[test]
fn projected_domain_rejects_fields_that_broad_predicate_allows() {
    use workflow::{CompiledNodeKind, CompiledRetryPolicy};
    let node = constructors::nop(0, fields(false));
    assert!(domain::original_projection(
        &[node.clone()],
        StepIdx::new(0)
    ));
    assert!(!domain::original_projection(&[], StepIdx::new(u16::MAX)));
    assert!(!domain::original_projection(
        &[node.clone()],
        StepIdx::new(1)
    ));
    let mut wrong_id = node.clone();
    wrong_id.id = StepIdx::new(1);
    let mut output = node.clone();
    output.output = NodeOutput::Slot(SlotIdx::new(4));
    let mut retry = node.clone();
    retry.retry = Some(CompiledRetryPolicy {
        max_attempts: 0,
        delay_ms: 0,
    });
    let mut next = node.clone();
    next.routing.next = NodeContinuation::Step(StepIdx::new(1));
    let mut handler = node.clone();
    handler.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(1),
        error_slot: None,
    };
    let mut error_slot = node.clone();
    error_slot.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(0),
        error_slot: Some(SlotIdx::new(4)),
    };
    let mut payload = node.clone();
    payload.kind = CompiledNodeKind::Finish {
        result: SlotIdx::new(4),
    };
    for (malformed, expected_word) in [
        (wrong_id, 1),
        (output, 1),
        (retry, 1),
        (next, 1),
        (handler, 1),
        (error_slot, 1),
        (payload, 6),
    ] {
        assert_eq!(
            word::index(std::slice::from_ref(&malformed)),
            Some(expected_word)
        );
        assert!(word::matches(
            std::slice::from_ref(&malformed),
            expected_word
        ));
        assert!(!domain::original_projection(&[malformed], StepIdx::new(0)));
    }
    assert!(!domain::original_projection(
        &[node.clone(), node.clone(), node],
        StepIdx::new(0)
    ));
}
