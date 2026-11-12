#[path = "vb_f2_signal_body_results/support.rs"]
mod support;
use support::*;
use vb_core::{CompiledNodeKind as K, SlotValue};

fn workflow(body: &str, check: &str) -> String {
    let body = body
        .lines()
        .map(|line| format!("{}\n", line.strip_prefix("  ").unwrap_or(line)))
        .collect::<String>();
    let body = format!("{body}- id: checked\n  set: {{output: checked, value: '{check}'}}\n");
    source("vars: {xs: [3, 1, 2]}\n", &body, "$step.checked")
}

#[test]
fn ordered_results_publish_a_real_step_value() -> TestResult {
    let yaml = "version: velvet-ballistics/v1\nname: ordered_declared\nwhen: {manual: {}}\ninputs: {items: {is: list, of: number, max: 3}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      steps:\n        - id: projected\n          set: {output: projected, value: '$entry'}\n  - id: done\n    finish: {result: '$step.loop'}\n";
    let plan = vb_compile::compile_workflow(yaml.as_bytes())?;
    assert!(
        plan.to_parts()
            .nodes
            .iter()
            .any(|n| matches!(n.kind, K::ForEachJoin { .. }))
    );
    Ok(())
}

#[test]
fn named_lists_preserve_order_and_original_source_at_runtime() -> TestResult {
    let yaml = workflow(
        "  - id: original\n    set: {output: source, value: '$vars.xs'}\n  - id: loop\n    for_each:\n      variable: entry\n      input: $source\n      steps:\n        - id: scratch\n          set: {output: scratch, value: '100'}\n        - id: projected\n          set: {output: projected, value: '$entry'}\n",
        "count($step.loop) == 3 and $step.loop.0 == 3 and $step.loop.1 == 1 and $step.loop.2 == 2 and $source.0 == 3 and $source.1 == 1 and $source.2 == 2",
    );
    for quantum in [1, 64] {
        let outcome = execute(&yaml, quantum, None)?;
        assert_eq!(outcome.value, SlotValue::Bool(true));
        assert_eq!(
            outcome
                .events
                .iter()
                .filter(|event| matches!(
                    event,
                    vb_runtime::journal::RuntimeJournalEvent::RunFinished { .. }
                ))
                .count(),
            1
        );
        assert_eq!(outcome.asks, 0);
        assert_eq!(outcome.waits, 0);
    }
    Ok(())
}

#[test]
fn positive_at_once_is_a_serial_upper_bound() -> TestResult {
    for cap in [1, 2, u32::MAX] {
        let body = format!(
            "  - id: original\n    set: {{output: source, value: '$vars.xs'}}\n  - id: loop\n    for_each:\n      variable: entry\n      input: $source\n      at_once: {cap}\n      steps:\n        - id: projected\n          set: {{output: projected, value: '$entry'}}\n"
        );
        for quantum in [1, 64] {
            let outcome = execute(
                &workflow(
                    &body,
                    "count($step.loop) == 3 and $step.loop.0 == 3 and $step.loop.1 == 1 and $step.loop.2 == 2",
                ),
                quantum,
                None,
            )?;
            assert_eq!(outcome.value, SlotValue::Bool(true));
        }
    }
    Ok(())
}

#[test]
fn declared_positive_at_once_compiles() -> TestResult {
    let yaml = "version: velvet-ballistics/v1\nname: declared_cap\nwhen: {manual: {}}\ninputs: {items: {is: list, of: number, max: 2}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      at_once: 2\n      steps:\n        - id: projected\n          set: {output: projected, value: '$entry'}\n  - id: done\n    finish: {result: '$input.items'}\n";
    vb_compile::compile_workflow(yaml.as_bytes())?;
    Ok(())
}

#[test]
fn empty_sources_and_empty_or_nonproducing_bodies_have_exact_results() -> TestResult {
    for (values, body, expected) in [
        (
            "[]",
            "      steps:\n        - id: projected\n          set: {output: projected, value: '7'}\n",
            "count($step.loop) == 0",
        ),
        (
            "[3, 1, 2]",
            "      steps: []\n",
            "count($step.loop) == 3 and $step.loop.0 == null and $step.loop.1 == null and $step.loop.2 == null",
        ),
        (
            "[3, 1, 2]",
            "      steps:\n        - id: pause\n          if: 'false'\n          wait: {timeout: '$delay'}\n",
            "count($step.loop) == 3 and $step.loop.0 == null and $step.loop.1 == null and $step.loop.2 == null",
        ),
    ] {
        let yaml = workflow(
            &format!(
                "  - id: loop\n    for_each:\n      variable: entry\n      input: $vars.xs\n{body}"
            ),
            expected,
        )
        .replace("xs: [3, 1, 2]", &format!("xs: {values}"));
        for quantum in [1, 64] {
            let outcome = execute(&yaml, quantum, None)?;
            assert_eq!(outcome.value, SlotValue::Bool(true));
            assert_eq!(outcome.waits, 0);
        }
    }
    Ok(())
}

