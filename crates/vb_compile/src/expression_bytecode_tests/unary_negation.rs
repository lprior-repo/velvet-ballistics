#![forbid(unsafe_code)]

use super::compile_expr_to_bytecode_with_step_slots;
use crate::expression::parse_expression;
use vb_core::ids::{ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{FiniteF64, SlotValue};
use vb_core::workflow::{
    CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts, node,
};
use vb_core::{EngineError, RunFrame};

const SIGN: u64 = 0x8000_0000_0000_0000;

fn evaluate(source: &str, input: SlotValue) -> Result<Result<SlotValue, EngineError>, String> {
    let expression = parse_expression(source).map_err(|error| error.to_string())?;
    let mut constants = Vec::new();
    let mut accessors = Vec::new();
    let program = compile_expr_to_bytecode_with_step_slots(
        &expression,
        &mut constants,
        &mut accessors,
        &[(Box::<str>::from("value"), SlotIdx::new(0))],
    )
    .map_err(|error| error.to_string())?;
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: "unary_negation".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![node(StepIdx::new(0), CompiledNodeKind::Nop, None, None)].into_boxed_slice(),
        expressions: vec![program].into_boxed_slice(),
        accessors: accessors.into_boxed_slice(),
        constants: constants.into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|error| error.to_string())?;
    let mut run =
        RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1).map_err(|error| error.to_string())?;
    run.write_slot(SlotIdx::new(0), input)
        .map_err(|error| error.to_string())?;
    Ok(vb_core::engine::eval_expr(&workflow, &run, ExprIdx::new(0)).map(|(value, _)| value))
}

fn float_bits(value: SlotValue) -> Result<u64, String> {
    match value {
        SlotValue::F64(value) => Ok(value.get().to_bits()),
        other => Err(format!("expected F64, found {other:?}")),
    }
}

#[test]
fn unary_negation_literal_zero_bits() -> Result<(), String> {
    for (source, expected) in [("-0.0", SIGN), ("--0.0", 0), ("-(-0.0)", 0)] {
        let value = evaluate(source, SlotValue::Null)?.map_err(|error| error.to_string())?;
        assert_eq!(float_bits(value)?, expected, "{source}");
    }
    Ok(())
}

#[test]
fn unary_negation_reference_float_bits() -> Result<(), String> {
    for bits in [
        0,
        SIGN,
        1,
        0x8000_0000_0000_0001,
        0x7fef_ffff_ffff_ffff,
        0xffef_ffff_ffff_ffff,
        0x3ff0_0000_0000_0000,
        0xbff0_0000_0000_0000,
    ] {
        let input = SlotValue::F64(
            FiniteF64::new(f64::from_bits(bits)).map_err(|error| error.to_string())?,
        );
        for (source, expected) in [("-$steps.value", bits ^ SIGN), ("--$steps.value", bits)] {
            let value = evaluate(source, input)?.map_err(|error| error.to_string())?;
            assert_eq!(float_bits(value)?, expected, "{source}, input {bits:#018x}");
        }
    }
    Ok(())
}

#[test]
fn unary_negation_integer_values_and_overflow() -> Result<(), String> {
    for (input, expected) in [
        (0, 0),
        (1, -1),
        (-1, 1),
        (i64::MAX, -i64::MAX),
        (-i64::MAX, i64::MAX),
    ] {
        assert_eq!(
            evaluate("-$steps.value", SlotValue::I64(input))?,
            Ok(SlotValue::I64(expected))
        );
        assert_eq!(
            evaluate("--$steps.value", SlotValue::I64(input))?,
            Ok(SlotValue::I64(input))
        );
    }
    for source in ["-$steps.value", "--$steps.value"] {
        assert_eq!(
            evaluate(source, SlotValue::I64(i64::MIN))?,
            Err(EngineError::InvalidCompiledWorkflow {
                reason: "integer arithmetic overflow",
            })
        );
    }
    Ok(())
}

#[test]
fn unary_negation_rejects_nonnumeric_operand() -> Result<(), String> {
    assert_eq!(
        evaluate("-$steps.value", SlotValue::Bool(true))?,
        Err(EngineError::TypeMismatch {
            expected: "number",
            found: "boolean",
        })
    );
    Ok(())
}
