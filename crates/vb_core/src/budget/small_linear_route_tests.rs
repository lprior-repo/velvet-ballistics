use super::super::{
    depth::compute_structural_depth, loop_body::count_total_steps,
    reachable::compute_reachable_metrics, successors::bounded_tracking_vec,
};
use super::*;
use crate::ids::{ActionId, SlotIdx};
use crate::workflow::{CompiledNode, CompiledRetryPolicy, ErrorRouting};

const IDS: [u16; 5] = [0, 1, 2, 65534, u16::MAX];
const ROUTES: [Option<u16>; 6] = [None, Some(0), Some(1), Some(2), Some(65534), Some(u16::MAX)];

fn kinds() -> [CompiledNodeKind; 6] {
    [
        CompiledNodeKind::Nop,
        CompiledNodeKind::Do {
            action: ActionId::new(u16::MAX),
            input: SlotIdx::new(u16::MAX),
        },
        CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(u16::MAX),
        },
        CompiledNodeKind::WaitEvent {
            event: SlotIdx::new(u16::MAX),
            timeout_slot: Some(SlotIdx::new(0)),
        },
        CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: Some(SlotIdx::new(u16::MAX)),
        },
        CompiledNodeKind::Finish {
            result: SlotIdx::new(u16::MAX),
        },
    ]
}

fn node(id: u16, kind: CompiledNodeKind, next: Option<u16>, error: Option<u16>) -> CompiledNode {
    let mut node = crate::workflow::node(
        StepIdx::new(id),
        kind,
        Some(SlotIdx::new(u16::MAX)),
        next.map(StepIdx::new),
    );
    node.routing.error = error.map_or(ErrorRouting::None, |handler| ErrorRouting::Handler {
        handler: StepIdx::new(handler),
        error_slot: Some(SlotIdx::new(u16::MAX)),
    });
    node
}

// Reference assembly uses the general traversal helpers and shared dimension calculation.
impl WholeWorkflowBudget {
    fn general_budget_for_tiny_test(
        nodes: &[CompiledNode],
        entry: StepIdx,
        contract: &ResourceContract,
    ) -> Result<Self, BudgetTraversalError> {
        let node_count = nodes.len();
        if entry.as_usize() >= node_count {
            return Err(BudgetTraversalError::EntryOutOfBounds { entry });
        }
        {
            let mut visited: Vec<bool> = vec![false; node_count];
            let mut in_path: Vec<u16> = bounded_tracking_vec(node_count);
            let max_total_steps = count_total_steps(nodes, entry, node_count)?;
            u32::try_from(max_total_steps).map_err(|_| {
                BudgetTraversalError::StepCountOverflow {
                    actual: max_total_steps,
                }
            })?;

            let mut max_fanout: u16 = 0;
            let mut max_action_tickets: u32 = 0;
            let mut max_gather_pages: u32 = 0;
            let mut max_gather_items: u32 = 0;
            let mut max_for_each_iterations: u32 = 0;
            let mut max_together_branches: u16 = 0;
            let mut max_repeat_attempts: u16 = 0;
            let mut max_timer_entries: u32 = 0;
            let mut max_open_gates: u32 = 0;
            let mut max_signal_subscriptions: u32 = 0;
            compute_reachable_metrics(
                nodes,
                entry,
                &mut visited,
                &mut in_path,
                node_count,
                &mut max_fanout,
                &mut max_action_tickets,
                &mut max_gather_pages,
                &mut max_gather_items,
                &mut max_for_each_iterations,
                &mut max_together_branches,
                &mut max_repeat_attempts,
                &mut max_timer_entries,
                &mut max_open_gates,
                &mut max_signal_subscriptions,
            )?;

            let peak = super::super::parallel::compute_peak(nodes.iter().enumerate().filter_map(
                |(index, node)| (visited.get(index).copied() == Some(true)).then_some(node),
            ))?;
            let max_parallel_in_flight = u16::try_from(peak)
                .map_err(|_| BudgetTraversalError::StepCountOverflow { actual: peak })?;
            let max_nesting_depth = compute_structural_depth(nodes, &visited)?;
            let max_total_slots = u64::from(contract.max_slots);

            // Phase 0 executes at most one step per runtime tick, so steps bound time.
            let max_run_time_seconds = max_total_steps;

            let derived = derive_budget_dimensions(max_total_steps, max_action_tickets, contract)?;

            Ok(Self {
                max_total_steps,
                max_total_slots,
                max_fanout,
                max_nesting_depth,
                max_steps_executable: Self::MAX_STEPS_EXECUTABLE,
                max_action_tickets,
                max_parallel_in_flight,
                max_retries_per_action: contract.max_retry_attempts,
                max_gather_pages,
                max_gather_items,
                max_for_each_iterations,
                max_together_branches,
                max_repeat_attempts,
                max_run_time_seconds,
                max_result_bytes: contract.max_output_bytes,
                max_total_slots_written: u32::from(contract.max_slots),
                max_timer_entries,
                max_trace_events: max_total_steps,
                max_journal_batch_bytes: contract.max_journal_batch_bytes,
                max_queue_depth: contract.max_queue_depth,
                max_ipc_payload_bytes: contract.max_ipc_payload_bytes,
                max_blob_bytes: contract.max_blob_bytes,
                max_input_bytes: contract.max_input_bytes,
                max_open_gates,
                max_signal_subscriptions,
                max_incident_emissions: derived.max_incident_emissions,
                max_transitions: derived.max_transitions,
                max_journal_bytes_total: derived.max_journal_bytes_total,
                max_value_store_entries: derived.max_value_store_entries,
                max_cost_units: derived.max_cost_units,
            })
        }
    }
}

