#![forbid(unsafe_code)]

use vb_compile::{CompileError, compile_workflow};
use vb_core::{
    CapabilitySet, RunId, SlotIdx, SlotValue, StepBudget, Taint, ValueStore, new_run_frame,
};
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::collect::CollectStates;

type TestResult = Result<(), String>;

const FINISH: &str = "  - id: done\n    finish:\n      result: \"$steps.folded\"\n";

fn source(prefix: &str, expression: &str, suffix: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: reduce_lexical_binding\nwhen:\n  manual: {{}}\nsteps:\n{prefix}  - id: folded\n    reduce:\n      variable: total\n      input: \"8\"\n      initial: \"10\"\n      steps:\n        - id: update\n          set:\n            output: local_value\n            value: \"{expression}\"\n{suffix}"
    )
}

fn assert_run(source: &str, input: &[i64], expected: i64) -> TestResult {
    let workflow = compile_workflow(source.as_bytes())
        .map_err(|error| format!("compilation failed: {error:?}"))?;
    let original_ir = postcard::to_allocvec(&workflow.to_parts())
        .map_err(|error| format!("IR encoding failed: {error:?}"))?;
    let mut frame = new_run_frame(RunId::new(1), &workflow)
        .map_err(|error| format!("frame creation failed: {error:?}"))?;
    let values: Vec<SlotValue> = input.iter().copied().map(SlotValue::I64).collect();
    let mut store = ValueStore::new();
    let list = store
        .insert_list(values.clone())
        .map_err(|error| format!("input list allocation failed: {error:?}"))?;
    frame
        .write_slot_with_taint(
            SlotIdx::new(8),
            SlotValue::List(list),
            Taint::DerivedFromSecret,
        )
        .map_err(|error| format!("input binding failed: {error:?}"))?;
    let signal = drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut StepBudget::new(128),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )
    .map_err(|error| format!("execution failed: {error:?}"))?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(expected)));
    assert_eq!(
        *frame
            .read_slot(SlotIdx::new(8))
            .map_err(|error| format!("input slot: {error:?}"))?,
        SlotValue::List(list)
    );
    assert_eq!(
        frame
            .read_taint(SlotIdx::new(8))
            .map_err(|error| format!("input taint: {error:?}"))?,
        Taint::DerivedFromSecret
    );
    assert_eq!(
        store
            .list(list)
            .map_err(|error| format!("input list: {error:?}"))?,
        values.as_slice()
    );
    assert_eq!(
        postcard::to_allocvec(&workflow.to_parts())
            .map_err(|error| format!("IR encoding failed: {error:?}"))?,
        original_ir
    );
    Ok(())
}

#[test]
fn declared_accumulator_executes_for_empty_single_and_multiple_items() -> TestResult {
    let source = source("", "$total + 1", FINISH);
    for (input, expected) in [(&[][..], 10), (&[4][..], 11), (&[4, 7, -3][..], 13)] {
        assert_run(&source, input, expected)?;
    }
    Ok(())
}

#[test]
fn reduce_result_and_outer_binding_are_available_to_later_expressions() -> TestResult {
    let prefix = "  - id: seed\n    set:\n      output: increment\n      value: \"5\"\n";
    let suffix = "  - id: after\n    set:\n      output: answer\n      value: \"$steps.folded + 7\"\n  - id: done\n    finish:\n      result: \"$step.after\"\n";
    assert_run(&source(prefix, "$total + $increment", suffix), &[2, 9], 27)
}

#[test]
fn accumulator_shadowing_stays_inside_the_reduce_body() -> TestResult {
    let prefix = "  - id: seed\n    set:\n      output: total\n      value: \"99\"\n";
    let suffix = "  - id: after\n    set:\n      output: answer\n      value: \"$steps.folded + $total\"\n  - id: done\n    finish:\n      result: \"$steps.after\"\n";
    assert_run(&source(prefix, "$total + 1", suffix), &[1, 2, 3], 112)
}

#[test]
fn accumulator_binding_does_not_escape_the_reduce_body() -> TestResult {
    let suffix = "  - id: done\n    finish:\n      result: \"$total\"\n";
    let errors = match compile_workflow(source("", "$total + 1", suffix).as_bytes()) {
        Err(errors) => errors,
        Ok(_) => return Err("body-local accumulator escaped".into()),
    };
    assert!(errors.0.iter().any(|error| matches!(
        error,
        CompileError::UnknownReferenceName { kind: "output", name, .. } if name.as_ref() == "total"
    )));
    Ok(())
}

#[test]
fn accumulator_expression_preserves_the_initial_integer_type() -> TestResult {
    let errors = match compile_workflow(source("", "$total == 10", FINISH).as_bytes()) {
        Err(errors) => errors,
        Ok(_) => return Err("reduce accepted a changing accumulator type".into()),
    };
    assert!(errors.0.iter().any(|error| matches!(
        error,
        CompileError::StepFieldShape {
            field: "reduce.steps",
            ..
        }
    )));
    Ok(())
}

#[test]
fn local_accumulator_precedes_outer_step_id_without_hiding_explicit_steps() -> TestResult {
    let prefix = "  - id: total\n    set:\n      output: increment\n      value: \"7\"\n";
    let suffix = "  - id: after\n    set:\n      output: answer\n      value: \"$steps.folded + $total + $steps.total + $increment\"\n  - id: done\n    finish:\n      result: \"$steps.after\"\n";
    assert_run(
        &source(prefix, "$total + $steps.total", suffix),
        &[2, 9],
        45,
    )
}

#[test]
fn global_output_and_step_name_ambiguity_remains_an_error() -> TestResult {
    let prefix = "  - id: total\n    set:\n      output: increment\n      value: \"7\"\n  - id: seed\n    set:\n      output: total\n      value: \"99\"\n";
    let suffix = "  - id: done\n    finish:\n      result: \"$total\"\n";
    let expression = "$total + 1";
    assert_run(&source(prefix, expression, FINISH), &[2, 9], 12)?;
    let errors = match compile_workflow(source(prefix, expression, suffix).as_bytes()) {
        Err(errors) => errors,
        Ok(_) => return Err("global ambiguous reference was accepted".into()),
    };
    assert!(errors.0.iter().any(|error| matches!(
        error,
        CompileError::ExpressionLoweringUnsupported { feature }
            if feature.contains("ambiguous reference")
    )));
    Ok(())
}
