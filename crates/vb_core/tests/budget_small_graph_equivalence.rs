use proptest::prelude::*;
use vb_core::{
    ActionId, CompiledNode, CompiledNodeKind as K, ResourceContract, SlotIdx, StepIdx,
    WholeWorkflowBudget, WorkflowError,
    workflow::{SecretResultPolicy, node, node_with_handler},
};

#[path = "budget_small_graph_equivalence/retries.rs"]
mod retries;

fn kind(tag: usize) -> K {
    match tag {
        0 => K::Nop,
        1 => K::Do {
            action: ActionId::new(3),
            input: SlotIdx::new(0),
        },
        2 => K::WaitUntil {
            deadline_slot: SlotIdx::new(0),
        },
        3 => K::WaitEvent {
            event: SlotIdx::new(0),
            timeout_slot: Some(SlotIdx::new(1)).into(),
        },
        4 => K::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: Some(SlotIdx::new(1)).into(),
        },
        _ => K::Finish {
            result: SlotIdx::new(0),
        },
    }
}

fn make_node(id: u16, tag: usize, next: Option<u16>, error: Option<u16>) -> CompiledNode {
    match error {
        Some(handler) => node_with_handler(
            StepIdx::new(id),
            kind(tag),
            None,
            next.map(StepIdx::new),
            StepIdx::new(handler),
            Some(SlotIdx::new(2)),
        ),
        None => node(StepIdx::new(id), kind(tag), None, next.map(StepIdx::new)),
    }
}

fn compare(nodes: &[CompiledNode], entry: u16, contract: &ResourceContract) {
    let actual = WholeWorkflowBudget::compute(nodes, StepIdx::new(entry), contract);
    // Every authored edge is None, 0 or 1. These padding nodes are unreachable
    // and force the unchanged production general traversal by exceeding len 2.
    let mut general = nodes.to_vec();
    for id in [1, 2] {
        if general.len() == usize::from(id) {
            general.push(make_node(id, 0, None, None));
        }
    }
    assert_eq!(general.len(), 3);
    let expected = WholeWorkflowBudget::compute(&general, StepIdx::new(entry), contract);
    assert_eq!(
        actual, expected,
        "entry={entry}, nodes={nodes:?}, contract={contract:?}"
    );
}

fn contract_strategy() -> impl Strategy<Value = ResourceContract> {
    (
        any::<[u16; 7]>(),
        any::<[u32; 6]>(),
        any::<[u64; 3]>(),
        any::<u8>(),
        any::<bool>(),
    )
        .prop_map(|(short, bytes, wide, max_expr_stack, allow)| {
            let [
                max_steps,
                max_slots,
                max_constants,
                max_accessors,
                max_expressions,
                max_retry_attempts,
                max_fanout,
            ] = short;
            let [
                max_input_bytes,
                max_output_bytes,
                max_ipc_payload_bytes,
                max_collect_items,
                max_queue_depth,
                max_journal_batch_bytes,
            ] = bytes;
            let [
                max_step_budget_per_tick,
                max_transitions_per_tick,
                max_blob_bytes,
            ] = wide;
            ResourceContract {
                max_steps,
                max_slots,
                max_constants,
                max_accessors,
                max_expressions,
                max_expr_stack,
                max_step_budget_per_tick,
                max_transitions_per_tick,
                max_input_bytes,
                max_output_bytes,
                max_blob_bytes,
                max_ipc_payload_bytes,
                max_retry_attempts,
                max_fanout,
                max_collect_items,
                max_queue_depth,
                max_journal_batch_bytes,
                allows_secret_results: if allow {
                    SecretResultPolicy::Allow
                } else {
                    SecretResultPolicy::Deny
                },
            }
        })
}

fn extreme_contract(maximum: bool) -> ResourceContract {
    let short = if maximum { u16::MAX } else { 0 };
    let bytes = if maximum { u32::MAX } else { 0 };
    let wide = if maximum { u64::MAX } else { 0 };
    ResourceContract {
        max_steps: short,
        max_slots: short,
        max_constants: short,
        max_accessors: short,
        max_expressions: short,
        max_expr_stack: if maximum { u8::MAX } else { 0 },
        max_step_budget_per_tick: wide,
        max_transitions_per_tick: wide,
        max_input_bytes: bytes,
        max_output_bytes: bytes,
        max_blob_bytes: wide,
        max_ipc_payload_bytes: bytes,
        max_retry_attempts: short,
        max_fanout: short,
        max_collect_items: bytes,
        max_queue_depth: bytes,
        max_journal_batch_bytes: bytes,
        allows_secret_results: SecretResultPolicy::Allow,
    }
}

#[test]
fn exhaustive_all_bounded_simple_graphs_match_general_at_resource_extremes() {
    for contract in [
        ResourceContract::DEFAULT,
        extreme_contract(false),
        extreme_contract(true),
    ] {
        assert_eq!(
            WholeWorkflowBudget::compute(&[], StepIdx::new(0), &contract),
            Err(WorkflowError::EntryOutOfBounds {
                entry: StepIdx::new(0)
            })
        );
        let mut cases = 0;
        for first in 0..6 {
            for next in [None, Some(0)] {
                for error in [None, Some(0)] {
                    compare(&[make_node(0, first, next, error)], 0, &contract);
                    cases += 1;
                }
            }
            for second in 0..6 {
                for next0 in [None, Some(0), Some(1)] {
                    for error0 in [None, Some(0), Some(1)] {
                        for next1 in [None, Some(0), Some(1)] {
                            for error1 in [None, Some(0), Some(1)] {
                                let nodes = [
                                    make_node(0, first, next0, error0),
                                    make_node(1, second, next1, error1),
                                ];
                                for entry in [0, 1] {
                                    compare(&nodes, entry, &contract);
                                    cases += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(cases, 5856);
    }
}

#[test]
fn maximum_resource_derived_arithmetic_matches_general_without_wrapping() {
    let contract = extreme_contract(true);
    let nodes = [
        make_node(0, 1, None, Some(1)),
        make_node(1, 1, None, Some(0)),
    ];
    compare(&nodes, 0, &contract);
    let actual = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    let expected = actual.as_ref().map(|budget| {
        (
            budget.max_transitions,
            budget.max_journal_bytes_total,
            budget.max_value_store_entries,
            budget.max_cost_units,
        )
    });
    assert_eq!(expected, Ok((131_072, 8_589_934_590, 131_072, 131_072)));
}

proptest! {
    #[test]
    fn arbitrary_resource_fields_preserve_general_result_and_error(
        contract in contract_strategy(),
        first in 0usize..6, second in 0usize..6,
        next0 in prop::option::of(0u16..2), error0 in prop::option::of(0u16..2),
        next1 in prop::option::of(0u16..2), error1 in prop::option::of(0u16..2),
        retry0 in prop::option::of((any::<u16>(), any::<u64>())),
        retry1 in prop::option::of((any::<u16>(), any::<u64>())),
        entry in 0u16..2,
    ) {
        let mut first = make_node(0, first, next0, error0);
        let mut second = make_node(1, second, next1, error1);
        first.retry = retries::policy(retry0);
        second.retry = retries::policy(retry1);
        compare(&[first, second], entry, &contract);
    }
}
