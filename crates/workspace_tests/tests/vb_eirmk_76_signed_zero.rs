#![forbid(unsafe_code)]

use std::error::Error;

use vb_core::{
    CapabilitySet, EngineError, FiniteF64, RunId, SlotIdx, SlotValue, StepBudget, Taint,
    ValueStore, new_run_frame,
};
use vb_expr::ExprError;
use vb_runtime::engine::{
    EvidenceCollector, RetryPolicy, RuntimeEngineError, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

const FLOAT_NEGATION_CASES: [(f64, u64); 8] = [
    (0.0, 0x8000_0000_0000_0000),
    (-0.0, 0x0000_0000_0000_0000),
    (f64::MAX, 0xffef_ffff_ffff_ffff),
    (-f64::MAX, 0x7fef_ffff_ffff_ffff),
    (f64::from_bits(1), 0x8000_0000_0000_0001),
    (f64::from_bits(0x8000_0000_0000_0001), 1),
    (12.5, 0xc029_0000_0000_0000),
    (-12.5, 0x4029_0000_0000_0000),
];

fn bytecode_eval(expression: &str, x: SlotValue, y: SlotValue) -> vb_expr::ExprResult<SlotValue> {
    let resolver = |reference: &str| match reference {
        "$input.x" => Some(SlotIdx::ZERO),
        "$input.zeta" => Some(SlotIdx::new(1)),
        _ => None,
    };
    let (program, constants) = vb_expr::bytecode::compile_expr(expression, &resolver)?;
    vb_expr::eval_expr_program(&program, &[Some(x), Some(y)], &constants)
}

fn workflow_eval(
    expression: &str,
    x: SlotValue,
    y: SlotValue,
) -> Result<RuntimeSignal, Box<dyn Error>> {
    let source = format!(
        "version: velvet-ballistics/v1\nname: signed_zero\nwhen: {{manual: {{}}}}\ninputs: {{x: any, zeta: any}}\nsteps:\n  - id: calculate\n    set: {{output: answer, value: '{expression}'}}\n  - id: done\n    finish: {{result: answer}}\n"
    );
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let mut frame = new_run_frame(RunId::new(1), &workflow)?;
    for (input, value) in workflow.input_slots().iter().zip([x, y]) {
        frame.write_slot_with_taint(input.slot, value, Taint::Clean)?;
    }
    let result = drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut StepBudget::new(8),
        &mut ValueStore::new(),
        &[],
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )?;
    Ok(result)
}

fn float_slot(value: f64) -> Result<SlotValue, String> {
    FiniteF64::new(value)
        .map(SlotValue::F64)
        .map_err(|error| error.to_string())
}

fn assert_float_bits(value: SlotValue, expected: u64) -> Result<(), String> {
    match value {
        SlotValue::F64(actual) if actual.get().to_bits() == expected => Ok(()),
        SlotValue::F64(actual) => Err(format!(
            "expected F64 bits {expected:#018x}, got {:#018x}",
            actual.get().to_bits()
        )),
        other => Err(format!("expected F64, got {other:?}")),
    }
}

fn workflow_value(signal: RuntimeSignal) -> Result<SlotValue, String> {
    match signal {
        RuntimeSignal::Finished(value) => Ok(value),
        other => Err(format!("expected Finished, got {other:?}")),
    }
}

#[test]
fn bytecode_negation_preserves_negative_zero_for_a_runtime_slot() -> Result<(), Box<dyn Error>> {
    let (program, constants) = vb_expr::bytecode::compile_expr("-$input.x", &|reference: &str| {
        (reference == "$input.x").then_some(SlotIdx::ZERO)
    })?;
    let input = SlotValue::F64(FiniteF64::new(0.0)?);
    let result = vb_expr::eval_expr_program(&program, &[Some(input)], &constants)?;
    let SlotValue::F64(value) = result else {
        return Err("bytecode negation did not return F64".into());
    };
    assert_eq!(value.get().to_bits(), 0x8000_0000_0000_0000);
    Ok(())
}

#[test]
fn compiled_workflow_negation_preserves_negative_zero_for_a_runtime_slot()
-> Result<(), Box<dyn Error>> {
    let source = b"version: velvet-ballistics/v1\nname: signed_zero\nwhen: {manual: {}}\ninputs: {x: any}\nsteps:\n  - id: calculate\n    set: {output: answer, value: '-$input.x'}\n  - id: done\n    finish: {result: answer}\n";
    let workflow = vb_compile::compile_workflow(source)?;
    let mut frame = new_run_frame(RunId::new(1), &workflow)?;
    let input = workflow.input_slots().first().ok_or("missing x input")?;
    frame.write_slot_with_taint(
        input.slot,
        SlotValue::F64(FiniteF64::new(0.0)?),
        Taint::Clean,
    )?;
    let result = drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut StepBudget::new(8),
        &mut ValueStore::new(),
        &[],
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )?;
    let RuntimeSignal::Finished(SlotValue::F64(value)) = result else {
        return Err("compiled workflow negation did not finish with F64".into());
    };
    assert_eq!(value.get().to_bits(), 0x8000_0000_0000_0000);
    Ok(())
}

