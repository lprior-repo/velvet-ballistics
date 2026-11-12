use super::support::{BODY, FINAL, SourceRun, source};
use vb_core::{CoreError, EngineError, SlotValue, StepState, Taint};
use vb_runtime::engine::{EvidenceEvent, RuntimeEngineError};

fn assert_limit(max: u32, values: &[SlotValue], taint: Taint) -> Result<(), String> {
    let src = source(
        &format!("{{is: list, of: any, max: {max}}}"),
        Some(1),
        "entry",
        BODY,
        FINAL,
    );
    let mut run = SourceRun::new(&src, values, taint)?;
    let original = run.workflow.to_parts();
    let layout = run.layout(max)?;
    let first = layout.first;
    let second = layout.second;
    let result = run.drive(128);
    match result {
        Err(RuntimeEngineError::Core(EngineError::IterationLimitExceeded { resource })) => {
            if resource != "for_each_limit" {
                return Err(format!("wrong resource: {resource}"));
            }
        }
        other => return Err(format!("expected limit error, got {other:?}")),
    }
    run.assert_input(values, taint)?;
    let parts = run.workflow.to_parts();
    if parts != original {
        return Err("workflow changed".to_string());
    }
    match run.frame.read_slot(first) {
        Err(CoreError::SlotUninitialized { slot }) if slot == first => {}
        other => return Err(format!("first slot wrong: {other:?}")),
    }
    match run.frame.read_slot(second) {
        Err(CoreError::SlotUninitialized { slot }) if slot == second => {}
        other => return Err(format!("second slot wrong: {other:?}")),
    }
    for slot in [layout.item, layout.cursor, layout.output] {
        assert_eq!(
            run.frame.read_slot(slot),
            Err(CoreError::SlotUninitialized { slot })
        );
        assert_eq!(
            run.frame.read_taint(slot),
            Err(CoreError::SlotUninitialized { slot })
        );
    }
    assert_eq!(run.frame.read_slot(layout.anchor), Ok(&SlotValue::I64(42)));
    assert_eq!(run.frame.read_taint(layout.anchor), Ok(Taint::Clean));
    let SlotValue::List(accumulator) = *run
        .frame
        .read_slot(layout.accumulator)
        .map_err(|e| format!("{e:?}"))?
    else {
        return Err("missing initialized accumulator".into());
    };
    assert_eq!(
        run.store.list(accumulator).map_err(|e| format!("{e:?}"))?,
        &[]
    );
    assert_eq!(run.frame.read_taint(layout.accumulator), Ok(taint));
    assert_eq!(
        run.store.list_count(),
        3,
        "Start must not allocate after the limit failure"
    );
    assert_eq!(run.frame.step_state(layout.start), Ok(StepState::Failed));
    assert_eq!(run.frame.pc(), layout.start);
    for step in layout.before_start {
        assert_eq!(run.frame.step_state(step), Ok(StepState::Succeeded));
    }
    for step in layout.body.into_iter().chain(layout.after) {
        assert_eq!(run.frame.step_state(step), Ok(StepState::Pending));
    }
    let events = run.evidence.drain();
    let started: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            EvidenceEvent::StepStarted { step, .. } => Some(*step),
            _ => None,
        })
        .collect();
    let mut expected_started = layout.before_start.to_vec();
    expected_started.push(layout.start);
    if started != expected_started {
        return Err(format!("started wrong: {started:?}"));
    }
    let succeeded: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            EvidenceEvent::StepSucceeded { step, .. } => Some(*step),
            _ => None,
        })
        .collect();
    if succeeded != layout.before_start {
        return Err(format!("succeeded wrong: {succeeded:?}"));
    }
    let failed = events
        .iter()
        .filter_map(|e| match e {
            EvidenceEvent::StepFailed { step } => Some(*step),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(failed, Vec::new());
    let written = events
        .iter()
        .filter_map(|e| match e {
            EvidenceEvent::SlotWritten { slot, .. } => Some(*slot),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        written,
        vec![layout.anchor, layout.accumulator, layout.accumulator]
    );
    for e in &events {
        if let EvidenceEvent::SlotWritten { slot, .. } = e {
            if *slot == first || *slot == second {
                return Err("body slot written".to_string());
            }
        }
    }
    Ok(())
}

#[test]
fn foreach_len4_rejected_before_body() -> Result<(), String> {
    let values = [
        SlotValue::I64(-7),
        SlotValue::Bool(true),
        SlotValue::Null,
        SlotValue::I64(9),
    ];
    for taint in [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret] {
        assert_limit(3, &values, taint)?;
    }
    Ok(())
}

#[test]
fn foreach_nonempty_max0_rejected_before_body() -> Result<(), String> {
    let values = [SlotValue::Null];
    for taint in [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret] {
        assert_limit(0, &values, taint)?;
    }
    Ok(())
}
