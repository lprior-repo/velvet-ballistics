#[path = "vb_f2_signal_body_results/support.rs"]
mod support;
use support::*;
use vb_core::{CompiledNodeKind as K, SlotValue};
use vb_runtime::journal::RuntimeJournalEvent as E;

#[test]
fn asks_suspend_each_item_until_its_answer_before_advancing() -> TestResult {
    for cap in [1, 2, u32::MAX] {
        let body = format!(
            "- id: loop\n  for_each:\n    variable: entry\n    input: $vars.xs\n    at_once: {cap}\n    steps:\n      - id: question\n        ask: {{prompt: '$entry'}}\n- id: checked\n  set: {{output: checked, value: 'count($step.loop) == 3 and $step.loop.0 == 73 and $step.loop.1 == 74 and $step.loop.2 == 75'}}\n"
        );
        let yaml = source("vars: {xs: [3, 1, 2]}\n", &body, "$step.checked");
        for quantum in [1, 64] {
            let outcome = execute(&yaml, quantum, None)?;
            assert_eq!(outcome.value, SlotValue::Bool(true));
            assert_eq!(outcome.asks, 3);
            assert_eq!(outcome.waits, 0);
            let boundaries = outcome
                .events
                .iter()
                .filter_map(|event| match event {
                    E::AskScheduled { .. } => Some("scheduled"),
                    E::AskAnswered { .. } => Some("answered"),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                boundaries,
                [
                    "scheduled",
                    "answered",
                    "scheduled",
                    "answered",
                    "scheduled",
                    "answered"
                ]
            );
        }
    }
    Ok(())
}

#[test]
fn error_handler_then_and_skipped_loop_do_not_publish_a_false_aggregate() -> TestResult {
    let body = "- id: loop\n  for_each:\n    variable: entry\n    input: $input.payload\n    steps:\n      - id: projected\n        set: {output: projected, value: '$entry'}\n  on_error: {handler: fallback}\n  then: done\n- id: fallback\n  set: {output: recovered, value: '77'}\n";
    for (declarations, body, dynamic, failed, fallback_runs) in [
        (
            "inputs: {payload: {is: any}}\n",
            body.to_owned(),
            Some(SlotValue::I64(9)),
            true,
            1,
        ),
        (
            "vars: {xs: [3, 1, 2]}\n",
            body.replace("$input.payload", "$vars.xs"),
            None,
            false,
            0,
        ),
        (
            "inputs: {payload: {is: any}}\n",
            body.replace("  on_error:", "  if: 'false'\n  on_error:"),
            Some(SlotValue::I64(9)),
            false,
            1,
        ),
    ] {
        let yaml = source(declarations, &body, "$saved");
        let plan = vb_compile::compile_workflow(yaml.as_bytes())?;
        let result = plan
            .to_parts()
            .nodes
            .iter()
            .find(|node| matches!(node.kind, K::ForEachJoin { .. }))
            .and_then(|node| node.output.slot())
            .ok_or("aggregate result slot")?;
        for quantum in [1, 64] {
            let outcome = execute(&yaml, quantum, dynamic)?;
            assert_eq!(outcome.value, SlotValue::I64(42));
            assert_eq!(outcome.asks, 0);
            assert_eq!(outcome.waits, 0);
            let failures = outcome
                .events
                .iter()
                .filter(|event| matches!(event, E::StepFailed { .. }))
                .count();
            assert_eq!(failures, usize::from(failed));
            let recovery_writes = outcome
                .events
                .iter()
                .filter(|event| match event {
                    E::SlotWritten { value, .. } => {
                        postcard::from_bytes::<SlotValue>(value).ok() == Some(SlotValue::I64(77))
                    }
                    _ => false,
                })
                .count();
            assert_eq!(recovery_writes, fallback_runs);
            let results = outcome
                .events
                .iter()
                .filter(|event| matches!(event, E::SlotWritten { slot, .. } if *slot == result))
                .count();
            assert_eq!(results, usize::from(dynamic.is_none()));
        }
    }
    Ok(())
}

#[test]
fn zero_concurrency_and_known_scalar_sources_keep_typed_rejections() -> TestResult {
    for (input, cap, expected) in [
        (
            "$vars.xs",
            0,
            vb_compile::CompileError::StepFieldShape {
                step: 2,
                field: "for_each.at_once",
                expected: "a positive concurrency upper bound",
            },
        ),
        (
            "$seed",
            1,
            vb_compile::CompileError::TypeMismatch {
                field: "for_each.input",
                expected: "list",
                found: "non-list",
            },
        ),
    ] {
        let yaml = source(
            "vars: {xs: [1]}\n",
            &format!(
                "- id: loop\n  for_each:\n    variable: entry\n    input: {input}\n    at_once: {cap}\n    steps: []\n"
            ),
            "$saved",
        );
        assert_eq!(
            vb_compile::compile_workflow(yaml.as_bytes()),
            Err(vb_compile::CompileErrors(vec![expected]))
        );
    }
    Ok(())
}

#[test]
fn named_conditional_outputs_still_require_definite_assignment() -> TestResult {
    for reference in ["$step.guarded", "$picked"] {
        let body = format!(
            "- id: loop\n  for_each:\n    variable: entry\n    input: $input.items\n    steps:\n      - id: previous\n        set: {{output: previous, value: '7'}}\n      - id: guarded\n        if: 'false'\n        set: {{output: picked, value: '$entry'}}\n      - id: illegal_read\n        set: {{output: illegal_read, value: '{reference}'}}\n"
        );
        let yaml = source(
            "inputs: {items: {is: list, of: number, max: 2}}\n",
            &body,
            "$saved",
        );
        let errors = vb_compile::compile_workflow(yaml.as_bytes())
            .err()
            .ok_or("named conditional output was accepted")?;
        let [
            vb_compile::CompileError::Validation(vb_validate::ValidationError::TypeMismatch {
                expected,
                found,
            }),
        ] = errors.0.as_slice()
        else {
            return Err(format!("unexpected diagnostic: {errors:?}").into());
        };
        assert_eq!(expected, "valid compiled slot flow");
        assert!(found.ends_with("UninitializedRead"), "{found}");
    }
    Ok(())
}

#[test]
fn conditional_body_result_resets_across_ask_resume_and_never_initializes_its_name() -> TestResult {
    let body = "- id: loop\n  for_each:\n    variable: entry\n    input: $vars.xs\n    steps:\n      - id: previous\n        set: {output: previous, value: '7'}\n      - id: question\n        if: '$entry == 1'\n        ask: {prompt: '$entry'}\n- id: checked\n  set: {output: checked, value: 'count($step.loop) == 3 and $step.loop.0 == 7 and $step.loop.1 == 73 and $step.loop.2 == 7'}\n";
    let yaml = source("vars: {xs: [3, 1, 2]}\n", body, "$step.checked");
    let plan = vb_compile::compile_workflow(yaml.as_bytes())?;
    let parts = plan.to_parts();
    let answer = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, K::Ask { .. }))
        .and_then(|node| node.output.slot())
        .ok_or("answer")?;
    let private = parts
        .nodes
        .iter()
        .find(|node| match node.kind {
            K::SetConst { value } => {
                parts.constants.get(value.as_usize()) == Some(&vb_core::ConstValue::Null)
            }
            _ => false,
        })
        .and_then(|node| node.output.slot())
        .ok_or("private iteration result")?;
    assert_ne!(answer, private);
    let reset_step = parts
        .nodes
        .iter()
        .find(|node| node.output.slot() == Some(private))
        .ok_or("reset step")?
        .id;
    for quantum in [1, 64] {
        let outcome = execute(&yaml, quantum, None)?;
        assert_eq!(outcome.value, SlotValue::Bool(true));
        assert_eq!((outcome.asks, outcome.waits), (1, 0));
        let mut iteration = 0usize;
        let mut answer_writes = Vec::new();
        for event in &outcome.events {
            match event {
                E::StepStarted { step, .. } if *step == reset_step => {
                    iteration = iteration.checked_add(1).ok_or("iteration")?
                }
                E::SlotWritten {
                    slot, value, taint, ..
                } if *slot == answer => answer_writes.push((
                    iteration,
                    postcard::from_bytes::<SlotValue>(value)?,
                    *taint,
                )),
                _ => {}
            }
        }
        // Admission of the answer and AskResume both retain the actual answer.
        assert_eq!(
            answer_writes,
            [
                (2, SlotValue::I64(73), vb_core::Taint::DerivedFromSecret),
                (2, SlotValue::I64(73), vb_core::Taint::DerivedFromSecret)
            ]
        );
        let resets = outcome
            .events
            .iter()
            .filter(|event| match event {
                E::SlotWritten { slot, value, .. } if *slot == private => {
                    postcard::from_bytes::<SlotValue>(value).ok() == Some(SlotValue::Null)
                }
                _ => false,
            })
            .count();
        assert_eq!(resets, 3);
    }
    Ok(())
}

