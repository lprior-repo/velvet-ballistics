#![forbid(unsafe_code)]

#[path = "vb_f1_collect/support.rs"]
mod support;

#[path = "vb_f1_collect/action_boundary.rs"]
mod action_boundary;

#[path = "vb_f1_collect/terminal_boundary.rs"]
mod terminal_boundary;

#[path = "vb_f1_collect/replay_terminal.rs"]
mod replay_terminal;

use support::SourceRun;
use vb_core::{SlotValue, Taint};
use vb_runtime::engine::{EvidenceEvent, RuntimeSignal};

const SOURCE: &str = r#"version: velvet-ballistics/v1
name: collect_execution
when:
  manual: {}
inputs:
  entries: { is: list, of: any, max: 8 }
steps:
  - id: bounded
    collect:
      variable: batch
      source: "$input.entries"
      pages: 3
      items: 2
      steps:
        - id: first
          set:
            output: copied
            value: "$batch"
        - id: second
          set:
            output: echoed
            value: "$copied"
  - id: done
    finish:
      result: "$step.bounded"
"#;

fn expected_body_sequence(values: &[SlotValue]) -> Vec<u16> {
    (0..values.chunks(2).len())
        .flat_map(|_| [1u16, 2u16])
        .collect()
}

fn assert_body_events(run: &mut SourceRun, values: &[SlotValue]) -> Result<(), String> {
    let events = run.evidence.drain();
    let body_succeeded: Vec<u16> = events
        .iter()
        .filter_map(|e| match e {
            EvidenceEvent::StepSucceeded { step, .. } if step.get() == 1 || step.get() == 2 => {
                Some(step.get())
            }
            _ => None,
        })
        .collect();
    assert_page_writes(run, &events, values)?;
    let expected = expected_body_sequence(values);
    if body_succeeded != expected {
        return Err(format!(
            "body event sequence mismatch: got {:?}, expected {:?}",
            body_succeeded, expected
        ));
    }
    Ok(())
}

fn assert_page_writes(
    run: &SourceRun,
    events: &[EvidenceEvent],
    values: &[SlotValue],
) -> Result<(), String> {
    let expected: Vec<Vec<SlotValue>> = values.chunks(2).map(|chunk| chunk.to_vec()).collect();

    let steps = [1u16, 2u16];
    for step in steps {
        let slot = run.output(step)?;
        let actual_ids = events.iter().filter_map(|event| {
            if let EvidenceEvent::SlotWritten {
                slot: written,
                value: SlotValue::List(id),
                ..
            } = event
            {
                if *written == slot {
                    return Some(*id);
                }
            }
            None
        });
        let actual: Result<Vec<Vec<SlotValue>>, String> = actual_ids
            .map(|id| {
                run.store
                    .list(id)
                    .map_err(|e| format!("{e:?}"))
                    .map(|v| v.to_vec())
            })
            .collect();
        let actual = actual?;
        if actual != expected {
            return Err(format!(
                "step {}: expected {:?}, got {:?}",
                step, expected, actual
            ));
        }
    }

    Ok(())
}

fn assert_body_output(run: &SourceRun, step: u16, expected: &[SlotValue]) -> Result<(), String> {
    let slot = run.output(step)?;
    let sv = run.frame.read_slot(slot).map_err(|e| format!("{e:?}"))?;
    match sv {
        SlotValue::List(body_id) => {
            let body_list = run.store.list(*body_id).map_err(|e| format!("{e:?}"))?;
            if body_list != expected {
                return Err(format!(
                    "body output at step {} mismatch: got {:?}, expected {:?}",
                    step, body_list, expected
                ));
            }
        }
        other => return Err(format!("body output not list: {other:?}")),
    }
    Ok(())
}

