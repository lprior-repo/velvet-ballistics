#[path = "vb_f2_signal_body_results/support.rs"]
mod support;
use support::*;
use vb_core::{CompiledNodeKind as K, ExprOp, SlotValue};
use vb_runtime::journal::RuntimeJournalEvent as E;

fn handled_source(previous: bool, tail: &str, extra: &str, check: &str) -> String {
    let previous = if previous {
        "      - id: previous\n        set: {output: previous_value, value: '7'}\n"
    } else {
        ""
    };
    let body = format!(
        "- id: loop\n  for_each:\n    variable: entry\n    input: $vars.xs\n    steps:\n{previous}      - id: projected\n        set: {{output: projected_value, value: '10 / $entry'}}\n        on_error: {{handler: tail}}\n{tail}{extra}- id: checked\n  set: {{output: checked, value: '{check}'}}\n"
    );
    source("vars: {xs: [1, 0, 2]}\n", &body, "$step.checked")
}

const WAIT: &str = "      - id: tail\n        if: 'false'\n        wait: {timeout: '$delay'}\n";
const CHOOSE: &str = "      - id: tail\n        choose:\n          branches:\n            - when: 'true'\n              steps:\n                - id: not_visited\n                  if: 'false'\n                  wait: {timeout: '$delay'}\n";
const FINISH: &str = "      - id: tail\n        if: 'false'\n        finish: {result: '$saved'}\n";

fn check_handled_results(previous: bool) -> TestResult {
    let middle = if previous { "7" } else { "null" };
    let check = format!(
        "count($step.loop) == 3 and $step.loop.0 == 10 and $step.loop.1 == {middle} and $step.loop.2 == 5 and $vars.xs.0 == 1 and $vars.xs.1 == 0 and $vars.xs.2 == 2"
    );
    for tail in [WAIT, CHOOSE, FINISH] {
        let yaml = handled_source(previous, tail, "", &check);
        let plan = vb_compile::compile_workflow(yaml.as_bytes())?;
        let parts = plan.to_parts();
        let fault = parts
            .nodes
            .iter()
            .find(|node| match node.kind {
                K::EvalExpr { expr } => plan
                    .expression(expr)
                    .is_some_and(|program| program.ops.contains(&ExprOp::Div)),
                _ => false,
            })
            .ok_or("division instruction")?;
        let named = fault.output.slot().ok_or("named result")?;
        for quantum in [1, 64] {
            let outcome = execute(&yaml, quantum, None)?;
            assert_eq!(outcome.value, SlotValue::Bool(true));
            assert_eq!((outcome.asks, outcome.waits), (0, 0));
            let failures = outcome
                .events
                .iter()
                .filter_map(|event| match event {
                    E::StepFailed { step, .. } => Some(*step),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(failures, [fault.id]);
            let named_writes = outcome
                .events
                .iter()
                .filter_map(|event| match event {
                    E::SlotWritten { slot, value, .. } if *slot == named => {
                        Some(postcard::from_bytes::<SlotValue>(value))
                    }
                    _ => None,
                })
                .collect::<Result<Vec<_>, _>>()?;
            assert_eq!(named_writes, [SlotValue::I64(10), SlotValue::I64(5)]);
        }
    }
    Ok(())
}

#[test]
fn handled_last_producer_failure_keeps_the_previous_iteration_body_value() -> TestResult {
    check_handled_results(true)
}

#[test]
fn handled_last_producer_failure_uses_null_and_resets_before_the_next_item() -> TestResult {
    check_handled_results(false)
}

#[test]
fn handled_failure_does_not_definitely_assign_the_named_producer_output() -> TestResult {
    for reference in ["$step.projected", "$projected_value"] {
        let extra = format!(
            "      - id: illegal_read\n        set: {{output: illegal_read, value: '{reference}'}}\n"
        );
        let yaml = handled_source(true, WAIT, &extra, "true");
        let errors = vb_compile::compile_workflow(yaml.as_bytes())
            .err()
            .ok_or("failed named output accepted")?;
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
