use super::support::{BODY, FINAL, SourceRun, source};
use vb_core::{CoreError, EngineError, SlotValue, Taint};
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
    let first = run.output(2)?;
    let second = run.output(3)?;
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
    let events = run.evidence.drain();
    let started: Vec<u16> = events
        .iter()
        .filter_map(|e| match e {
            EvidenceEvent::StepStarted { step, .. } => Some(step.get()),
            _ => None,
        })
        .collect();
    if started != [0, 1] {
        return Err(format!("started wrong: {started:?}"));
    }
    let succeeded: Vec<u16> = events
        .iter()
        .filter_map(|e| match e {
            EvidenceEvent::StepSucceeded { step, .. } => Some(step.get()),
            _ => None,
        })
        .collect();
    if succeeded != [0] {
        return Err(format!("succeeded wrong: {succeeded:?}"));
    }
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
