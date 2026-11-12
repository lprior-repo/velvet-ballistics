#![forbid(unsafe_code)]

use super::SOURCE;
use super::support::SourceRun;
use vb_core::replay::step::{ReplayAction, ReplayCollectStates, replay_step_with_collect};
use vb_core::{CompiledNodeKind, CoreError, SlotIdx, SlotValue, StepIdx, Taint};

const VALUES: [SlotValue; 5] = [
    SlotValue::I64(7),
    SlotValue::I64(1),
    SlotValue::I64(7),
    SlotValue::I64(7),
    SlotValue::I64(1),
];
const TAINTS: [Taint; 5] = [
    Taint::Clean,
    Taint::DerivedFromSecret,
    Taint::Secret,
    Taint::DerivedFromSecret,
    Taint::Clean,
];

fn prepared_run() -> Result<SourceRun, String> {
    let mut run = SourceRun::new(SOURCE, &VALUES, Taint::Secret)?;
    let input = run
        .store
        .insert_list_with_taint(
            VALUES.to_vec().into_boxed_slice(),
            TAINTS.to_vec().into_boxed_slice(),
        )
        .map_err(|e| format!("{e:?}"))?;
    run.frame
        .write_slot_with_taint(run.input_slot, SlotValue::List(input), Taint::Secret)
        .map_err(|e| format!("{e:?}"))?;
    run.input_list = input;
    assert_eq!(run.store.list_count(), 2);
    Ok(run)
}

fn replay_one(
    run: &mut SourceRun,
    states: &mut ReplayCollectStates,
) -> Result<ReplayAction, String> {
    let pc = run.frame.pc();
    let node = run
        .workflow
        .node(pc)
        .ok_or("compiled replay node missing")?;
    replay_step_with_collect(node, &mut run.frame, &mut run.store, &run.workflow, states)
        .map_err(|e| format!("{e:?}"))
}

fn advance(
    run: &mut SourceRun,
    states: &mut ReplayCollectStates,
    at: u16,
    next: u16,
) -> Result<(), String> {
    assert_eq!(run.frame.pc(), StepIdx::new(at));
    match replay_one(run, states)? {
        ReplayAction::Continue(target) => assert_eq!(target, StepIdx::new(next)),
        _ => return Err("expected actual replay Continue".to_string()),
    }
    assert_eq!(run.frame.pc(), StepIdx::new(next));
    Ok(())
}

fn assert_list(
    run: &SourceRun,
    slot: SlotIdx,
    values: &[SlotValue],
    taint: Taint,
) -> Result<(), String> {
    let value = run.frame.read_slot(slot).map_err(|e| format!("{e:?}"))?;
    let SlotValue::List(id) = value else {
        return Err(format!("expected list, got {value:?}"));
    };
    assert_eq!(run.store.list(*id).map_err(|e| format!("{e:?}"))?, values);
    assert_eq!(run.frame.read_taint(slot), Ok(taint));
    Ok(())
}

fn assert_source(run: &SourceRun) -> Result<(), String> {
    run.assert_input(&VALUES, Taint::Secret)?;
    for (index, (value, taint)) in VALUES.iter().zip(TAINTS.iter()).enumerate() {
        let index = u32::try_from(index).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            run.store.list_item_with_taint(run.input_list, index),
            Ok((*value, *taint))
        );
    }
    Ok(())
}

fn assert_pending_output(run: &SourceRun) -> Result<(), String> {
    let slot = run.output(4)?;
    assert_eq!(
        run.frame.read_slot(slot),
        Err(CoreError::SlotUninitialized { slot })
    );
    Ok(())
}

fn replay_three_pages(run: &mut SourceRun, states: &mut ReplayCollectStates) -> Result<(), String> {
    advance(run, states, 0, 1)?;
    for (page, more) in VALUES.chunks(2).zip([true, true, false]) {
        for (at, next) in [(1u16, 2u16), (2, 3)] {
            advance(run, states, at, next)?;
            assert_list(run, run.output(at)?, page, Taint::Secret)?;
        }
        if more {
            advance(run, states, 3, 1)?;
        }
    }
    assert_eq!(run.frame.executed(), 9);
    assert_eq!(run.store.list_count(), 5);
    assert_list(run, run.output(0)?, &[SlotValue::I64(1)], Taint::Secret)?;
    assert_pending_output(run)?;
    assert_source(run)
}

