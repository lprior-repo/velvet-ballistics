use vb_core::{
    ActionId, CompiledNode, CompiledNodeKind as K, ResourceContract, SlotIdx, StepIdx,
    WholeWorkflowBudget, WorkflowError,
    workflow::{node, node_with_handler},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn simple(id: u16, kind: K, next: Option<u16>, handler: Option<u16>) -> CompiledNode {
    match handler {
        Some(handler) => node_with_handler(
            StepIdx::new(id),
            kind,
            None,
            next.map(StepIdx::new),
            StepIdx::new(handler),
            None,
        ),
        None => node(StepIdx::new(id), kind, None, next.map(StepIdx::new)),
    }
}

fn compute(nodes: &[CompiledNode], entry: u16) -> Result<WholeWorkflowBudget, WorkflowError> {
    WholeWorkflowBudget::compute(nodes, StepIdx::new(entry), &ResourceContract::DEFAULT)
}

fn general(nodes: &[CompiledNode], entry: u16) -> Result<WholeWorkflowBudget, WorkflowError> {
    // An unreferenced third node forces the actual general traversal. All test
    // edges name 0, 1 or invalid 99, so it cannot add work or make an edge valid.
    let mut reference = nodes.to_vec();
    reference.push(simple(2, K::Nop, None, None));
    compute(&reference, entry)
}

fn action() -> K {
    K::Do {
        action: ActionId::new(1),
        input: SlotIdx::new(0),
    }
}

#[test]
fn error_only_handler_matches_general_step_and_timer_budget() -> TestResult {
    let nodes = [
        simple(0, action(), None, Some(1)),
        simple(
            1,
            K::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let reference = general(&nodes, 0)?;
    assert_eq!(
        (reference.max_total_steps, reference.max_timer_entries),
        (2, 1)
    );
    let actual = compute(&nodes, 0)?;
    assert_eq!((actual.max_total_steps, actual.max_timer_entries), (2, 1));
    assert_eq!(actual.max_action_tickets, 1);
    assert_eq!(actual, reference);
    Ok(())
}

#[test]
fn normal_and_error_join_count_the_shared_handler_once() -> TestResult {
    let nodes = [
        simple(0, action(), Some(1), Some(1)),
        simple(
            1,
            K::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None,
            },
            None,
            None,
        ),
    ];
    let actual = compute(&nodes, 0)?;
    assert_eq!(actual, general(&nodes, 0)?);
    assert_eq!((actual.max_total_steps, actual.max_timer_entries), (2, 1));
    assert_eq!(actual.max_action_tickets, 1);
    assert_eq!(actual.max_signal_subscriptions, 1);
    Ok(())
}

fn kinds() -> [(K, u32, u32, u32, u32); 6] {
    [
        (K::Nop, 0, 0, 0, 0),
        (action(), 1, 0, 0, 0),
        (
            K::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
            0,
            1,
            0,
            0,
        ),
        (
            K::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None,
            },
            0,
            1,
            0,
            1,
        ),
        (
            K::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
            0,
            1,
            1,
            0,
        ),
        (
            K::Finish {
                result: SlotIdx::new(0),
            },
            0,
            0,
            0,
            0,
        ),
    ]
}

#[test]
fn all_simple_normal_pairs_preserve_the_complete_general_budget() -> TestResult {
    for (first, actions, timers, gates, signals) in kinds() {
        for (second, more_actions, more_timers, more_gates, more_signals) in kinds() {
            let nodes = [
                simple(0, first.clone(), Some(1), None),
                simple(1, second, None, None),
            ];
            let actual = compute(&nodes, 0)?;
            assert_eq!(actual, general(&nodes, 0)?);
            assert_eq!(actual.max_total_steps, 2);
            assert_eq!(actual.max_action_tickets, actions + more_actions);
            assert_eq!(actual.max_timer_entries, timers + more_timers);
            assert_eq!(actual.max_open_gates, gates + more_gates);
            assert_eq!(actual.max_signal_subscriptions, signals + more_signals);
        }
    }
    Ok(())
}

#[test]
fn forward_and_backward_error_handlers_reserve_all_resource_kinds() -> TestResult {
    for (handler_kind, actions, timers, gates, signals) in kinds() {
        for entry in [0, 1] {
            let handler = 1 - entry;
            let nodes = if entry == 0 {
                [
                    simple(0, action(), None, Some(handler)),
                    simple(1, handler_kind.clone(), None, None),
                ]
            } else {
                [
                    simple(0, handler_kind.clone(), None, None),
                    simple(1, action(), None, Some(handler)),
                ]
            };
            let actual = compute(&nodes, entry)?;
            assert_eq!(actual, general(&nodes, entry)?);
            assert_eq!(actual.max_total_steps, 2);
            assert_eq!(actual.max_action_tickets, 1 + actions);
            assert_eq!(actual.max_timer_entries, timers);
            assert_eq!(actual.max_open_gates, gates);
            assert_eq!(actual.max_signal_subscriptions, signals);
        }
    }
    Ok(())
}

#[test]
fn error_self_edge_and_unreachable_handler_keep_once_only_metrics() -> TestResult {
    for handler in [0, 1] {
        let nodes = [
            simple(0, action(), None, Some(0)),
            simple(
                1,
                K::WaitUntil {
                    deadline_slot: SlotIdx::new(0),
                },
                None,
                Some(handler),
            ),
        ];
        let actual = compute(&nodes, 0)?;
        assert_eq!(actual, general(&nodes, 0)?);
        assert_eq!((actual.max_total_steps, actual.max_timer_entries), (1, 0));
        assert_eq!(actual.max_action_tickets, 1);
    }
    Ok(())
}

#[test]
fn invalid_edges_and_entries_keep_exact_general_errors() {
    for (next, handler) in [(Some(99), None), (None, Some(99)), (Some(99), Some(1))] {
        let nodes = [
            simple(0, action(), next, handler),
            simple(1, K::Nop, None, None),
        ];
        let expected = Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99),
        });
        assert_eq!(general(&nodes, 0), expected);
        assert_eq!(compute(&nodes, 0), expected);
    }
    let nodes = [simple(0, K::Nop, None, None), simple(1, K::Nop, None, None)];
    let expected = Err(WorkflowError::EntryOutOfBounds {
        entry: StepIdx::new(99),
    });
    assert_eq!(compute(&nodes, 99), expected);
    assert_eq!(general(&nodes, 99), expected);
}
