use super::action_fixture::{ACTION, EXIT, FINISH, START, guard_slot};
use super::support::{Fixture, ROOT, TAINTS, TestResult, VALUES};
use vb_core::{
    CompiledNodeKind, CompiledWorkflow, CoreError, SlotValue, StepIdx, StepState, Taint,
};
use vb_runtime::engine::EvidenceEvent;

pub(super) fn compiled(workflow: &CompiledWorkflow) -> TestResult {
    assert_eq!(workflow.node_count(), 8);
    let guard = workflow.node(StepIdx::new(0)).ok_or("guard missing")?;
    let CompiledNodeKind::Choose {
        branches,
        otherwise,
    } = &guard.kind
    else {
        return Err("authored guard did not compile to Choose".into());
    };
    assert_eq!(branches.len(), 1);
    assert_eq!(
        branches.first().ok_or("guard branch missing")?.target,
        ACTION
    );
    assert_eq!(*otherwise, Some(START));
    let action = workflow.node(ACTION).ok_or("real Do missing")?;
    assert!(matches!(action.kind, CompiledNodeKind::Do { action, input }
        if action == vb_core::ActionId::new(0) && input == guard_slot(workflow)?));
    assert_eq!(action.next, Some(START));
    Ok(())
}

pub(super) fn cut(fixture: &Fixture, events: &[EvidenceEvent], after: bool) -> TestResult {
    frame_state(fixture, after)?;
    finish_state(fixture, after)?;
    owned(fixture)?;
    sequence(fixture, events, after)?;
    pages(fixture, events)
}

fn frame_state(fixture: &Fixture, after: bool) -> TestResult {
    let frame = &fixture.state.frame;
    assert_eq!(frame.pc(), if after { EXIT } else { FINISH });
    assert_eq!(frame.step_state(ACTION)?, StepState::Pending);
    let flag = guard_slot(&fixture.state.workflow)?;
    assert_eq!(frame.read_slot(flag)?, &SlotValue::Bool(false));
    assert_eq!(frame.read_taint(flag)?, Taint::Clean);
    assert_eq!(frame.step_state(StepIdx::new(5))?, StepState::Succeeded);
    assert_eq!(frame.step_state(EXIT)?, StepState::Pending);
    let collector = fixture.output(START)?;
    assert_eq!(frame.read_slot(collector)?, &SlotValue::List(ROOT));
    assert_eq!(frame.read_taint(collector)?, Taint::Secret);
    assert_ne!(collector, fixture.output(FINISH)?);
    assert_ne!(fixture.output(FINISH)?, fixture.input);
    Ok(())
}

fn finish_state(fixture: &Fixture, after: bool) -> TestResult {
    let frame = &fixture.state.frame;
    let output = fixture.output(FINISH)?;
    if after {
        assert_eq!(frame.step_state(FINISH)?, StepState::Succeeded);
        assert_eq!(frame.read_slot(output)?, &SlotValue::List(ROOT));
        assert_eq!(frame.read_taint(output)?, Taint::Secret);
    } else {
        assert_eq!(frame.step_state(FINISH)?, StepState::Pending);
        assert_eq!(
            frame.read_slot(output),
            Err(CoreError::SlotUninitialized { slot: output })
        );
    }
    Ok(())
}

fn owned(fixture: &Fixture) -> TestResult {
    assert_eq!(fixture.state.store.list(ROOT)?, VALUES);
    for (index, (value, taint)) in VALUES.iter().zip(TAINTS).enumerate() {
        assert_eq!(
            fixture
                .state
                .store
                .list_item_with_taint(ROOT, u32::try_from(index)?)?,
            (*value, taint)
        );
    }
    assert_eq!(fixture.state.store.list_count(), 6);
    let pagination = fixture.state.collect_states.checkpoint_v1();
    assert!(pagination.entries.is_empty());
    assert!(pagination.lineages.is_empty());
    Ok(())
}

fn sequence(fixture: &Fixture, events: &[EvidenceEvent], after: bool) -> TestResult {
    let steps: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            EvidenceEvent::StepStarted { step } => Some(step.get()),
            _ => None,
        })
        .collect();
    let expected = if after {
        vec![0, 2, 3, 4, 5, 3, 4, 5, 3, 4, 5, 6]
    } else {
        vec![0, 2, 3, 4, 5, 3, 4, 5, 3, 4, 5]
    };
    assert_eq!(steps, expected);
    assert_eq!(events.len(), if after { 35 } else { 32 });
    let tail = events.get(2..).ok_or("guard evidence missing")?;
    for (triple, raw) in tail.chunks_exact(3).zip(steps.iter().skip(1)) {
        let step = StepIdx::new(*raw);
        let output = if *raw == 5 {
            None
        } else {
            Some(fixture.output(step)?)
        };
        assert_eq!(triple.first(), Some(&EvidenceEvent::StepStarted { step }));
        assert_eq!(
            triple.last(),
            Some(&EvidenceEvent::StepSucceeded { step, output })
        );
    }

    assert_eq!(
        events.get(..2),
        Some(
            [
                EvidenceEvent::StepStarted {
                    step: StepIdx::new(0)
                },
                EvidenceEvent::StepSucceeded {
                    step: StepIdx::new(0),
                    output: None
                },
            ]
            .as_slice()
        )
    );
    Ok(())
}

fn pages(fixture: &Fixture, events: &[EvidenceEvent]) -> TestResult {
    let collector = fixture.output(START)?;
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
    assert_eq!(pages.len(), 4);
    for ((id, taint, extra), (values, cursor)) in
        pages.iter().take(3).zip(VALUES.chunks(2).zip([2, 4, 5]))
    {
        assert_eq!(fixture.state.store.list(*id)?, values);
        assert_eq!(*taint, Taint::Secret);
        let live = extra.ok_or("live page evidence missing")?;
        assert_eq!(live.source, ROOT);
        assert_eq!(live.current_page, *id);
        assert_eq!(live.collector_slot, collector);
        assert_eq!(live.cursor, cursor);
        assert_eq!(live.item_count, 5);
        assert_eq!(live.page_size, 2);
        assert_eq!(live.limit, 6);
    }
    assert_eq!(pages.last(), Some(&(ROOT, Taint::Secret, None)));
    Ok(())
}