fn promote_and_finish(run: &mut SourceRun, states: &mut ReplayCollectStates) -> Result<(), String> {
    let collector = run.output(0)?;
    let output = run.output(4)?;
    let original = run.workflow.to_parts();
    assert_ne!(output, collector);
    assert_ne!(output, run.input_slot);
    assert_pending_output(run)?;
    advance(run, states, 3, 4)?;
    assert_eq!(
        run.frame.read_slot(collector),
        Ok(&SlotValue::List(run.input_list))
    );
    assert_list(run, collector, &VALUES, Taint::Secret)?;
    assert_pending_output(run)?;
    assert_eq!(run.frame.executed(), 10);
    assert_eq!(run.store.list_count(), 5);
    assert_source(run)?;
    assert_eq!(run.workflow.to_parts(), original);
    advance(run, states, 4, 5)?;
    assert_eq!(
        run.frame.read_slot(output),
        Ok(&SlotValue::List(run.input_list))
    );
    assert_list(run, output, &VALUES, Taint::Secret)?;
    assert_eq!(run.frame.executed(), 11);
    assert_eq!(run.store.list_count(), 5);
    assert_source(run)?;
    assert_eq!(run.workflow.to_parts(), original);
    Ok(())
}

fn complete(run: &mut SourceRun, states: &mut ReplayCollectStates) -> Result<(), String> {
    assert_eq!(run.frame.pc(), StepIdx::new(5));
    let node = run.workflow.node(run.frame.pc()).ok_or("Finish missing")?;
    match node.kind {
        CompiledNodeKind::Finish { result } => assert_eq!(result, run.output(4)?),
        _ => return Err("compiled terminal node is not Finish".to_string()),
    }
    match replay_one(run, states)? {
        ReplayAction::Finished => {}
        _ => return Err("expected actual replay Finished".to_string()),
    }
    assert_eq!(run.frame.pc(), StepIdx::new(5));
    Ok(())
}

#[test]
fn replay_terminal_preserves_owned_source_and_item_taints() -> Result<(), String> {
    let mut run = prepared_run()?;
    let original = run.workflow.to_parts();
    let mut states = ReplayCollectStates::new();
    assert_source(&run)?;
    replay_three_pages(&mut run, &mut states)?;
    assert_eq!(run.workflow.to_parts(), original);
    promote_and_finish(&mut run, &mut states)?;
    complete(&mut run, &mut states)?;
    assert_eq!(run.frame.executed(), 12);
    assert_eq!(run.store.list_count(), 5);
    assert_eq!(
        run.frame.read_slot(run.output(4)?),
        Ok(&SlotValue::List(run.input_list))
    );
    assert_list(&run, run.output(4)?, &VALUES, Taint::Secret)?;
    assert_source(&run)?;
    assert_eq!(run.workflow.to_parts(), original);
    Ok(())
}

#[test]
fn replay_empty_source_takes_done_edge_then_finishes() -> Result<(), String> {
    let mut run = SourceRun::new(SOURCE, &[], Taint::Clean)?;
    let original = run.workflow.to_parts();
    let mut states = ReplayCollectStates::new();
    let collector = run.output(0)?;
    let output = run.output(4)?;
    assert_ne!(output, collector);
    assert_pending_output(&run)?;
    advance(&mut run, &mut states, 0, 4)?;
    assert_list(&run, collector, &[], Taint::Clean)?;
    assert_pending_output(&run)?;
    assert_eq!(run.store.list_count(), 2);
    let empty = *run
        .frame
        .read_slot(collector)
        .map_err(|e| format!("{e:?}"))?;
    advance(&mut run, &mut states, 4, 5)?;
    assert_eq!(run.frame.read_slot(output), Ok(&empty));
    assert_list(&run, output, &[], Taint::Clean)?;
    assert_eq!(run.store.list_count(), 2);
    complete(&mut run, &mut states)?;
    assert_eq!(run.frame.executed(), 3);
    assert_eq!(run.store.list_count(), 2);
    run.assert_input(&[], Taint::Clean)?;
    assert_eq!(run.workflow.to_parts(), original);
    Ok(())
}
