//! Matrix aggregation, prohibited-cell, bounds and lexical isolation tests.
//!
//! Results are aggregated per outer family so one red cell cannot hide the
//! other legal cells.

#![forbid(unsafe_code)]

use vb_compile::{CompileError, CompileErrors};
use vb_core::input_validation::InputValueIssue;
use vb_core::root_bundle::{OwnedRootBundle, OwnedRootBundleError};
use vb_core::{SlotValue, StepIdx, Taint, ValueStore};

use crate::cells;
use crate::source::{FAMILIES, compound, indent};
use crate::support::{Cell, TestResult, cell, source_for, workflow_for};

fn aggregate(outer: &'static str) -> TestResult {
    let mut failures: Vec<String> = Vec::new();
    let mut exercised = 0_usize;
    for inner in FAMILIES {
        let current = cell(outer, inner);
        if current.prohibited() {
            continue;
        }
        exercised = exercised.saturating_add(1);
        if let Err(error) = cells::qualify(current) {
            failures.push(format!("{}: {error}", current.label()));
        }
    }
    if exercised != 6 {
        return Err(format!(
            "{outer}: exercised {exercised} legal cells, expected 6"
        ));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("{outer} cells failed:\n{}", failures.join("\n")))
    }
}

#[test]
fn outer_repeat_cells_qualify_owned_recovery() -> TestResult {
    aggregate("repeat")
}

#[test]
fn outer_together_cells_qualify_owned_recovery() -> TestResult {
    let mut failures: Vec<String> = Vec::new();
    let mut exercised = 0_usize;
    for inner in FAMILIES {
        let current = cell("together", inner);
        if current.prohibited() {
            continue;
        }
        exercised = exercised.saturating_add(1);
        if let Err(error) = cells::qualify(current) {
            failures.push(format!("{}: {error}", current.label()));
        }
    }
    if exercised != 5 {
        return Err(format!(
            "together: exercised {exercised} legal cells, expected 5"
        ));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("together cells failed:\n{}", failures.join("\n")))
    }
}

#[test]
fn outer_choose_cells_qualify_owned_recovery() -> TestResult {
    aggregate("choose")
}

#[test]
fn together_within_together_returns_exact_typed_indices() -> TestResult {
    let current = cell("together", "together");
    let source = source_for(current)?;
    let errors = match vb_compile::compile_workflow(source.as_bytes()) {
        Ok(_) => return Err("nested Together source unexpectedly compiled".to_owned()),
        Err(errors) => errors,
    };
    let exact = errors.0.iter().any(|error| {
        matches!(
            error,
            CompileError::Workflow(vb_core::WorkflowError::NestedTogether { outer, inner })
                if *outer == StepIdx::new(2) && *inner == StepIdx::new(4)
        )
    });
    if !exact {
        return Err(format!(
            "expected NestedTogether {{ outer: StepIdx(2), inner: StepIdx(4) }}, got {errors:?}"
        ));
    }
    if errors.0.len() != 1 {
        return Err(format!(
            "nested Together produced {} diagnostics",
            errors.0.len()
        ));
    }
    Ok(())
}

fn repeat_bound_source(max_attempts: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: shipping_nested_bounds\nwhen: {{manual: {{}}}}\nvars: {{sentinel: 1}}\nsteps:\n  - id: r\n    repeat:\n      max_attempts: {max_attempts}\n      steps:\n        - id: leaf\n          set: {{output: leaf, value: '9'}}\n  - id: done\n    finish: {{result: '$vars.sentinel'}}\n"
    )
}

fn compile_errors(source: &str) -> TestResult<CompileErrors> {
    match vb_compile::compile_workflow(source.as_bytes()) {
        Ok(_) => Err("source unexpectedly compiled".to_owned()),
        Err(errors) => Ok(errors),
    }
}

#[test]
fn repeat_max_attempts_bounds_and_one_over_are_exact() -> TestResult {
    let zero = compile_errors(&repeat_bound_source("0"))?;
    if !zero.0.iter().any(|error| {
        matches!(error, CompileError::StepFieldShape { field, .. } if *field == "repeat.max_attempts")
    }) {
        return Err(format!("zero attempts diagnostic: {zero:?}"));
    }
    let over = compile_errors(&repeat_bound_source("65536"))?;
    if !over.0.iter().any(|error| {
        matches!(
            error,
            CompileError::CanonicalYaml { category, message }
                if *category == "field_shape"
                    && message.as_ref() == "field shape error: max_attempts expected u16 integer"
        )
    }) {
        return Err(format!("one-over attempts diagnostic: {over:?}"));
    }
    let at_max = repeat_bound_source("65535");
    let workflow = workflow_for(&at_max)?;
    let repeat = (0..workflow.node_count())
        .map(StepIdx::new)
        .find_map(|step| match workflow.node(step).map(|node| &node.kind) {
            Some(vb_core::CompiledNodeKind::RepeatStart { max_attempts, .. })
                if max_attempts.get() == 65535 =>
            {
                Some(step)
            }
            _ => None,
        })
        .ok_or_else(|| "max_attempts 65535 did not lower".to_owned())?;
    if workflow.step_name(repeat) != Some("r") {
        return Err("max_attempts 65535 lowered at the wrong step".to_owned());
    }
    Ok(())
}

