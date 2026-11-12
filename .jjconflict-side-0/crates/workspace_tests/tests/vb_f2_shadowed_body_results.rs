#[path = "vb_f2_signal_body_results/support.rs"]
mod support;
use support::*;
use vb_core::{SlotValue, Taint};

#[path = "vb_f2_shadowed_body_results/scopes.rs"]
mod scopes;

const TAILS: [(&str, &str, usize); 5] = [
    (
        "skipped_wait",
        "- id: seed\n  if: 'false'\n  wait: {timeout: '$delay'}\n",
        0,
    ),
    ("wait", "- id: seed\n  wait: {timeout: '$delay'}\n", 1),
    (
        "choose",
        "- id: seed\n  choose:\n    branches:\n      - when: '$step.seed == 99'\n        steps:\n          - id: chosen\n            set: {output: chosen, value: '$step.seed + 1'}\n",
        0,
    ),
    (
        "for_each",
        "- id: seed\n  for_each:\n    variable: shadow_item\n    input: $input.items\n    steps:\n      - id: visited\n        set: {output: visited, value: '$shadow_item'}\n",
        0,
    ),
    (
        "skipped_finish",
        "- id: seed\n  if: 'false'\n  finish: {result: '$step.seed'}\n",
        0,
    ),
];

fn seeded_source(body: &str, result: &str) -> String {
    source(
        if body.contains("$input.items") {
            "inputs: {items: {is: list, of: number, max: 2}}\n"
        } else {
            ""
        },
        body,
        result,
    )
    .replacen("value: '42'", "value: '99'", 1)
}

fn repeated(body: &str, name: &str) -> String {
    format!(
        "- id: {name}\n  repeat:\n    max_attempts: 1\n    steps:\n{}",
        indent(body, 6)
    )
}

#[test]
fn repeat_keeps_last_value_when_skipped_wait_shadows_ancestor_step() -> TestResult {
    let body = "- id: scope\n  repeat:\n    max_attempts: 1\n    steps:\n      - id: value\n        set: {output: value, value: '7'}\n      - id: seed\n        if: 'false'\n        wait: {timeout: '$delay'}\n";
    let yaml = source("", body, "$step.scope").replacen("value: '42'", "value: '99'", 1);
    for quantum in [1, 64] {
        let outcome = execute(&yaml, quantum, None)?;
        assert_eq!(outcome.waits, 0);
        assert_eq!(outcome.asks, 0);
        assert_eq!(outcome.value, SlotValue::I64(7));
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
    }
    Ok(())
}

#[test]
fn runtime_nonproducing_shadows_preserve_value_or_null_and_ancestor_reads() -> TestResult {
    for (name, tail, waits) in TAILS {
        if name == "for_each" {
            continue;
        }
        for produced in [false, true] {
            let body = if produced {
                format!("- id: value\n  set: {{output: value, value: '7'}}\n{tail}")
            } else {
                tail.to_owned()
            };
            for together in [false, true] {
                let (outer, result) = if together {
                    (
                        format!(
                            "- id: scope\n  together:\n    branches:\n      - label: first\n        steps:\n{}",
                            indent(&body, 10)
                        ),
                        "$step.scope.0",
                    )
                } else {
                    (repeated(&body, "scope"), "$step.scope")
                };
                let expected = if produced { "7" } else { "null" };
                let check = format!(
                    "{outer}- id: checked\n  set: {{output: checked, value: '{result} == {expected} and $saved == 99 and $delay == 10'}}\n"
                );
                let yaml = seeded_source(&check, "$step.checked");
                for quantum in [1, 64] {
                    let outcome = execute(&yaml, quantum, None)?;
                    assert_eq!(
                        outcome.value,
                        SlotValue::Bool(true),
                        "{name}/{produced}/{together}/{quantum}"
                    );
                    assert_eq!(outcome.waits, waits);
                    assert_eq!(outcome.asks, 0);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn runtime_shadowing_producer_keeps_its_new_value_and_secret_taint() -> TestResult {
    let body = "- id: seed\n  ask: {prompt: '$saved'}\n- id: delay\n  if: 'false'\n  wait: {timeout: '$delay'}\n";
    let yaml = seeded_source(&repeated(body, "scope"), "$step.scope");
    for quantum in [1, 64] {
        let outcome = execute(&yaml, quantum, None)?;
        assert_eq!(outcome.value, SlotValue::I64(73));
        assert_eq!(outcome.asks, 1);
        assert_eq!(outcome.waits, 0);
        let result = outcome
            .events
            .iter()
            .find_map(|event| match event {
                vb_runtime::journal::RuntimeJournalEvent::RunFinished { result, .. } => {
                    Some(*result)
                }
                _ => None,
            })
            .ok_or("missing result")?;
        let result_taint =
            outcome
                .events
                .iter()
                .rev()
                .find_map(|event| match event {
                    vb_runtime::journal::RuntimeJournalEvent::SlotWritten {
                        slot, taint, ..
                    } if *slot == result => Some(*taint),
                    _ => None,
                })
                .ok_or("missing result taint")?;
        assert_eq!(result_taint, Taint::DerivedFromSecret);
    }
    Ok(())
}

#[test]
fn runtime_shadowing_set_reads_the_ancestor_before_publishing_its_own_value() -> TestResult {
    let body = "- id: seed\n  set: {output: inner_value, value: '$step.seed - 92'}\n- id: delay\n  if: 'false'\n  wait: {timeout: '$delay'}\n";
    let outer = repeated(body, "scope");
    let checked = format!(
        "{outer}- id: checked\n  set: {{output: checked, value: '$step.scope == 7 and $step.seed == 99 and $saved == 99'}}\n"
    );
    let yaml = seeded_source(&checked, "$step.checked");
    for quantum in [1, 64] {
        let outcome = execute(&yaml, quantum, None)?;
        assert_eq!(outcome.value, SlotValue::Bool(true));
        assert_eq!(outcome.waits, 0);
        assert_eq!(outcome.asks, 0);
    }
    Ok(())
}

#[test]
fn runtime_active_shadowing_finish_still_reads_ancestor_and_terminates() -> TestResult {
    let body = "- id: value\n  set: {output: value, value: '7'}\n- id: seed\n  if: 'true'\n  finish: {result: '$step.seed'}\n";
    let yaml = seeded_source(&repeated(body, "scope"), "$step.scope");
    for quantum in [1, 64] {
        let outcome = execute(&yaml, quantum, None)?;
        assert_eq!(outcome.value, SlotValue::I64(99));
        assert_eq!(outcome.waits, 0);
        assert_eq!(outcome.asks, 0);
    }
    Ok(())
}
