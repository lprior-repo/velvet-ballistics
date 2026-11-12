use super::support::{
    EXIT, FINISH, Fixture, NEXT, ROOT, SOURCE, TAINTS, TestResult, VALUES, at_cut, drive,
};
use vb_core::{CoreError, SlotValue, StepState, Taint};
use vb_runtime::engine::{EvidenceEvent, RuntimeSignal};
use vb_runtime::run_continuation::{ContinuationCut, RunContinuationV1};
use vb_runtime::shard::types::RegisteredExecutionMode;

fn assert_owned(fixture: &Fixture) -> TestResult {
    assert_eq!(fixture.source.source, SOURCE.as_bytes());
    assert_eq!(
        fixture.state.frame.read_slot(fixture.input)?,
        &SlotValue::List(ROOT)
    );
    assert_eq!(
        fixture.state.frame.read_taint(fixture.input)?,
        Taint::Secret
    );
    assert_eq!(fixture.state.store.list(ROOT)?, VALUES);
    for (raw, (value, taint)) in VALUES.iter().zip(TAINTS.iter()).enumerate() {
        let index = u32::try_from(raw)?;
        assert_eq!(
            fixture.state.store.list_item_with_taint(ROOT, index)?,
            (*value, *taint)
        );
    }
    assert_eq!(
        fixture
            .state
            .store
            .list_item_with_taint(vb_core::ListId::new(0), 1)?,
        (SlotValue::I64(1), Taint::DerivedFromSecret)
    );
    assert_eq!(fixture.state.store.list_count(), 6);
    assert!(
        fixture
            .state
            .collect_states
            .checkpoint_v1()
            .entries
            .is_empty()
    );
    assert!(
        fixture
            .state
            .collect_states
            .checkpoint_v1()
            .lineages
            .is_empty()
    );
    Ok(())
}

fn assert_cut(fixture: &Fixture, after: bool) -> TestResult {
    let frame = &fixture.state.frame;
    let collector = fixture.output(vb_core::StepIdx::new(0))?;
    let output = fixture.output(FINISH)?;
    assert_ne!(collector, output);
    assert_ne!(collector, fixture.input);
    assert_ne!(output, fixture.input);
    assert_eq!(frame.pc(), if after { EXIT } else { FINISH });
    assert_eq!(frame.step_state(NEXT)?, StepState::Succeeded);
    assert_eq!(frame.step_state(EXIT)?, StepState::Pending);
    assert_eq!(frame.read_slot(collector)?, &SlotValue::List(ROOT));
    assert_eq!(frame.read_taint(collector)?, Taint::Secret);
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
    assert_body(fixture)?;
    assert_owned(fixture)
}

fn assert_body(fixture: &Fixture) -> TestResult {
    for step in [1, 2] {
        let slot = fixture.output(vb_core::StepIdx::new(step))?;
        let SlotValue::List(id) = fixture.state.frame.read_slot(slot)? else {
            return Err("body output not list".into());
        };
        assert_eq!(fixture.state.store.list(*id)?, [SlotValue::I64(1)]);
        assert_eq!(fixture.state.frame.read_taint(slot)?, Taint::Secret);
    }
    Ok(())
}

fn assert_finish_events(fixture: &Fixture, events: &[EvidenceEvent]) -> TestResult {
    let output = fixture.output(FINISH)?;
    assert_eq!(
        events,
        [
            EvidenceEvent::StepStarted { step: FINISH },
            EvidenceEvent::SlotWritten {
                slot: output,
                value: SlotValue::List(ROOT),
                taint: Taint::Secret,
                extra: None,
            },
            EvidenceEvent::StepSucceeded {
                step: FINISH,
                output: Some(output),
            },
        ]
    );
    Ok(())
}

fn capture_restore(fixture: &mut Fixture, after: bool) -> TestResult {
    let original = fixture.state.clone();
    assert_cut(fixture, after)?;
    let authority = fixture.authority()?;
    let checkpoint = RunContinuationV1::capture_transport(
        &fixture.state,
        &authority,
        super::golden::limits(1_048_576)?,
    )?;
    assert_eq!(fixture.state, original);
    let expected = if after {
        ContinuationCut::AfterCollectFinish
    } else {
        ContinuationCut::BeforeCollectFinish
    };
    let (decoded, _) = RunContinuationV1::from_transport_bytes(
        checkpoint.as_bytes(),
        &authority,
        super::golden::limits(1_048_576)?,
    )?;
    assert_eq!(decoded.cut(), expected);
    fixture.state = decoded.restore(authority)?;
    let mut reconciled = original;
    reconciled.registered_execution = RegisteredExecutionMode::ReconcileOnly;
    assert_eq!(fixture.state, reconciled);
    assert_cut(fixture, after)
}

fn resume(mut fixture: Fixture, after: bool) -> TestResult {
    let parts = fixture.state.workflow.to_parts();
    capture_restore(&mut fixture, after)?;
    if !after {
        let (signal, events) = drive(&mut fixture.state, 1)?;
        assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
        assert_finish_events(&fixture, &events)?;
        assert_cut(&fixture, true)?;
    }
    let (signal, events) = drive(&mut fixture.state, 1)?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::List(ROOT)));
    assert_eq!(
        events,
        [
            EvidenceEvent::StepStarted { step: EXIT },
            EvidenceEvent::StepSucceeded {
                step: EXIT,
                output: None
            },
        ]
    );
    assert_eq!(fixture.state.frame.executed(), 12);
    assert_eq!(fixture.state.frame.step_state(EXIT)?, StepState::Succeeded);
    assert_eq!(fixture.state.workflow.to_parts(), parts);
    assert_owned(&fixture)
}

#[test]
fn real_terminal_before_finish_restores_exact_owned_state_and_finishes_once() -> TestResult {
    resume(at_cut(false)?, false)
}

#[test]
fn real_after_finish_restores_exact_owned_state_and_runs_only_exit() -> TestResult {
    resume(at_cut(true)?, true)
}

#[test]
fn complete_digest_distinguishes_both_real_cuts() -> TestResult {
    let before = at_cut(false)?;
    let after = at_cut(true)?;
    let first = RunContinuationV1::capture(&before.state, &before.authority()?)?;
    let second = RunContinuationV1::capture(&after.state, &after.authority()?)?;
    assert_ne!(first.digest(), second.digest());
    assert_eq!(
        before.state.store.checkpoint_v1(),
        after.state.store.checkpoint_v1()
    );
    Ok(())
}
