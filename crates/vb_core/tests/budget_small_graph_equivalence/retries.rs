use super::*;
use vb_core::workflow::CompiledRetryPolicy;

pub(super) fn policy(value: Option<(u16, u64)>) -> Option<CompiledRetryPolicy> {
    value.map(|(max_attempts, delay_ms)| CompiledRetryPolicy {
        max_attempts,
        delay_ms,
    })
}

fn policies() -> Vec<Option<CompiledRetryPolicy>> {
    let mut result = vec![None];
    for attempts in [0, 1, 2, u16::MAX] {
        for delay in [0, 1, CompiledRetryPolicy::MAX_DELAY_MS, u64::MAX] {
            result.push(policy(Some((attempts, delay))));
        }
    }
    assert_eq!(result.len(), 17);
    result
}

fn routes() -> [(Option<u16>, Option<u16>); 9] {
    [
        (None, None),
        (None, Some(0)),
        (None, Some(1)),
        (Some(0), None),
        (Some(0), Some(0)),
        (Some(0), Some(1)),
        (Some(1), None),
        (Some(1), Some(0)),
        (Some(1), Some(1)),
    ]
}

#[test]
fn nop_error_only_delayed_retry_retains_general_timer() {
    let mut first = make_node(0, 0, None, Some(1));
    first.retry = policy(Some((1, 1)));
    let nodes = [first, make_node(1, 5, None, None)];
    compare(&nodes, 0, &ResourceContract::DEFAULT);
    assert_eq!(
        WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &ResourceContract::DEFAULT)
            .map(|budget| budget.max_timer_entries),
        Ok(1)
    );
}

#[test]
fn retry_timers_are_additive_to_wait_and_ask_timers_on_normal_routes() {
    for tag in 0..6 {
        let own = u32::from(matches!(tag, 2 | 3 | 4));
        for retry in policies() {
            let mut first = make_node(0, tag, Some(1), None);
            first.retry = retry;
            let nodes = [first, make_node(1, 5, None, None)];
            compare(&nodes, 0, &ResourceContract::DEFAULT);
            let delayed = u32::from(retry.is_some_and(|policy| policy.delay_ms > 0));
            assert_eq!(
                WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &ResourceContract::DEFAULT)
                    .map(|budget| budget.max_timer_entries),
                Ok(own + delayed)
            );
        }
    }
}

#[test]
fn all_simple_graphs_and_retry_metadata_preserve_every_general_budget_field() {
    let policies = policies();
    let contract = ResourceContract::DEFAULT;
    let mut cases = 0;
    for first_tag in 0..6 {
        for next in [None, Some(0)] {
            for error in [None, Some(0)] {
                for retry in &policies {
                    let mut first = make_node(0, first_tag, next, error);
                    first.retry = *retry;
                    compare(&[first], 0, &contract);
                    cases += 1;
                }
            }
        }
        for second_tag in 0..6 {
            for (next0, error0) in routes() {
                for (next1, error1) in routes() {
                    for retry0 in &policies {
                        for retry1 in &policies {
                            let mut first = make_node(0, first_tag, next0, error0);
                            let mut second = make_node(1, second_tag, next1, error1);
                            first.retry = *retry0;
                            second.retry = *retry1;
                            for entry in [0, 1] {
                                compare(&[first.clone(), second.clone()], entry, &contract);
                                cases += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 24 * 17 + 5832 * 17 * 17);
}

#[test]
fn retry_metadata_preserves_exact_invalid_edge_and_entry_errors() {
    for retry in policies() {
        for (next, error) in [(Some(99), None), (None, Some(99))] {
            let mut first = make_node(0, 0, next, error);
            first.retry = retry;
            let nodes = [first, make_node(1, 5, None, None)];
            assert_eq!(
                WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &ResourceContract::DEFAULT),
                Err(WorkflowError::StepOutOfBounds {
                    step: StepIdx::new(99)
                })
            );
            assert_eq!(
                WholeWorkflowBudget::compute(&nodes, StepIdx::new(99), &ResourceContract::DEFAULT),
                Err(WorkflowError::EntryOutOfBounds {
                    entry: StepIdx::new(99)
                })
            );
        }
    }
}