fn run_case(values: &[SlotValue]) -> Result<(), String> {
    let mut run = SourceRun::new(SOURCE, values, Taint::Clean)?;
    let before = run.workflow.to_parts();
    let signal = run.drive(64).map_err(|e| format!("{e:?}"))?;
    let RuntimeSignal::Finished(SlotValue::List(id)) = signal else {
        return Err(format!("unexpected signal: {signal:?}"));
    };
    let actual = run.store.list(id).map_err(|e| format!("{e:?}"))?;
    if actual != values {
        return Err(format!(
            "result mismatch: got {:?}, expected {:?}",
            actual, values
        ));
    }
    assert_body_events(&mut run, values)?;
    let last_page = values
        .chunks(2)
        .last()
        .map(|c| c.to_vec())
        .unwrap_or_default();
    assert_body_output(&run, 0, values)?;
    if !values.is_empty() {
        assert_body_output(&run, 1, &last_page)?;
        assert_body_output(&run, 2, &last_page)?;
    }
    run.assert_input(values, Taint::Clean)?;
    if run.workflow.to_parts() != before {
        return Err("workflow parts changed".to_string());
    }
    Ok(())
}

#[test]
fn empty_list() -> Result<(), String> {
    run_case(&[])
}

#[test]
fn single_page() -> Result<(), String> {
    run_case(&[SlotValue::I64(1), SlotValue::I64(2)])
}

#[test]
fn multiple_pages_all_aggregated() -> Result<(), String> {
    run_case(&[
        SlotValue::I64(1),
        SlotValue::I64(2),
        SlotValue::I64(3),
        SlotValue::I64(4),
        SlotValue::I64(5),
    ])
}

#[test]
fn duplicates_cross_page() -> Result<(), String> {
    run_case(&[
        SlotValue::I64(7),
        SlotValue::I64(1),
        SlotValue::I64(7),
        SlotValue::I64(7),
        SlotValue::I64(1),
    ])
}

#[test]
fn collect_item_limit_exceeded() -> Result<(), String> {
    let values = vec![
        SlotValue::I64(1),
        SlotValue::I64(2),
        SlotValue::I64(3),
        SlotValue::I64(4),
        SlotValue::I64(5),
        SlotValue::I64(6),
        SlotValue::I64(7),
    ];
    let mut run = SourceRun::new(SOURCE, &values, Taint::Clean)?;
    let before = run.workflow.to_parts();
    let result = run.drive(64);
    assert!(
        matches!(
            result,
            Err(vb_runtime::engine::RuntimeEngineError::Core(
                vb_core::EngineError::CollectItemLimitExceeded
            ))
        ),
        "expected CollectItemLimitExceeded error"
    );
    let events = run.evidence.drain();
    let body_succeeded: Vec<u16> = events
        .iter()
        .filter_map(|e| match e {
            EvidenceEvent::StepSucceeded { step, .. } if step.get() == 1 || step.get() == 2 => {
                Some(step.get())
            }
            _ => None,
        })
        .collect();
    assert!(
        body_succeeded.is_empty(),
        "no collect body steps should complete when limit exceeded, got {:?}",
        body_succeeded
    );
    run.assert_input(&values, Taint::Clean)?;
    assert_eq!(run.workflow.to_parts(), before);
    Ok(())
}

#[test]
fn zero_budget_resumes_collect() -> Result<(), String> {
    let values = vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)];
    let mut run = SourceRun::new(SOURCE, &values, Taint::Clean)?;
    let before = run.workflow.to_parts();

    let signal = run.drive(0).map_err(|e| format!("{e:?}"))?;
    match signal {
        vb_runtime::engine::RuntimeSignal::StepBudgetExhausted => {}
        other => return Err(format!("expected StepBudgetExhausted, got {:?}", other)),
    }
    let zero_events = run.evidence.drain();
    assert_eq!(zero_events.len(), 0, "zero budget must emit no events");
    run.assert_input(&values, Taint::Clean)?;
    let after = run.workflow.to_parts();
    assert_eq!(before, after);

    let signal = run.drive(64).map_err(|e| format!("{e:?}"))?;
    let id = match signal {
        vb_runtime::engine::RuntimeSignal::Finished(SlotValue::List(id)) => id,
        other => return Err(format!("expected Finished(List), got {:?}", other)),
    };
    let collected = run.store.list(id).map_err(|e| format!("{e:?}"))?;
    assert_eq!(collected, values.as_slice());

    assert_body_events(&mut run, &values)?;
    assert_body_output(&run, 0, &values)?;
    assert_body_output(&run, 1, &[SlotValue::I64(3)])?;
    assert_body_output(&run, 2, &[SlotValue::I64(3)])?;

    run.assert_input(&values, Taint::Clean)?;
    let after2 = run.workflow.to_parts();
    assert_eq!(before, after2);
    Ok(())
}
