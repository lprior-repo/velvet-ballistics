#![forbid(unsafe_code)]
//! Live branch ordering, join results and taint propagation.

use super::support::*;
use vb_core::{SlotIdx, SlotValue, Taint};

#[test]
fn live_together_runs_branches_in_declaration_order_and_joins_them() -> TestResult {
    let live = live(FANOUT, Box::new([]))?;
    let events = events(&live)?;
    let (_start, branches, join) = together_start(&live.plan)?;
    assert_eq!(branches.len(), 3, "declared branch count");
    let order: Vec<vb_core::StepIdx> = succeeded(&events)
        .into_iter()
        .filter(|step| branches.contains(step))
        .collect();
    assert_eq!(
        order, branches,
        "branch entry steps must execute in declaration order"
    );
    assert_eq!(
        succeeded(&events)
            .iter()
            .filter(|step| **step == join)
            .count(),
        1,
        "the join succeeds exactly once after all branches"
    );

    let finish = finish_result(&live.plan)?;
    let state = &live.state;
    assert_eq!(state.frame.parallel_in_flight(), 0, "join releases slots");
    assert_eq!(
        list_value(&state.store, *state.frame.read_slot(finish)?)?,
        vec![SlotValue::I64(7), SlotValue::I64(11), SlotValue::I64(13)],
        "join result preserves branch declaration order"
    );
    assert_eq!(
        list_taints(&state.store, *state.frame.read_slot(finish)?)?,
        vec![Taint::Clean, Taint::Clean, Taint::Clean]
    );
    let terminal: Vec<SlotIdx> = events
        .iter()
        .filter_map(|event| match event {
            vb_storage::JournalEvent::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .collect();
    assert_eq!(terminal, vec![finish], "one terminal receipt for the join");
    Ok(())
}

#[test]
fn live_together_join_merges_branch_taints_per_item() -> TestResult {
    let plan = tainted_plan()?;
    let hidden = hidden_slot(&plan)?;
    let live = live(TAINTED_FANOUT, Box::new([(hidden, SlotValue::I64(23))]))?;
    assert_eq!(live.plan.digest(), plan.digest());
    let result_slot = finish_result(&live.plan)?;
    let state = &live.state;
    let joined = *state.frame.read_slot(result_slot)?;
    assert_eq!(
        list_value(&state.store, joined)?,
        vec![SlotValue::I64(23), SlotValue::I64(11)],
        "tainted branch result must be resolved, not the input symbol"
    );
    assert_eq!(
        list_taints(&state.store, joined)?,
        vec![Taint::Secret, Taint::Clean],
        "join keeps each branch item taint"
    );
    assert_eq!(
        state.frame.read_taint(result_slot)?,
        Taint::Secret,
        "join output taint is the join of branch taints"
    );
    Ok(())
}

#[test]
fn live_together_repeated_activation_rebuilds_ordered_result() -> TestResult {
    let source = r#"version: velvet-ballistics/v1
name: together_repeated_activation
when: {manual: {}}
vars: {xs: [3, 5]}
steps:
  - id: seed
    set: {output: saved, value: '42'}
  - id: source
    set: {output: source, value: '$vars.xs'}
  - id: loop
    for_each:
      variable: entry
      input: $source
      at_once: 1
      steps:
        - id: group
          together:
            branches:
              - label: left
                steps:
                  - id: left_value
                    set: {output: left, value: '7'}
              - label: right
                steps:
                  - id: right_value
                    set: {output: right, value: '11'}
  - id: done
    finish: {result: '$saved'}
"#;
    let live = live(source, Box::new([]))?;
    let events = events(&live)?;
    let (_start, branches, join) = together_start(&live.plan)?;
    assert_eq!(
        succeeded(&events)
            .iter()
            .filter(|step| **step == join)
            .count(),
        2,
        "each for_each activation completes its own Together join"
    );
    let order: Vec<vb_core::StepIdx> = succeeded(&events)
        .into_iter()
        .filter(|step| branches.contains(step))
        .collect();
    assert_eq!(
        order,
        [branches.clone(), branches].concat(),
        "each activation repeats the declared branch order"
    );
    let join_slot = live
        .plan
        .node(join)
        .and_then(|node| node.output.slot())
        .ok_or("join output missing")?;
    assert_eq!(
        list_value(&live.state.store, *live.state.frame.read_slot(join_slot)?)?,
        vec![SlotValue::I64(7), SlotValue::I64(11)],
        "the final activation result keeps branch order"
    );
    assert_eq!(
        live.state.frame.parallel_in_flight(),
        0,
        "repeated activations release every reservation"
    );
    Ok(())
}
