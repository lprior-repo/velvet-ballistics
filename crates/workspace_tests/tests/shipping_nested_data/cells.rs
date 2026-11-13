//! Per-cell owned shipping assertions over the frozen nested matrix.

use vb_core::SlotValue;

use crate::flow::{one_over_refusal, run_cell};
use crate::support::TestResult;

const OUTER_SUPPORTED: [&str; 2] = ["for_each", "reduce"];
const OUTER_ALL: [&str; 3] = ["for_each", "collect", "reduce"];
const INNER_SUPPORTED: [&str; 5] = ["repeat", "for_each", "reduce", "together", "choose"];
const INNER_ALL: [&str; 6] = [
    "repeat", "for_each", "collect", "reduce", "together", "choose",
];

fn outer_multiplicity(family: &str) -> usize {
    match family {
        "for_each" | "collect" | "reduce" => 2,
        _ => 1,
    }
}

fn inner_multiplicity(family: &str) -> usize {
    match family {
        "repeat" | "for_each" | "collect" | "reduce" => 2,
        _ => 1,
    }
}

fn expected_terminal(outer: &str) -> (Option<SlotValue>, Option<Vec<SlotValue>>) {
    match outer {
        "reduce" => (Some(SlotValue::I64(3)), None),
        "collect" => (None, Some(vec![SlotValue::I64(3), SlotValue::I64(5)])),
        _ => (None, Some(vec![SlotValue::I64(3), SlotValue::I64(3)])),
    }
}

#[test]
fn supported_owned_cells_recover_with_exact_evidence() -> TestResult {
    let mut failures = Vec::new();
    let mut checked = 0usize;
    for outer in OUTER_SUPPORTED {
        for inner in INNER_SUPPORTED {
            checked = checked.checked_add(1).ok_or("cell count overflow")?;
            match run_cell(outer, inner) {
                Ok(observed) => {
                    let expected_leaf = outer_multiplicity(outer)
                        .checked_mul(inner_multiplicity(inner))
                        .ok_or("multiplicity overflow")?;
                    if observed.leaf_successes != expected_leaf {
                        failures.push(format!(
                            "{outer}/{inner}: leaf successes {} != {expected_leaf}",
                            observed.leaf_successes
                        ));
                    }
                    if observed.cut_completed != 0 {
                        failures.push(format!("{outer}/{inner}: cut was terminal"));
                    }
                    if !observed.cut_replay_selected || !observed.full_replay_selected {
                        failures.push(format!("{outer}/{inner}: owned replay not selected"));
                    }
                    if observed.terminal_taint != vb_core::Taint::Secret {
                        failures.push(format!(
                            "{outer}/{inner}: terminal taint {:?} != Secret",
                            observed.terminal_taint
                        ));
                    }
                    let (expected_value, expected_items) = expected_terminal(outer);
                    match (&expected_value, &observed.terminal_value) {
                        (Some(SlotValue::I64(expected)), SlotValue::I64(actual))
                            if expected == actual => {}
                        (None, SlotValue::List(_)) => {}
                        (expected, actual) => failures.push(format!(
                            "{outer}/{inner}: terminal {actual:?} != {expected:?}"
                        )),
                    }
                    if observed.terminal_items != expected_items {
                        failures.push(format!(
                            "{outer}/{inner}: materialized {:?} != {expected_items:?}",
                            observed.terminal_items
                        ));
                    }
                    if observed.leaf_terminal != Some((SlotValue::I64(9), vb_core::Taint::Clean)) {
                        failures.push(format!(
                            "{outer}/{inner}: leaf terminal {:?}",
                            observed.leaf_terminal
                        ));
                    }
                }
                Err(error) => failures.push(format!("{outer}/{inner}: {error:?} / {error}")),
            }
        }
    }
    if checked != 10 {
        failures.push(format!("checked {checked} cells, expected 10"));
    }
    assert!(
        failures.is_empty(),
        "owned cells failed:\n{}",
        failures.join("\n")
    );
    Ok(())
}