#[test]
fn collect_zero_pages_is_rejected() -> TestResult {
    let source = "version: velvet-ballistics/v1\nname: shipping_nested_collect_bound\nwhen: {manual: {}}\ninputs:\n  items: {is: list, of: any, max: 4}\nsteps:\n  - id: gather\n    collect:\n      variable: batch\n      source: '$input.items'\n      pages: 0\n      items: 1\n      steps:\n        - id: leaf\n          set: {output: leaf, value: '9'}\n  - id: done\n    finish: {result: '1'}\n";
    let errors = compile_errors(source)?;
    if !errors.0.iter().any(
        |error| matches!(error, CompileError::StepFieldShape { field, .. } if *field == "collect"),
    ) {
        return Err(format!("collect zero pages diagnostic: {errors:?}"));
    }
    Ok(())
}

#[test]
fn declared_list_max_accepts_bound_and_rejects_one_over() -> TestResult {
    let source = source_for(cell("repeat", "repeat"))?;
    let workflow = workflow_for(&source)?;
    let slot = workflow
        .input_slots()
        .first()
        .map(|input| input.slot)
        .ok_or_else(|| "missing input slot".to_owned())?;
    let mut at_bound = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let list = at_bound
        .insert_list_with_taint(
            Box::from([
                SlotValue::I64(1),
                SlotValue::I64(2),
                SlotValue::I64(3),
                SlotValue::I64(4),
            ]),
            Box::from([Taint::Clean, Taint::Clean, Taint::Clean, Taint::Clean]),
        )
        .map_err(|error| format!("bound list: {error:?}"))?;
    OwnedRootBundle::new(
        Box::from([(slot, SlotValue::List(list), Taint::Clean)]),
        at_bound.checkpoint_v1(),
        &workflow,
    )
    .map_err(|error| format!("declared max 4 rejected: {error:?}"))?;

    let mut over = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let list = over
        .insert_list_with_taint(
            Box::from([
                SlotValue::I64(1),
                SlotValue::I64(2),
                SlotValue::I64(3),
                SlotValue::I64(4),
                SlotValue::I64(5),
            ]),
            Box::from([
                Taint::Clean,
                Taint::Clean,
                Taint::Clean,
                Taint::Clean,
                Taint::Clean,
            ]),
        )
        .map_err(|error| format!("over list: {error:?}"))?;
    match OwnedRootBundle::new(
        Box::from([(slot, SlotValue::List(list), Taint::Clean)]),
        over.checkpoint_v1(),
        &workflow,
    ) {
        Err(OwnedRootBundleError::RootConstraintMismatch {
            slot: rejected,
            issue: InputValueIssue::Bounds,
        }) if rejected == slot => Ok(()),
        Err(error) => Err(format!("one-over list diagnostic: {error:?}")),
        Ok(_) => Err("declared max 4 accepted 5 items".to_owned()),
    }
}

#[test]
fn inner_output_does_not_escape_any_legal_outer_scope() -> TestResult {
    let mut failures: Vec<String> = Vec::new();
    for outer in ["repeat", "together", "choose"] {
        for inner in FAMILIES {
            let current = cell(outer, inner);
            if current.prohibited() {
                continue;
            }
            let escaped = match escaped_source(current) {
                Ok(source) => source,
                Err(error) => {
                    failures.push(format!("{}: build: {error}", current.label()));
                    continue;
                }
            };
            let errors = match vb_compile::compile_workflow(escaped.as_bytes()) {
                Ok(_) => {
                    failures.push(format!("{}: escaped output compiled", current.label()));
                    continue;
                }
                Err(errors) => errors,
            };
            if !errors.0.iter().any(|error| {
                matches!(error, CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "leaf")
            }) {
                failures.push(format!("{}: wrong escape diagnostic {errors:?}", current.label()));
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "lexical isolation failures:\n{}",
            failures.join("\n")
        ))
    }
}

fn escaped_source(cell: Cell) -> TestResult<String> {
    let inner_body = compound(
        cell.inner,
        "inner",
        "- id: leaf\n  set: {output: leaf, value: '9'}\n",
    )?;
    let outer_body =
        format!("{inner_body}- id: grab\n  set: {{output: grabbed, value: '$leaf'}}\n");
    let outer = compound(cell.outer, "outer", &outer_body)?;
    Ok(format!(
        "version: velvet-ballistics/v1\nname: shipping_nested_escape\nwhen: {{manual: {{}}}}\ninputs:\n  items: {{is: list, of: any, max: 4}}\nvars: {{sentinel: 42}}\nsteps:\n{}  - id: done\n    finish: {{result: '$vars.sentinel'}}\n",
        indent(&outer, 2)
    ))
}

#[test]
fn sibling_repeat_outputs_occupy_distinct_slots() -> TestResult {
    let source = "version: velvet-ballistics/v1\nname: shipping_nested_siblings\nwhen: {manual: {}}\nvars: {sentinel: 1}\nsteps:\n  - id: fan\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: left\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: left_leaf\n                set: {output: left_value, value: '3'}\n        - id: right\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: right_leaf\n                set: {output: right_value, value: '5'}\n  - id: done\n    finish: {result: '$vars.sentinel'}\n";
    let workflow = workflow_for(source)?;
    let left = crate::support::step_named(&workflow, "left_leaf")?;
    let right = crate::support::step_named(&workflow, "right_leaf")?;
    let left_slot = workflow
        .node(left)
        .and_then(|node| node.output.slot())
        .ok_or_else(|| "missing left slot".to_owned())?;
    let right_slot = workflow
        .node(right)
        .and_then(|node| node.output.slot())
        .ok_or_else(|| "missing right slot".to_owned())?;
    if left_slot == right_slot {
        return Err("sibling outputs alias one slot".to_owned());
    }
    Ok(())
}
