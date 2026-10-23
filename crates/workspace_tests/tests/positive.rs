//! Positive F1 scalar Set -> Finish tests.

use vb_core::value::SlotValue;

use crate::vb_f1_scalar_do_support::{assert_finished_with_value, F1SourceRun};

fn yaml_set_finish(output: &str, value: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: f1_set\nwhen: {{ manual: {{}} }}\nsteps:\n  - id: s1\n    set:\n      output: {output}\n      value: \"{value}\"\n  - id: s2\n    finish:\n      result: 0\n"
    )
}

fn yaml_set_expr_finish(value: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: f1_expr\nwhen: {{ manual: {{}} }}\nsteps:\n  - id: s1\n    set:\n      output: x\n      value: \"10\"\n  - id: s2\n    set:\n      output: result\n      value: \"{value}\"\n  - id: s3\n    finish:\n      result: 1\n"
    )
}

fn yaml_step_ref_finish(ref_syntax: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: f1_ref\nwhen: {{ manual: {{}} }}\nsteps:\n  - id: producer\n    set:\n      output: out\n      value: \"42\"\n  - id: consumer\n    finish:\n      result: {ref_syntax}\n"
    )
}

// Positive: I64 scalar Set -> Finish
#[test]
fn test_i64_set_unchanged_to_finish() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_set_finish("answer", "42"))?;
    assert_finished_with_value(&mut run, SlotValue::I64(42))
}

// Positive: Bool scalar Set -> Finish
#[test]
fn test_bool_set_to_finish() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_set_finish("flag", "true"))?;
    assert_finished_with_value(&mut run, SlotValue::Bool(true))
}

// Positive: Null scalar Set -> Finish
#[test]
fn test_null_set_to_finish() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_set_finish("null_val", "null"))?;
    assert_finished_with_value(&mut run, SlotValue::Null)
}

// Positive: F64 scalar Set -> Finish (finite)
#[test]
fn test_f64_set_to_finish() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_set_finish("ratio", "3.14"))?;
    let expected =
        SlotValue::F64(vb_core::FiniteF64::new(3.14).map_err(|e| format!("FiniteF64: {e:?}"))?);
    assert_finished_with_value(&mut run, expected)
}

// Positive: arithmetic expression through prior output $alias
#[test]
fn test_arithmetic_through_prior_output_alias() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_set_expr_finish("$x + 2"))?;
    assert_finished_with_value(&mut run, SlotValue::I64(12))
}

// Positive: step reference syntax $step_id
#[test]
fn test_step_reference_by_id() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_step_ref_finish("$producer"))?;
    assert_finished_with_value(&mut run, SlotValue::I64(42))
}

// Positive: step reference syntax $step.id
#[test]
fn test_step_reference_qualified() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_step_ref_finish("$step.producer"))?;
    assert_finished_with_value(&mut run, SlotValue::I64(42))
}

// Positive: step reference syntax $steps.id
#[test]
fn test_step_reference_steps_prefix() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_step_ref_finish("$steps.producer"))?;
    assert_finished_with_value(&mut run, SlotValue::I64(42))
}