#[test]
fn collect_involving_owned_cells_recover_with_exact_evidence() -> TestResult {
    let mut results = Vec::new();
    let mut failures = Vec::new();
    let mut checked = 0usize;
    for outer in OUTER_ALL {
        for inner in INNER_ALL {
            if outer != "collect" && inner != "collect" {
                continue;
            }
            checked = checked.checked_add(1).ok_or("cell count overflow")?;
            match run_cell(outer, inner) {
                Ok(observed) => {
                    let expected_leaf = outer_multiplicity(outer)
                        .checked_mul(inner_multiplicity(inner))
                        .ok_or("multiplicity overflow")?;
                    if observed.leaf_successes != expected_leaf {
                        failures.push(format!(
                            "{outer}/{inner}: leaf successes {} != {expected_leaf}",
                            observed.leaf_successes
                        ));
                    }
                    if observed.cut_completed != 0 {
                        failures.push(format!("{outer}/{inner}: cut was terminal"));
                    }
                    if !observed.cut_replay_selected || !observed.full_replay_selected {
                        failures.push(format!("{outer}/{inner}: owned replay not selected"));
                    }
                    if observed.terminal_taint != vb_core::Taint::Secret {
                        failures.push(format!(
                            "{outer}/{inner}: terminal taint {:?} != Secret",
                            observed.terminal_taint
                        ));
                    }
                    let (expected_value, expected_items) = expected_terminal(outer);
                    match (&expected_value, &observed.terminal_value) {
                        (Some(SlotValue::I64(expected)), SlotValue::I64(actual))
                            if expected == actual => {}
                        (None, SlotValue::List(_)) => {}
                        (expected, actual) => failures.push(format!(
                            "{outer}/{inner}: terminal {actual:?} != {expected:?}"
                        )),
                    }
                    if observed.terminal_items != expected_items {
                        failures.push(format!(
                            "{outer}/{inner}: materialized {:?} != {expected_items:?}",
                            observed.terminal_items
                        ));
                    }
                    if observed.leaf_terminal != Some((SlotValue::I64(9), vb_core::Taint::Clean)) {
                        failures.push(format!(
                            "{outer}/{inner}: leaf terminal {:?}",
                            observed.leaf_terminal
                        ));
                    }
                    results.push(format!(
                        "GREEN {outer}/{inner}: replay cut={} full={} terminal={:?}/{:?} leaf={}",
                        observed.cut_replay_selected,
                        observed.full_replay_selected,
                        observed.terminal_value,
                        observed.terminal_taint,
                        observed.leaf_successes
                    ));
                }
                Err(error) => {
                    let detail = format!("RED {outer}/{inner}: {error:?} / {error}");
                    results.push(detail.clone());
                    failures.push(detail);
                }
            }
        }
    }
    eprintln!("{}", results.join("\n"));
    if checked != 8 {
        failures.push(format!("checked {checked} cells, expected 8"));
    }
    assert!(
        failures.is_empty(),
        "Collect owned replay qualification failed:\n{}",
        failures.join("\n")
    );
    Ok(())
}

#[test]
fn one_over_input_list_is_refused_for_each_outer() -> TestResult {
    let mut failures = Vec::new();
    for outer in OUTER_ALL {
        match one_over_refusal(outer, "choose") {
            Ok(Some(stage)) if stage.starts_with("bundle:") && stage.contains("Bounds") => {
                eprintln!("{outer}: one-over refused at {stage}");
            }
            Ok(Some(stage)) => failures.push(format!("{outer}: wrong refusal stage {stage}")),
            Ok(None) => failures.push(format!("{outer}: one-over input list was accepted")),
            Err(error) => failures.push(format!("{outer}: probe failed: {error}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}

#[test]
fn control_variables_do_not_escape_lexical_scope() -> TestResult {
    let outer_variables = [
        ("for_each", "item_outer"),
        ("collect", "page_outer"),
        ("reduce", "accumulator_outer"),
    ];
    let mut failures = Vec::new();
    for (outer, outer_variable) in outer_variables {
        let source = crate::support::nested_source(outer, "for_each")?;
        for (label, variable) in [("inner", "item_inner"), ("outer", outer_variable)] {
            let leaked = source.replace(
                "  - id: done\n",
                &format!(
                    "  - id: leak\n    set: {{output: leak, value: '${variable}'}}\n  - id: done\n"
                ),
            );
            if leaked == source {
                failures.push(format!("{outer}: source rewrite for {label} failed"));
                continue;
            }
            if vb_compile::compile_workflow(leaked.as_bytes()).is_ok() {
                failures.push(format!("{outer}/{label}: ${variable} escaped scope"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}
