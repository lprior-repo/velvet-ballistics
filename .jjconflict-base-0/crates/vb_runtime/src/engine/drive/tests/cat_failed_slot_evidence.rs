use super::common::{errh, fin, mkr, mkwf, wuntil};
use crate::engine::{
    EvidenceCollector, EvidenceEvent as E, RetryPolicy, RuntimeEngineError, RuntimeSignal,
    drive_deterministic_full,
};
use crate::primitives::collect::CollectStates;
use vb_core::workflow::ErrorRouting;
use vb_core::{
    CapabilitySet, CompiledWorkflow, CoreError, RunFrame, SlotIdx, SlotValue, StepBudget, StepIdx,
    Taint,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fixture(legacy: bool, error_slot: Option<SlotIdx>) -> TestResult<(CompiledWorkflow, RunFrame)> {
    let failed = u16::from(legacy);
    let handler = failed.checked_add(1).ok_or("handler")?;
    let mut wait = wuntil(failed, 0);
    wait.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(handler),
        error_slot: if legacy { None } else { error_slot },
    };
    if error_slot.is_none() {
        wait.output = vb_core::workflow::NodeOutput::Slot(SlotIdx::new(2));
    }
    let mut nodes = Vec::new();
    if legacy {
        let mut wrapper = errh(0, failed, handler, error_slot.map(SlotIdx::get));
        wrapper.routing.error = ErrorRouting::Handler {
            handler: StepIdx::new(handler),
            error_slot,
        };
        nodes.push(wrapper);
    }
    nodes.extend([wait, fin(handler, if error_slot.is_some() { 2 } else { 0 })]);
    let workflow = mkwf(nodes, 3, &[0, 2])?;
    let mut frame = mkr(workflow.node_count(), workflow.slot_count())?;
    frame.write_slot(SlotIdx::ZERO, SlotValue::Bool(true))?;
    frame.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(-99), Taint::Secret)?;
    Ok((workflow, frame))
}

fn drive(
    plan: &CompiledWorkflow,
    frame: &mut RunFrame,
    evidence: &mut EvidenceCollector,
) -> Result<RuntimeSignal, RuntimeEngineError> {
    drive_deterministic_full(
        plan,
        frame,
        &mut StepBudget::new(16),
        &mut vb_core::ValueStore::new(),
        &[],
        RetryPolicy::NEVER,
        evidence,
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )
}

#[test]
fn recovered_failures_record_exact_error_slot_and_taint_before_step_failed() -> TestResult {
    for legacy in [false, true] {
        let (plan, mut frame) = fixture(legacy, Some(SlotIdx::new(2)))?;
        let mut evidence = EvidenceCollector::new();
        let failed = StepIdx::new(u16::from(legacy));
        let outcome = drive(&plan, &mut frame, &mut evidence)?;
        let events = evidence.drain();
        assert_eq!(
            outcome,
            RuntimeSignal::Finished(SlotValue::I64(i64::from(failed.get()))),
            "legacy={legacy}, events={events:?}, nodes={:?}",
            plan.to_parts().nodes,
        );
        let position = events
            .iter()
            .position(|event| *event == E::StepStarted { step: failed })
            .ok_or("failed start")?;
        assert_eq!(
            events.get(position.saturating_add(1)),
            Some(&E::SlotWritten {
                slot: SlotIdx::new(2),
                value: SlotValue::I64(i64::from(failed.get())),
                taint: Taint::Clean,
                extra: None
            })
        );
        assert_eq!(
            events.get(position.saturating_add(2)),
            Some(&E::StepFailed { step: failed })
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, E::SlotWritten { .. }))
                .count(),
            1
        );
    }
    Ok(())
}

#[test]
fn a_failure_without_error_slot_never_records_an_old_payload() -> TestResult {
    let (plan, mut frame) = fixture(false, None)?;
    let mut evidence = EvidenceCollector::new();
    assert_eq!(
        drive(&plan, &mut frame, &mut evidence)?,
        RuntimeSignal::Finished(SlotValue::Bool(true))
    );
    let events = evidence.drain();
    assert!(events.contains(&E::StepFailed {
        step: StepIdx::ZERO
    }));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, E::SlotWritten { .. }))
    );
    assert_eq!(frame.read_taint(SlotIdx::new(2))?, Taint::Secret);
    Ok(())
}

#[test]
fn missing_error_slot_evidence_capacity_cannot_report_successful_recovery() -> TestResult {
    let (plan, mut frame) = fixture(false, Some(SlotIdx::new(2)))?;
    let mut evidence = EvidenceCollector::with_capacity(1);
    assert!(
        matches!(drive(&plan, &mut frame, &mut evidence), Err(RuntimeEngineError::Core(CoreError::EvidenceCapacityExceeded { slot, required: "evidence SlotWritten event", .. })) if slot == SlotIdx::new(2))
    );
    assert_eq!(
        evidence.drain(),
        vec![E::StepStarted {
            step: StepIdx::ZERO
        }]
    );
    Ok(())
}
