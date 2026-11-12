use super::support::{BODY, FINAL, LoopLayout, SourceRun, source};
use std::collections::BTreeSet;
use vb_core::{CoreError, SlotValue, StepState, Taint};
use vb_runtime::engine::{EvidenceEvent, RuntimeSignal};

fn assert_case(max: u32, values: &[SlotValue], taint: Taint) -> Result<(), String> {
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
    let layout = run.layout(max)?;
    let slots = [
        run.input_slot,
        layout.first,
        layout.second,
        layout.anchor,
        layout.accumulator,
        layout.item,
        layout.cursor,
        layout.output,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    assert_eq!(
        slots.len(),
        8,
        "source, body, and loop-private/public slots must be distinct"
    );
    let sig = run.drive(128).map_err(|e| format!("drive: {e:?}"))?;
    assert_eq!(
        sig,
        RuntimeSignal::Finished(SlotValue::I64(42)),
        "expected Finished(I64(42))"
    );
    run.assert_input(values, taint)
        .map_err(|e| format!("assert_input: {e:?}"))?;
    assert_eq!(run.workflow.to_parts(), parts, "workflow parts changed");
    assert_slots(&run, &layout, values, taint)?;
    assert_evidence(&mut run, &layout, values, taint)?;
    Ok(())
}

fn assert_slots(
    run: &SourceRun,
    layout: &LoopLayout,
    values: &[SlotValue],
    taint: Taint,
) -> Result<(), String> {
    assert_eq!(run.frame.read_slot(layout.anchor), Ok(&SlotValue::I64(42)));
    assert_eq!(run.frame.read_taint(layout.anchor), Ok(Taint::Clean));
    for slot in [layout.first, layout.second, layout.item] {
        if let Some(last) = values.last() {
            assert_eq!(run.frame.read_slot(slot), Ok(last));
            assert_eq!(run.frame.read_taint(slot), Ok(taint));
        } else {
            assert_eq!(
                run.frame.read_slot(slot),
                Err(CoreError::SlotUninitialized { slot })
            );
        }
    }
    let SlotValue::List(output) = *run
        .frame
        .read_slot(layout.output)
        .map_err(|e| format!("{e:?}"))?
    else {
        return Err("ForEach output must be a list".into());
    };
    assert_ne!(output, run.input_list);
    assert_eq!(
        run.frame.read_slot(layout.accumulator),
        Ok(&SlotValue::List(output))
    );
    assert_eq!(
        run.store.list(output).map_err(|e| format!("{e:?}"))?,
        values
    );
    let expected = values
        .iter()
        .map(|value| (*value, taint))
        .collect::<Vec<_>>();
    assert_eq!(
        run.store
            .list_items_with_taint(output)
            .map_err(|e| format!("{e:?}"))?
            .collect::<Vec<_>>(),
        expected
    );
    for slot in [layout.accumulator, layout.output, layout.cursor] {
        assert_eq!(run.frame.read_taint(slot), Ok(taint));
    }
    let SlotValue::List(cursor) = *run
        .frame
        .read_slot(layout.cursor)
        .map_err(|e| format!("{e:?}"))?
    else {
        return Err("cursor must be a list".into());
    };
    let expected_cursor = if values.is_empty() {
        vec![]
    } else {
        vec![
            SlotValue::I64(i64::from(run.input_list.get())),
            SlotValue::I64(i64::try_from(values.len()).map_err(|e| e.to_string())?),
        ]
    };
    assert_eq!(
        run.store.list(cursor).map_err(|e| format!("{e:?}"))?,
        expected_cursor
    );
    for step in layout
        .before_start
        .into_iter()
        .chain([layout.start])
        .chain(layout.after)
    {
        assert_eq!(run.frame.step_state(step), Ok(StepState::Succeeded));
    }
    let body_state = if values.is_empty() {
        StepState::Pending
    } else {
        StepState::Succeeded
    };
    for step in layout.body {
        assert_eq!(run.frame.step_state(step), Ok(body_state));
    }
    Ok(())
}

fn assert_evidence(
    run: &mut SourceRun,
    layout: &LoopLayout,
    values: &[SlotValue],
    taint: Taint,
) -> Result<(), String> {
    let events = run.evidence.drain();
    let mut expected_steps = layout.before_start.to_vec();
    expected_steps.push(layout.start);
    for _ in values {
        expected_steps.extend(layout.body);
    }
    expected_steps.extend(layout.after);
    let started = events
        .iter()
        .filter_map(|e| match e {
            EvidenceEvent::StepStarted { step } => Some(*step),
            _ => None,
        })
        .collect::<Vec<_>>();
    let succeeded = events
        .iter()
        .filter_map(|e| match e {
            EvidenceEvent::StepSucceeded { step, .. } => Some(*step),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(started, expected_steps);
    assert_eq!(succeeded, expected_steps);
    let writes = events
        .iter()
        .filter_map(|e| match e {
            EvidenceEvent::SlotWritten {
                slot, value, taint, ..
            } if *slot == layout.first || *slot == layout.second => Some((*slot, *value, *taint)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut expected_writes = Vec::new();
    for value in values {
        expected_writes.push((layout.first, *value, taint));
        expected_writes.push((layout.second, *value, taint));
    }
    assert_eq!(writes, expected_writes);
    let output = *run
        .frame
        .read_slot(layout.output)
        .map_err(|e| format!("{e:?}"))?;
    let published = events
        .iter()
        .filter_map(|e| match e {
            EvidenceEvent::SlotWritten {
                slot, value, taint, ..
            } if *slot == layout.output => Some((*value, *taint)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(published, vec![(output, taint)]);
    Ok(())
}

#[test]
fn foreach_empty_max3() -> Result<(), String> {
    let values: Vec<SlotValue> = vec![];
    for taint in [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret] {
        assert_case(3, &values, taint)?;
    }
    Ok(())
}

#[test]
fn foreach_one_max3() -> Result<(), String> {
    let values = vec![SlotValue::I64(-7)];
    for taint in [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret] {
        assert_case(3, &values, taint)?;
    }
    Ok(())
}

#[test]
fn foreach_three_mixed_max3() -> Result<(), String> {
    let values = vec![SlotValue::I64(-7), SlotValue::Bool(true), SlotValue::Null];
    for taint in [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret] {
        assert_case(3, &values, taint)?;
    }
    Ok(())
}

#[test]
fn foreach_empty_max0() -> Result<(), String> {
    let values: Vec<SlotValue> = vec![];
    for taint in [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret] {
        assert_case(0, &values, taint)?;
    }
    Ok(())
}
