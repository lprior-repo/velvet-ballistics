use super::corruption::change_frame;
use super::support::{FINISH, NEXT, ROOT, TestResult, at_cut, drive, fixture, fixture_source};
use vb_core::{SlotValue, Taint};
use vb_runtime::run_continuation::{RunContinuationError, RunContinuationV1};

#[test]
fn live_next_boundary_and_revived_terminal_pagination_are_refused() -> TestResult {
    let mut fixture = fixture()?;
    let authority = fixture.authority()?;
    let (signal, events) = drive(&mut fixture.state, 9)?;
    assert_eq!(
        signal,
        vb_runtime::engine::RuntimeSignal::StepBudgetExhausted
    );
    assert_eq!(events.len(), 27);
    assert_eq!(fixture.state.frame.pc(), NEXT);
    let collector = fixture.output(vb_core::StepIdx::new(0))?;
    let live = fixture
        .state
        .collect_states
        .capture_state(fixture.expected.run, collector)
        .ok_or("live state missing")?;
    assert_eq!(live.source, ROOT);
    assert!(matches!(
        RunContinuationV1::capture(&fixture.state, &authority),
        Err(RunContinuationError::UnsupportedBoundary)
    ));
    let (signal, events) = drive(&mut fixture.state, 1)?;
    assert_eq!(
        signal,
        vb_runtime::engine::RuntimeSignal::StepBudgetExhausted
    );
    assert_eq!(events.len(), 3);
    assert_eq!(fixture.state.frame.pc(), FINISH);
    fixture
        .state
        .collect_states
        .upsert(live)
        .map_err(|e| format!("{e:?}"))?;
    assert!(matches!(
        RunContinuationV1::capture(&fixture.state, &authority),
        Err(RunContinuationError::CollectState)
    ));
    Ok(())
}

#[test]
fn collector_taint_and_pending_finish_output_are_checked() -> TestResult {
    let fixture = at_cut(false)?;
    let authority = fixture.authority()?;
    let collector = fixture.output(vb_core::StepIdx::new(0))?;
    let output = fixture.output(FINISH)?;
    for slot in [collector, output] {
        let mut state = fixture.state.clone();
        change_frame(&mut state, |frame| {
            *frame
                .slots
                .get_mut(usize::from(slot.get()))
                .ok_or("slot missing")? = Some(SlotValue::List(ROOT));
            *frame
                .taint
                .get_mut(usize::from(slot.get()))
                .ok_or("taint missing")? = if slot == collector {
                Taint::DerivedFromSecret
            } else {
                Taint::Secret
            };
            Ok(())
        })?;
        assert!(matches!(
            RunContinuationV1::capture(&state, &authority),
            Err(RunContinuationError::RuntimeStore(_) | RunContinuationError::BoundaryState)
        ));
    }
    Ok(())
}

#[test]
fn waiting_asking_running_and_parallel_conditions_are_explicitly_refused() -> TestResult {
    let fixture = at_cut(false)?;
    let authority = fixture.authority()?;
    for status in [
        vb_core::frame::CheckpointStepState::Waiting,
        vb_core::frame::CheckpointStepState::Asking,
        vb_core::frame::CheckpointStepState::Running,
    ] {
        let mut state = fixture.state.clone();
        change_frame(&mut state, |frame| {
            *frame.states.get_mut(1).ok_or("step missing")? = status;
            Ok(())
        })?;
        assert!(matches!(
            RunContinuationV1::capture(&state, &authority),
            Err(RunContinuationError::InFlight)
        ));
    }
    let mut state = fixture.state.clone();
    change_frame(&mut state, |frame| {
        frame.max_parallel_in_flight = 1;
        frame.parallel_in_flight = 1;
        Ok(())
    })?;
    assert!(matches!(
        RunContinuationV1::capture(&state, &authority),
        Err(RunContinuationError::CounterBounds | RunContinuationError::InFlight)
    ));
    Ok(())
}

#[test]
fn real_authored_wait_suspension_requires_missing_reconciliation_authority() -> TestResult {
    let source = super::support::SOURCE.replacen(
        "  - id: bounded",
        "  - id: signal\n    set: { output: event_code, value: \"1\" }\n  - id: external\n    wait: { event: \"$event_code\" }\n  - id: bounded",
        1,
    );
    let mut fixture = fixture_source(&source)?;
    let authority = fixture.authority()?;
    let parts = fixture.state.workflow.to_parts();
    let mut budget = vb_core::StepBudget::new(100);
    let mut evidence = vb_runtime::engine::EvidenceCollector::new();
    let state = &mut fixture.state;
    let granted = state
        .admission
        .as_ref()
        .ok_or("admission missing")?
        .granted_capabilities();
    let signal = vb_runtime::engine::drive_deterministic_full(
        &state.workflow,
        &mut state.frame,
        &mut budget,
        &mut state.store,
        &state.action_contracts,
        vb_runtime::engine::RetryPolicy::NEVER,
        &mut evidence,
        &mut state.collect_states,
        granted,
        state.run_attempt.get(),
    )?;
    assert_eq!(signal, vb_runtime::engine::RuntimeSignal::AwaitingWait);
    assert!(budget.remaining() > 0);
    assert_eq!(state.workflow.to_parts(), parts);
    assert!(matches!(
        RunContinuationV1::capture(state, &authority),
        Err(RunContinuationError::InFlight)
    ));
    Ok(())
}
