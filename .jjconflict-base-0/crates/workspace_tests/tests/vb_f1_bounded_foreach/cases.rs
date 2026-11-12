use super::support::{BODY, FINAL, SourceRun, source};
use std::collections::BTreeSet;
use vb_core::{CoreError, SlotIdx, SlotValue, Taint};
use vb_runtime::engine::{EvidenceEvent, RuntimeSignal};

fn assert_case(
    max: u32,
    values: &[SlotValue],
    taint: Taint,
    expected_steps: &[u16],
) -> Result<(), String> {
    let src = source(
        &format!("{{is: list, of: any, max: {max}}}"),
        Some(1),
        "entry",
        BODY,
        FINAL,
    );
    let mut run =
        SourceRun::new(&src, values, taint).map_err(|e| format!("SourceRun::new: {e:?}"))?;
    let parts = run.workflow.to_parts();
    let input_slot = run.input_slot;
    let first = run.output(2).map_err(|e| format!("output(2): {e:?}"))?;
    let second = run.output(3).map_err(|e| format!("output(3): {e:?}"))?;
    let anchor = run.output(0).map_err(|e| format!("output(0): {e:?}"))?;
    let mut slots = BTreeSet::new();
    slots.insert(input_slot);
    slots.insert(first);
    slots.insert(second);
    slots.insert(anchor);
    assert_eq!(slots.len(), 4, "expected 4 distinct slots");
    let sig = run.drive(128).map_err(|e| format!("drive: {e:?}"))?;
    assert_eq!(
        sig,
        RuntimeSignal::Finished(SlotValue::I64(42)),
        "expected Finished(I64(42))"
    );
    run.assert_input(values, taint)
        .map_err(|e| format!("assert_input: {e:?}"))?;
    assert_eq!(run.workflow.to_parts(), parts, "workflow parts changed");
    assert_slots(&run, first, second, anchor, values, taint)?;
    assert_evidence(&mut run, first, second, values, taint, expected_steps);
    Ok(())
}

fn assert_slots(
    run: &SourceRun,
    first: SlotIdx,
    second: SlotIdx,
    anchor: SlotIdx,
    values: &[SlotValue],
    taint: Taint,
) -> Result<(), String> {
    let anchor_val = run
        .frame
        .read_slot(anchor)
        .map_err(|e| format!("{:?}", e))?;
    assert_eq!(*anchor_val, SlotValue::I64(42));
    let anchor_taint = run
        .frame
        .read_taint(anchor)
        .map_err(|e| format!("{:?}", e))?;
    assert_eq!(anchor_taint, Taint::Clean);
    for slot in [first, second] {
        if let Some(last) = values.last() {
            let val = run.frame.read_slot(slot).map_err(|e| format!("{:?}", e))?;
            assert_eq!(*val, *last);
            let slot_taint = run.frame.read_taint(slot).map_err(|e| format!("{:?}", e))?;
            assert_eq!(slot_taint, taint);
        } else {
            let result = run.frame.read_slot(slot);
            assert_eq!(result, Err(CoreError::SlotUninitialized { slot }));
        }
    }
    Ok(())
}

fn assert_evidence(
    run: &mut SourceRun,
    first: SlotIdx,
    second: SlotIdx,
    values: &[SlotValue],
    taint: Taint,
    expected_steps: &[u16],
) {
    let events = run.evidence.drain();
    let mut succeeded = Vec::new();
    let mut writes = Vec::new();
    for ev in events {
        match ev {
            EvidenceEvent::StepSucceeded { step, .. } => succeeded.push(step.get()),
            EvidenceEvent::SlotWritten {
                slot,
                value,
                taint: actual_taint,
                ..
            } if slot == first || slot == second => writes.push((slot, value, actual_taint)),
            _ => {}
        }
    }
    assert_eq!(succeeded, expected_steps);
    let mut expected_writes = Vec::new();
    for v in values {
        expected_writes.push((first, *v, taint));
        expected_writes.push((second, *v, taint));
    }
    assert_eq!(writes, expected_writes);
}

#[test]
fn foreach_empty_max3() -> Result<(), String> {
    let values: Vec<SlotValue> = vec![];
    let steps = vec![0, 1, 5];
    for taint in [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret] {
        assert_case(3, &values, taint, &steps)?;
    }
    Ok(())
}

#[test]
fn foreach_one_max3() -> Result<(), String> {
    let values = vec![SlotValue::I64(-7)];
    let steps = vec![0, 1, 2, 3, 4, 5];
    for taint in [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret] {
        assert_case(3, &values, taint, &steps)?;
    }
    Ok(())
}

#[test]
fn foreach_three_mixed_max3() -> Result<(), String> {
    let values = vec![SlotValue::I64(-7), SlotValue::Bool(true), SlotValue::Null];
    let steps = vec![0, 1, 2, 3, 4, 2, 3, 4, 2, 3, 4, 5];
    for taint in [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret] {
        assert_case(3, &values, taint, &steps)?;
    }
    Ok(())
}

#[test]
fn foreach_empty_max0() -> Result<(), String> {
    let values: Vec<SlotValue> = vec![];
    let steps = vec![0, 1, 5];
    for taint in [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret] {
        assert_case(0, &values, taint, &steps)?;
    }
    Ok(())
}
