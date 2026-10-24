#![forbid(unsafe_code)]

use super::SOURCE;
use super::support::SourceRun;
use vb_core::{CompiledNodeKind, CoreError, ListId, SlotIdx, SlotValue, StepIdx, StepState, Taint};
use vb_runtime::engine::{EvidenceEvent, RuntimeSignal};
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
const AGGREGATE: Taint = Taint::Secret;
const NEXT: StepIdx = StepIdx::new(3);
const FINISH: StepIdx = StepIdx::new(4);
const EXIT: StepIdx = StepIdx::new(5);
type Events = Vec<EvidenceEvent>;
type Prepared = (
    SourceRun,
    vb_core::workflow::WorkflowParts,
    SlotIdx,
    SlotIdx,
);
fn assert_source(run: &SourceRun, id: ListId) -> Result<(), String> {
    run.assert_input(&VALUES, AGGREGATE)?;
    assert_eq!(id, run.input_list);
    assert_eq!(run.store.list(id).map_err(|e| format!("{e:?}"))?, VALUES);
    for (index, (value, taint)) in VALUES.iter().zip(TAINTS.iter()).enumerate() {
        let index = u32::try_from(index).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            run.store.list_item_with_taint(id, index),
            Ok((*value, *taint))
        );
    }
    Ok(())
}
fn assert_uninitialized(run: &SourceRun, slot: SlotIdx) {
    assert_eq!(
        run.frame.read_slot(slot),
        Err(CoreError::SlotUninitialized { slot })
    );
}
fn assert_boundary_events(
    events: &[EvidenceEvent],
    run: &SourceRun,
    step: StepIdx,
    slot: SlotIdx,
    output: Option<SlotIdx>,
) {
    assert_eq!(
        events,
        [
            EvidenceEvent::StepStarted { step },
            EvidenceEvent::SlotWritten {
                slot,
                value: SlotValue::List(run.input_list),
                taint: AGGREGATE,
                extra: None,
            },
            EvidenceEvent::StepSucceeded { step, output },
        ]
    );
}
fn assert_page_events(
    run: &SourceRun,
    events: &[EvidenceEvent],
    collector: SlotIdx,
) -> Result<(), String> {
    assert_eq!(events.len(), 27);
    super::assert_page_writes(run, events, &VALUES)?;
    for (triple, raw) in events.chunks_exact(3).zip([0u16, 1, 2, 3, 1, 2, 3, 1, 2]) {
        let step = StepIdx::new(raw);
        let output = if raw == 3 {
            None
        } else {
            Some(run.output(raw)?)
        };
        assert_eq!(triple.first(), Some(&EvidenceEvent::StepStarted { step }));
        assert_eq!(
            triple.last(),
            Some(&EvidenceEvent::StepSucceeded { step, output })
        );
    }
    let pages: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            EvidenceEvent::SlotWritten {
                slot,
                value: SlotValue::List(id),
                taint,
                extra,
            } if *slot == collector => Some((*id, *taint, *extra)),
            _ => None,
        })
        .collect();
    assert_eq!(pages.len(), 3);
    for ((id, taint, extra), (page, cursor)) in
        pages.into_iter().zip(VALUES.chunks(2).zip([2usize, 4, 5]))
    {
        assert_eq!(run.store.list(id).map_err(|e| format!("{e:?}"))?, page);
        assert_eq!(taint, AGGREGATE);
        let state = extra.ok_or("missing live page evidence")?;
        assert_eq!(state.run_id, run.frame.run_id());
        assert_eq!(state.collector_slot, collector);
        assert_eq!(state.source, run.input_list);
        assert_eq!(state.current_page, id);
        assert_eq!(state.cursor, cursor);
        assert_eq!(state.page_size, 2);
        assert_eq!(state.item_count, 5);
        assert_eq!(state.limit, 6);
        assert_eq!(state.time_limit_ms, None);
        if cursor == 5 {
            assert_eq!(
                run.collect_states
                    .capture_state(run.frame.run_id(), collector),
                Some(state)
            );
        }
    }
    Ok(())
}
fn prepare() -> Result<Prepared, String> {
    let mut run = SourceRun::new(SOURCE, &VALUES, AGGREGATE)?;
    // Prepare the owned input before execution; do not mutate compiled IR or PC.
    let input = run
        .store
        .insert_list_with_taint(
            VALUES.to_vec().into_boxed_slice(),
            TAINTS.to_vec().into_boxed_slice(),
        )
        .map_err(|e| format!("{e:?}"))?;
    run.frame
        .write_slot_with_taint(run.input_slot, SlotValue::List(input), AGGREGATE)
        .map_err(|e| format!("{e:?}"))?;
    run.input_list = input;
    let original = run.workflow.to_parts();
    assert_source(&run, input)?;
    assert_eq!(run.store.list_count(), 2);
    let collector = run.output(0)?;
    let output = run.output(4)?;
    assert_ne!(collector, run.input_slot);
    assert_ne!(output, collector);
    assert_ne!(output, run.input_slot);
    assert_eq!(run.workflow.node_count(), 6);
    let next_node = run
        .workflow
        .node(NEXT)
        .ok_or("missing compiled CollectNext")?;
    match next_node.kind {
        CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => {
            assert_eq!(collector_slot, collector);
            assert_eq!(body, StepIdx::new(1));
            assert_eq!(done, FINISH);
        }
        ref other => return Err(format!("expected compiled CollectNext, got {other:?}")),
    }
    let finish_node = run
        .workflow
        .node(FINISH)
        .ok_or("missing compiled CollectFinish")?;
    assert!(matches!(
        finish_node.kind,
        CompiledNodeKind::CollectFinish { collector_slot } if collector_slot == collector
    ));
    assert_eq!(finish_node.next, Some(EXIT));
    assert_uninitialized(&run, output);
    Ok((run, original, collector, output))
}
fn promote(run: &mut SourceRun, slots: (SlotIdx, SlotIdx), all: &mut Events) -> Result<(), String> {
    let (collector, output) = slots;
    let input = run.input_list;
    assert_eq!(
        run.drive(1).map_err(|e| format!("{e:?}"))?,
        RuntimeSignal::StepBudgetExhausted
    );
    assert_eq!(run.frame.pc(), FINISH);
    assert_eq!(run.frame.step_state(NEXT), Ok(StepState::Succeeded));
    assert_eq!(run.frame.step_state(FINISH), Ok(StepState::Pending));
    assert_uninitialized(&run, output);
    assert_eq!(run.frame.read_slot(collector), Ok(&SlotValue::List(input)));
    assert_eq!(run.frame.read_taint(collector), Ok(AGGREGATE));
    assert_eq!(
        run.collect_states
            .capture_state(run.frame.run_id(), collector),
        None
    );
    let events = run.evidence.drain();
    assert_boundary_events(&events, &run, NEXT, collector, None);
    all.extend(events);
    assert_eq!(run.store.list_count(), 5);
    Ok(())
}
fn finish(run: &mut SourceRun, slots: (SlotIdx, SlotIdx), all: &mut Events) -> Result<(), String> {
    let (collector, output) = slots;
    let input = run.input_list;
    assert_eq!(
        run.drive(1).map_err(|e| format!("{e:?}"))?,
        RuntimeSignal::StepBudgetExhausted
    );
    assert_eq!(run.frame.pc(), EXIT);
    assert_eq!(run.frame.step_state(FINISH), Ok(StepState::Succeeded));
    assert_eq!(run.frame.step_state(EXIT), Ok(StepState::Pending));
    assert_eq!(run.frame.read_slot(output), Ok(&SlotValue::List(input)));
    assert_eq!(run.frame.read_taint(output), Ok(AGGREGATE));
    assert_eq!(
        run.collect_states
            .capture_state(run.frame.run_id(), collector),
        None
    );
    let events = run.evidence.drain();
    assert_boundary_events(&events, &run, FINISH, output, Some(output));
    all.extend(events);
    assert_eq!(run.store.list_count(), 5);
    Ok(())
}
fn end_run(run: &mut SourceRun, slots: (SlotIdx, SlotIdx), all: &mut Events) -> Result<(), String> {
    let (collector, output) = slots;
    let input = run.input_list;
    assert_eq!(
        run.drive(1).map_err(|e| format!("{e:?}"))?,
        RuntimeSignal::Finished(SlotValue::List(input))
    );
    let events = run.evidence.drain();
    assert_eq!(
        events,
        [
            EvidenceEvent::StepStarted { step: EXIT },
            EvidenceEvent::StepSucceeded {
                step: EXIT,
                output: None,
            },
        ]
    );
    all.extend(events);
    assert_eq!(all.len(), 35);
    let output_writes: Vec<_> = all
        .iter()
        .filter(|event| {
            matches!(
                event, EvidenceEvent::SlotWritten { slot, .. } if *slot == output
            )
        })
        .collect();
    assert_eq!(output_writes.len(), 1);
    assert_eq!(run.frame.pc(), EXIT);
    assert_eq!(run.frame.step_state(EXIT), Ok(StepState::Succeeded));
    assert_eq!(run.store.list_count(), 5);
    assert_eq!(
        run.collect_states
            .capture_state(run.frame.run_id(), collector),
        None
    );
    Ok(())
}
#[test]
fn terminal_next_and_finish_resume_with_original_taints_and_exact_evidence() -> Result<(), String> {
    let (mut run, original, collector, output) = prepare()?;
    let input = run.input_list;
    // Three pages: Start, two body nodes per page, two advancing Next nodes.
    assert_eq!(
        run.drive(9).map_err(|e| format!("{e:?}"))?,
        RuntimeSignal::StepBudgetExhausted
    );
    assert_eq!(run.frame.pc(), NEXT);
    assert_eq!(run.frame.step_state(NEXT), Ok(StepState::Pending));
    assert_eq!(run.frame.step_state(FINISH), Ok(StepState::Pending));
    assert_uninitialized(&run, output);
    super::assert_body_output(&run, 0, &[SlotValue::I64(1)])?;
    super::assert_body_output(&run, 1, &[SlotValue::I64(1)])?;
    super::assert_body_output(&run, 2, &[SlotValue::I64(1)])?;
    let mut all_events = run.evidence.drain();
    assert_page_events(&run, &all_events, collector)?;
    assert_eq!(run.store.list_count(), 5);
    assert_source(&run, input)?;
    assert_eq!(run.workflow.to_parts(), original);
    promote(&mut run, (collector, output), &mut all_events)?;
    assert_source(&run, input)?;
    assert_eq!(run.workflow.to_parts(), original);
    finish(&mut run, (collector, output), &mut all_events)?;
    assert_source(&run, input)?;
    assert_eq!(run.workflow.to_parts(), original);
    end_run(&mut run, (collector, output), &mut all_events)?;
    assert_source(&run, input)?;
    assert_eq!(run.workflow.to_parts(), original);
    Ok(())
}
