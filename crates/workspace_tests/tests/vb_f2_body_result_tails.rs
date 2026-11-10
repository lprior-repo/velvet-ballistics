#[path = "vb_f2_signal_body_results/support.rs"]
mod support;
use support::*;
use vb_core::SlotValue;
#[path = "vb_f2_signal_body_results/owned.rs"]
mod owned;
fn repeated(body: &str) -> String {
    format!(
        "- id: scope\n  repeat:\n    max_attempts: 2\n    steps:\n{}",
        indent(body, 6)
    )
}
#[test]
fn repeat_retains_last_lexical_value_through_wait_and_choose_tails() -> TestResult {
    for tail in [
        "- id: paused\n  wait: {timeout: '$delay'}\n",
        "- id: branch\n  choose:\n    branches:\n      - when: 'true'\n        steps:\n          - id: inside\n            set: {output: inside, value: '7'}\n",
    ] {
        let body = format!("- id: value\n  set: {{output: value, value: '9'}}\n{tail}");
        let yaml = source("", &repeated(&body), "$step.scope");
        for quantum in [1, 64] {
            let outcome = execute(&yaml, quantum, None)?;
            assert!(!outcome.events.iter().any(|event| matches!(
                event,
                vb_runtime::journal::RuntimeJournalEvent::StepFailed { .. }
            )));
            assert_eq!(outcome.value, SlotValue::I64(9));
            assert_eq!(outcome.waits, if tail.contains("wait:") { 2 } else { 0 });
        }
    }
    Ok(())
}
#[test]
fn value_less_repeat_and_together_branches_materialize_null() -> TestResult {
    for body in [
        "- id: paused\n  wait: {timeout: '$delay'}\n",
        "- id: branch\n  choose:\n    branches:\n      - when: 'true'\n        steps:\n          - id: inside\n            set: {output: inside, value: '7'}\n",
    ] {
        let yaml = source("", &repeated(body), "$step.scope");
        for quantum in [1, 64] {
            assert_eq!(execute(&yaml, quantum, None)?.value, SlotValue::Null);
            let together = format!(
                "- id: group\n  together:\n    branches:\n      - label: one\n        steps:\n{}- id: observed\n  set: {{output: observed, value: '$step.group.0 == null'}}\n",
                indent(body, 10)
            );
            assert_eq!(
                execute(&source("", &together, "$step.observed"), quantum, None)?.value,
                SlotValue::Bool(true)
            );
        }
    }
    Ok(())
}
#[test]
fn final_ask_result_is_answered_once_per_repeat_iteration() -> TestResult {
    let yaml = source(
        "",
        &repeated("- id: question\n  ask: {prompt: '$seed'}\n"),
        "$step.scope",
    );
    let plan = vb_compile::compile_workflow(yaml.as_bytes())?;
    let parts = plan.to_parts();
    let answer = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, vb_core::CompiledNodeKind::Ask { .. }))
        .and_then(|node| node.output.slot())
        .ok_or("answer")?;
    let materializer = parts.nodes.iter().find(|node| matches!(node.kind, vb_core::CompiledNodeKind::Copy { source } if source == answer)).ok_or("materializer")?;
    let result = materializer.output.slot().ok_or("materialized result")?;
    for quantum in [1, 64] {
        let outcome = execute(&yaml, quantum, None)?;
        let mut writing_result = false;
        let writes = outcome
            .events
            .iter()
            .filter_map(|event| match event {
                vb_runtime::journal::RuntimeJournalEvent::StepStarted { step, .. } => {
                    writing_result = *step == materializer.id;
                    None
                }
                vb_runtime::journal::RuntimeJournalEvent::StepSucceeded { .. } => {
                    writing_result = false;
                    None
                }
                vb_runtime::journal::RuntimeJournalEvent::SlotWritten {
                    slot,
                    value,
                    taint,
                    ..
                } if writing_result && *slot == result => {
                    Some(postcard::from_bytes::<SlotValue>(value).map(|value| (value, *taint)))
                }
                _ => None,
            })
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(
            writes,
            vec![
                (SlotValue::I64(73), vb_core::Taint::DerivedFromSecret),
                (SlotValue::I64(74), vb_core::Taint::Clean)
            ]
        );
        assert_eq!(outcome.value, SlotValue::I64(74));
        assert_eq!((outcome.asks, outcome.waits), (2, 0));
    }
    Ok(())
}

#[test]
fn sibling_body_emitters_accept_nonproducing_tails_without_changing_their_results() -> TestResult {
    for (primitive, result, expected) in [
        (
            "for_each:\n  variable: entry_value\n  input: $input.items\n  at_once: 1",
            "$seed",
            42,
        ),
        (
            "collect:\n  variable: batch_value\n  source: $input.items\n  pages: 2\n  items: 1",
            "$seed",
            42,
        ),
        (
            "reduce:\n  variable: accumulator\n  input: $input.items\n  initial: '0'",
            "$step.scope",
            2,
        ),
    ] {
        let value = if primitive.starts_with("reduce:") {
            "$accumulator + 1"
        } else {
            "9"
        };
        let body = format!(
            "- id: value\n  set: {{output: local, value: '{value}'}}\n- id: paused\n  wait: {{timeout: '$delay'}}\n"
        );
        let group = format!(
            "- id: scope\n{}    steps:\n{}",
            indent(primitive, 2),
            indent(&body, 6)
        );
        let yaml = source(
            "inputs: {items: {is: list, of: number, max: 2}}\n",
            &group,
            result,
        );
        for quantum in [1, 64] {
            assert_eq!(
                owned::execute_owned(&yaml, quantum)?,
                SlotValue::I64(expected),
                "{primitive}"
            );
        }
    }
    let group = "- id: group\n  together:\n    branches:\n      - label: one\n        steps:\n          - id: value\n            set: {output: local, value: '9'}\n          - id: paused\n            wait: {timeout: '$delay'}\n- id: observed\n  set: {output: observed, value: '$step.group.0'}\n";
    for quantum in [1, 64] {
        let outcome = execute(&source("", group, "$step.observed"), quantum, None)?;
        assert_eq!(outcome.value, SlotValue::I64(9));
        assert_eq!(outcome.waits, 1);
    }
    Ok(())
}