#[test]
fn null_and_list_body_values_are_neither_filtered_nor_flattened() -> TestResult {
    for (value, check) in [
        (
            "null",
            "count($step.loop) == 3 and $step.loop.0 == null and $step.loop.2 == null",
        ),
        (
            "$vars.xs",
            "count($step.loop) == 3 and count($step.loop.0) == 3 and $step.loop.0.0 == 3 and $step.loop.2.2 == 2",
        ),
    ] {
        let yaml = workflow(
            &format!(
                "  - id: loop\n    for_each:\n      variable: entry\n      input: $vars.xs\n      steps:\n        - id: projected\n          set: {{output: projected, value: '{value}'}}\n"
            ),
            check,
        );
        for quantum in [1, 64] {
            assert_eq!(execute(&yaml, quantum, None)?.value, SlotValue::Bool(true));
        }
    }
    Ok(())
}

#[test]
fn nested_iteration_variables_and_accessor_sources_can_hold_lists() -> TestResult {
    let yaml = workflow(
        "  - id: loop\n    for_each:\n      variable: row\n      input: $vars.rows\n      steps:\n        - id: inner\n          for_each:\n            variable: cell\n            input: $row\n            steps:\n              - id: projected\n                set: {output: projected, value: '$cell'}\n",
        "count($step.loop) == 2 and count($step.loop.0) == 2 and $step.loop.0.0 == 4 and $step.loop.0.1 == 1 and count($step.loop.1) == 1 and $step.loop.1.0 == 9 and $vars.rows.0.1 == 1",
    ).replace("vars: {xs: [3, 1, 2]}", "vars: {xs: [3, 1, 2], rows: [[4, 1], [9]]}");
    let accessor = workflow(
        "  - id: loop\n    for_each:\n      variable: entry\n      input: $vars.config.values\n      steps:\n        - id: projected\n          set: {output: projected, value: '$entry'}\n",
        "count($step.loop) == 2 and $step.loop.0 == 6 and $step.loop.1 == 2 and $vars.config.values.0 == 6",
    ).replace("vars: {xs: [3, 1, 2]}", "vars: {xs: [3, 1, 2], config: {values: [6, 2]}}");
    for source in [&yaml, &accessor] {
        for quantum in [1, 64] {
            assert_eq!(execute(source, quantum, None)?.value, SlotValue::Bool(true));
        }
    }
    Ok(())
}

#[test]
fn a_prior_aggregate_is_a_list_source_and_a_later_body_producer_wins() -> TestResult {
    let yaml = workflow(
        "  - id: first\n    for_each:\n      variable: entry\n      input: $vars.xs\n      steps:\n        - id: projected\n          set: {output: projected, value: '$entry'}\n  - id: loop\n    for_each:\n      variable: next_entry\n      input: $step.first\n      steps:\n        - id: previous\n          set: {output: previous, value: '$next_entry'}\n        - id: last\n          set: {output: last_value, value: '7'}\n",
        "count($step.loop) == 3 and $step.loop.0 == 7 and $step.loop.2 == 7 and $step.first.0 == 3 and $step.first.2 == 2",
    );
    for quantum in [1, 64] {
        assert_eq!(execute(&yaml, quantum, None)?.value, SlotValue::Bool(true));
    }
    Ok(())
}

#[test]
fn conditional_last_producer_uses_the_last_executed_value_each_iteration() -> TestResult {
    for prefix in [
        "",
        "        - id: previous\n          set: {output: previous, value: '7'}\n",
    ] {
        let yaml = workflow(
            &format!(
                "  - id: loop\n    for_each:\n      variable: entry\n      input: $vars.xs\n      steps:\n{prefix}        - id: projected\n          if: '$entry == 1'\n          set: {{output: projected, value: '$entry'}}\n"
            ),
            if prefix.is_empty() {
                "count($step.loop) == 3 and $step.loop.0 == null and $step.loop.1 == 1 and $step.loop.2 == null"
            } else {
                "count($step.loop) == 3 and $step.loop.0 == 7 and $step.loop.1 == 1 and $step.loop.2 == 7"
            },
        );
        for quantum in [1, 64] {
            assert_eq!(execute(&yaml, quantum, None)?.value, SlotValue::Bool(true));
        }
    }
    Ok(())
}

#[test]
fn reduce_rejects_a_for_each_list_as_its_numeric_accumulator() -> TestResult {
    for prefix in [
        "",
        "        - id: previous\n          set: {output: previous, value: '7'}\n",
    ] {
        let yaml = workflow(
            &format!(
                "  - id: fold\n    reduce:\n      variable: accumulator\n      input: $vars.xs\n      initial: '0'\n      steps:\n{prefix}        - id: list_tail\n          for_each:\n            variable: entry\n            input: $vars.xs\n            steps:\n              - id: projected\n                set: {{output: projected, value: '$entry'}}\n"
            ),
            "true",
        );
        assert_eq!(
            vb_compile::compile_workflow(yaml.as_bytes()),
            Err(vb_compile::CompileErrors(vec![
                vb_compile::CompileError::StepFieldShape {
                    step: 2,
                    field: "reduce.steps",
                    expected: "a set value preserving the accumulator type",
                }
            ]))
        );
    }
    Ok(())
}