#[test]
fn choose_and_then_skips_do_not_read_an_unexecuted_last_producer() -> TestResult {
    let body = "- id: loop\n  for_each:\n    variable: entry\n    input: $vars.xs\n    steps:\n      - id: previous\n        set: {output: previous, value: '7'}\n      - id: route\n        choose:\n          branches:\n            - when: '$entry == 1'\n              steps:\n                - id: visited\n                  set: {output: visited, value: '1'}\n          otherwise: tail\n      - id: projected\n        set: {output: projected, value: '$entry'}\n      - id: tail\n        if: 'false'\n        wait: {timeout: '$delay'}\n- id: checked\n  set: {output: checked, value: 'count($step.loop) == 3 and $step.loop.0 == 7 and $step.loop.1 == 1 and $step.loop.2 == 7'}\n";
    let then = body.replace("      - id: route\n        choose:\n          branches:\n            - when: '$entry == 1'\n              steps:\n                - id: visited\n                  set: {output: visited, value: '1'}\n          otherwise: tail\n", "      - id: route\n        if: '$entry != 1'\n        wait: {timeout: '$delay'}\n        then: tail\n");
    for (body, waits) in [(body, 0), (then.as_str(), 2)] {
        for quantum in [1, 64] {
            let outcome = execute(
                &source("vars: {xs: [3, 1, 2]}\n", body, "$step.checked"),
                quantum,
                None,
            )?;
            assert_eq!(outcome.value, SlotValue::Bool(true));
            assert_eq!((outcome.asks, outcome.waits), (0, waits));
        }
    }
    Ok(())
}