#[test]
fn bytecode_negation_flips_float_bits_and_checked_integer_values() -> Result<(), String> {
    for (input, expected) in FLOAT_NEGATION_CASES {
        let value = float_slot(input)?;
        let single = bytecode_eval("-$input.x", value, SlotValue::Null)
            .map_err(|error| error.to_string())?;
        assert_float_bits(single, expected)?;
        let double = bytecode_eval("--$input.x", value, SlotValue::Null)
            .map_err(|error| error.to_string())?;
        assert_float_bits(double, input.to_bits())?;
    }
    for (input, expected) in [
        (0, 0),
        (1, -1),
        (-1, 1),
        (i64::MAX, -9_223_372_036_854_775_807),
        (-9_223_372_036_854_775_807, i64::MAX),
    ] {
        let result = bytecode_eval("-$input.x", SlotValue::I64(input), SlotValue::Null)
            .map_err(|error| error.to_string())?;
        if result != SlotValue::I64(expected) {
            return Err(format!("negation of {input} returned {result:?}"));
        }
    }
    Ok(())
}

#[test]
fn workflow_negation_flips_float_bits_and_checked_integer_values() -> Result<(), String> {
    for (input, expected) in FLOAT_NEGATION_CASES {
        let value = float_slot(input)?;
        let single = workflow_eval("-$input.x", value, SlotValue::Null)
            .map_err(|error| error.to_string())?;
        assert_float_bits(workflow_value(single)?, expected)?;
        let double = workflow_eval("--$input.x", value, SlotValue::Null)
            .map_err(|error| error.to_string())?;
        assert_float_bits(workflow_value(double)?, input.to_bits())?;
    }
    for (input, expected) in [
        (0, 0),
        (1, -1),
        (-1, 1),
        (i64::MAX, -9_223_372_036_854_775_807),
        (-9_223_372_036_854_775_807, i64::MAX),
    ] {
        let signal = workflow_eval("-$input.x", SlotValue::I64(input), SlotValue::Null)
            .map_err(|error| error.to_string())?;
        if workflow_value(signal)? != SlotValue::I64(expected) {
            return Err(format!(
                "workflow negation of {input} returned an unexpected value"
            ));
        }
    }
    Ok(())
}

#[test]
fn bytecode_negation_keeps_typed_errors_and_binary_subtraction() -> Result<(), String> {
    for expression in ["-$input.x", "--$input.x"] {
        match bytecode_eval(expression, SlotValue::I64(i64::MIN), SlotValue::Null) {
            Err(ExprError::IntegerOverflow) => {}
            other => return Err(format!("expected IntegerOverflow, got {other:?}")),
        }
    }
    match bytecode_eval("-$input.x", SlotValue::Bool(true), SlotValue::Null) {
        Err(ExprError::TypeMismatch { expected, found })
            if expected == "number" && found == "boolean" => {}
        other => return Err(format!("expected numeric TypeMismatch, got {other:?}")),
    }
    assert_float_bits(
        bytecode_eval("$input.x - $input.zeta", float_slot(0.0)?, float_slot(0.0)?)
            .map_err(|error| error.to_string())?,
        0,
    )?;
    if bytecode_eval(
        "$input.x - $input.zeta",
        SlotValue::I64(9),
        SlotValue::I64(4),
    )
    .map_err(|error| error.to_string())?
        != SlotValue::I64(5)
    {
        return Err("binary subtraction changed for I64 values".into());
    }
    Ok(())
}

#[test]
fn workflow_negation_keeps_typed_errors_and_binary_subtraction() -> Result<(), String> {
    for expression in ["-$input.x", "--$input.x"] {
        let error = match workflow_eval(expression, SlotValue::I64(i64::MIN), SlotValue::Null) {
            Ok(signal) => return Err(format!("expected integer overflow, got {signal:?}")),
            Err(error) => error,
        };
        let runtime_error = error
            .downcast_ref::<RuntimeEngineError>()
            .ok_or("workflow overflow returned an unexpected error type")?;
        if !matches!(
            runtime_error,
            RuntimeEngineError::Core(EngineError::InvalidCompiledWorkflow {
                reason: "integer arithmetic overflow"
            })
        ) {
            return Err(format!(
                "expected checked integer overflow, got {runtime_error:?}"
            ));
        }
    }
    let error = match workflow_eval("-$input.x", SlotValue::Bool(true), SlotValue::Null) {
        Ok(signal) => return Err(format!("expected type mismatch, got {signal:?}")),
        Err(error) => error,
    };
    let runtime_error = error
        .downcast_ref::<RuntimeEngineError>()
        .ok_or("workflow type error returned an unexpected error type")?;
    if !matches!(
        runtime_error,
        RuntimeEngineError::Core(EngineError::TypeMismatch {
            expected: "number",
            found: "boolean"
        })
    ) {
        return Err(format!(
            "expected numeric TypeMismatch, got {runtime_error:?}"
        ));
    }
    let signal = workflow_eval("$input.x - $input.zeta", float_slot(0.0)?, float_slot(0.0)?)
        .map_err(|error| error.to_string())?;
    assert_float_bits(workflow_value(signal)?, 0)?;
    let signal = workflow_eval(
        "$input.x - $input.zeta",
        SlotValue::I64(9),
        SlotValue::I64(4),
    )
    .map_err(|error| error.to_string())?;
    if workflow_value(signal)? != SlotValue::I64(5) {
        return Err("workflow binary subtraction changed for I64 values".into());
    }
    Ok(())
}