fn compare(nodes: &[CompiledNode], entry: u16, contract: &ResourceContract) {
    let entry = StepIdx::new(entry);
    let expected = WholeWorkflowBudget::general_budget_for_tiny_test(nodes, entry, contract);
    let actual = WholeWorkflowBudget::compute_budget_local(nodes, entry, contract);
    assert_eq!(
        actual, expected,
        "entry={entry:?}, nodes={nodes:?}, contract={contract:?}"
    );
}

#[test]
fn boundary_matrix_matches_complete_general_budget_and_errors()
-> Result<(), Box<dyn std::error::Error>> {
    let contract = ResourceContract::DEFAULT;
    let mut cases = 0_u64;
    for first_kind in kinds() {
        for &first_id in &IDS {
            for &first_next in &ROUTES {
                for &first_error in &ROUTES {
                    let first = node(first_id, first_kind.clone(), first_next, first_error);
                    compare(&[first.clone()], 0, &contract);
                    cases = cases.checked_add(1).ok_or("case count")?;
                    for second_kind in kinds() {
                        for &second_id in &IDS {
                            for &second_next in &ROUTES {
                                for &second_error in &ROUTES {
                                    let second = node(
                                        second_id,
                                        second_kind.clone(),
                                        second_next,
                                        second_error,
                                    );
                                    for entry in [0, 1] {
                                        compare(&[first.clone(), second.clone()], entry, &contract);
                                        cases = cases.checked_add(1).ok_or("case count")?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 2_333_880);
    Ok(())
}

#[test]
fn tiny_path_accepts_raw_ids_and_routes_with_general_error_precedence()
-> Result<(), Box<dyn std::error::Error>> {
    let contract = ResourceContract::DEFAULT;
    let graphs = [
        vec![node(0, CompiledNodeKind::Nop, Some(2), Some(u16::MAX))],
        vec![
            node(0, CompiledNodeKind::Nop, Some(u16::MAX), Some(1)),
            node(1, CompiledNodeKind::Nop, Some(u16::MAX), Some(2)),
        ],
        vec![
            node(0, CompiledNodeKind::Nop, Some(2), Some(1)),
            node(1, CompiledNodeKind::Nop, Some(u16::MAX), None),
        ],
        vec![
            node(0, CompiledNodeKind::Nop, None, None),
            node(
                1,
                CompiledNodeKind::Do {
                    action: ActionId::new(7),
                    input: SlotIdx::new(0),
                },
                Some(2),
                Some(u16::MAX),
            ),
        ],
        vec![
            node(2, CompiledNodeKind::Nop, Some(u16::MAX), Some(u16::MAX)),
            node(
                0,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![
            node(1, CompiledNodeKind::Nop, Some(1), Some(0)),
            node(
                1,
                CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(0),
                    timeout_slot: None,
                },
                Some(0),
                Some(1),
            ),
        ],
    ];
    for graph in &graphs {
        let expected =
            WholeWorkflowBudget::general_budget_for_tiny_test(graph, StepIdx::new(0), &contract)
                .map(Some);
        assert_eq!(
            compute_small_linear_budget(graph, StepIdx::new(0), &contract),
            expected,
            "{graph:?}"
        );
    }
    for (graph, target) in graphs.iter().zip([u16::MAX, 2, u16::MAX]) {
        assert_eq!(
            WholeWorkflowBudget::compute_budget_local(graph, StepIdx::new(0), &contract),
            Err(BudgetTraversalError::StepOutOfBounds {
                step: StepIdx::new(target)
            }),
        );
    }
    Ok(())
}

#[test]
fn entry_bounds_and_unsupported_shapes_retain_existing_dispatch()
-> Result<(), Box<dyn std::error::Error>> {
    let contract = ResourceContract::DEFAULT;
    for entry in IDS {
        assert_eq!(
            WholeWorkflowBudget::compute_budget_local(&[], StepIdx::new(entry), &contract),
            Err(BudgetTraversalError::EntryOutOfBounds {
                entry: StepIdx::new(entry)
            }),
        );
        for count in [1, 2] {
            let nodes = [
                node(u16::MAX, CompiledNodeKind::Nop, None, None),
                node(0, CompiledNodeKind::Nop, None, None),
            ];
            let slice = nodes.get(..count).ok_or("node range")?;
            if usize::from(entry) >= count {
                compare(slice, entry, &contract);
                assert_eq!(
                    WholeWorkflowBudget::compute_budget_local(
                        slice,
                        StepIdx::new(entry),
                        &contract
                    ),
                    Err(BudgetTraversalError::EntryOutOfBounds {
                        entry: StepIdx::new(entry)
                    }),
                );
            }
        }
    }
    let unsupported = [node(
        0,
        CompiledNodeKind::Copy {
            source: SlotIdx::new(0),
        },
        None,
        None,
    )];
    assert_eq!(
        compute_small_linear_budget(&unsupported, StepIdx::new(0), &contract),
        Ok(None)
    );
    let three = [
        node(0, CompiledNodeKind::Nop, None, None),
        node(1, CompiledNodeKind::Nop, None, None),
        node(2, CompiledNodeKind::Nop, None, None),
    ];
    assert_eq!(
        compute_small_linear_budget(&three, StepIdx::new(0), &contract),
        Ok(None)
    );
    Ok(())
}

#[test]
fn raw_lookup_retry_and_contract_extremes_match_general() {
    let contracts = [
        ResourceContract::DEFAULT,
        ResourceContract {
            max_slots: 0,
            max_constants: 0,
            max_retry_attempts: 0,
            max_output_bytes: 0,
            max_journal_batch_bytes: 0,
            ..ResourceContract::DEFAULT
        },
        ResourceContract {
            max_slots: u16::MAX,
            max_constants: u16::MAX,
            max_retry_attempts: u16::MAX,
            max_output_bytes: u32::MAX,
            max_blob_bytes: u64::MAX,
            max_journal_batch_bytes: u32::MAX,
            ..ResourceContract::DEFAULT
        },
    ];
    for first in kinds() {
        for second in kinds() {
            for retry in [
                None,
                Some(CompiledRetryPolicy {
                    max_attempts: 1,
                    delay_ms: 0,
                }),
                Some(CompiledRetryPolicy {
                    max_attempts: u16::MAX,
                    delay_ms: u64::MAX,
                }),
            ] {
                let mut nodes = [
                    node(1, first.clone(), Some(0), Some(65534)),
                    node(65534, second.clone(), Some(1), Some(65534)),
                ];
                for node in &mut nodes {
                    node.retry = retry;
                }
                for contract in &contracts {
                    for entry in [0, 1] {
                        compare(&nodes, entry, contract);
                    }
                }
            }
        }
    }
}
