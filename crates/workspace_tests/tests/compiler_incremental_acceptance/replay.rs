use super::*;
use vb_core::replay::step::{ReplayAction, ReplayCollectStates, replay_step_with_collect};

fn replay_values(
    workflow: &CompiledWorkflow,
    items: &[i64],
) -> Result<(Vec<SlotValue>, Taint), String> {
    let mut frame = RunFrame::new(
        RunId::new(1),
        StepIdx::new(0),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|error| format!("replay frame: {error}"))?;
    let mut store = ValueStore::new();
    let input = workflow
        .input_slots()
        .first()
        .ok_or("replay input missing")?
        .slot;
    let values: Vec<SlotValue> = items.iter().copied().map(SlotValue::I64).collect();
    let list = store
        .insert_list(values.into_boxed_slice())
        .map_err(|error| format!("replay input list: {error}"))?;
    frame
        .write_slot_with_taint(input, SlotValue::List(list), Taint::Secret)
        .map_err(|error| format!("replay input slot: {error}"))?;
    let finish_result = workflow
        .to_parts()
        .nodes
        .into_iter()
        .find_map(|node| match node.kind {
            vb_core::workflow::CompiledNodeKind::Finish { result } => Some(result),
            _ => None,
        })
        .ok_or("replay finish node missing")?;
    let parts = workflow.to_parts();
    let bound = WholeWorkflowBudget::compute(
        &parts.nodes,
        workflow.entry(),
        &workflow.resource_contract(),
    )
    .map_err(|error| format!("replay workflow bound: {error:?}"))?
    .max_total_steps;
    let mut pc = workflow.entry();
    let mut states = ReplayCollectStates::new();
    for _ in 0..bound {
        let node = workflow
            .node(pc)
            .ok_or_else(|| format!("replay node missing: {pc:?}"))?;
        match replay_step_with_collect(node, &mut frame, &mut store, workflow, &mut states)
            .map_err(|error| format!("replay step: {error:?}"))?
        {
            ReplayAction::Continue(next) => pc = next,
            ReplayAction::Finished => {
                let SlotValue::List(result) = frame
                    .read_slot(finish_result)
                    .map_err(|error| format!("replay result slot: {error}"))?
                else {
                    return Err("replay finish result is not a list".into());
                };
                let values = store
                    .list(*result)
                    .map_err(|error| format!("replay result list: {error}"))?
                    .to_vec();
                let taint = frame
                    .read_taint(finish_result)
                    .map_err(|error| format!("replay result taint: {error}"))?;
                return Ok((values, taint));
            }
            ReplayAction::Suspended { .. } => return Err("replay unexpectedly suspended".into()),
            _ => return Err("replay returned a nonterminal action".into()),
        }
    }
    Err(format!("replay exceeded workflow bound {bound}"))
}

#[test]
fn replay_matches_public_runtime_values_and_taint_at_once_two() -> Result<(), String> {
    let workflow = compiled_source()?;
    let input = [11, 22, 33];
    let runtime = run_with_repeated_budget(&workflow, &input, 10_000)?;
    let (replayed_values, replayed_taint) = replay_values(&workflow, &input)?;
    assert_eq!(replayed_values, runtime.values);
    assert_eq!(replayed_taint, runtime.result_taint);
    assert_eq!(replayed_values, [11, 22, 33].map(SlotValue::I64));
    assert_eq!(replayed_taint, Taint::Secret);
    Ok(())
}
