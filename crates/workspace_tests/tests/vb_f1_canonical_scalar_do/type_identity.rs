//! F1 source-to-runtime type identity tests.
//! Compile-time type errors for boolean arithmetic and runtime
//! verification of numeric identity preservation.

use vb_compile::{compile_workflow, CompileError};
use vb_core::value::SlotValue;

use crate::vb_f1_scalar_do_support::{assert_finished_with_value, F1SourceRun};

fn yaml_bool_producer_consumer(value: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: bool_prod\nwhen: {{ manual: {{}} }}\nsteps:\n  - id: producer\n    set:\n      output: flag\n      value: \"true\"\n  - id: consumer\n    set:\n      output: result\n      value: \"{value}\"\n  - id: finish\n    finish:\n      result: 0\n"
    )
}


fn yaml_steps_prefix_consumer(value: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: steps_prefix\nwhen: {{ manual: {{}} }}\nsteps:\n  - id: producer\n    set:\n      output: flag\n      value: \"true\"\n  - id: s2\n    set:\n      output: result\n      value: \"{value}\"\n  - id: s3\n    finish:\n      result: 0\n"
    )
}

fn yaml_transitive_copy(value: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: trans_copy\nwhen: {{ manual: {{}} }}\nsteps:\n  - id: producer\n    set:\n      output: flag\n      value: \"true\"\n  - id: copier\n    set:\n      output: copied\n      value: \"{value}\"\n  - id: s3\n    set:\n      output: result\n      value: \"$copied + 2\"\n  - id: s4\n    finish:\n      result: 0\n"
    )
}

fn yaml_direct_literal() -> String {
    "version: velvet-ballistics/v1\nname: direct_lit\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set:\n      output: result\n      value: \"true + 2\"\n  - id: s2\n    finish:\n      result: 0\n"
        .to_owned()
}

fn yaml_numeric_producer(value: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: num_prod\nwhen: {{ manual: {{}} }}\nsteps:\n  - id: producer\n    set:\n      output: flag\n      value: \"10\"\n  - id: consumer\n    set:\n      output: result\n      value: \"{value}\"\n  - id: finish\n    finish:\n      result: result\n"
    )
}

fn yaml_numeric_qualified(value: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: num_qual\nwhen: {{ manual: {{}} }}\nsteps:\n  - id: producer\n    set:\n      output: flag\n      value: \"10\"\n  - id: consumer\n    set:\n      output: result\n      value: \"{value}\"\n  - id: finish\n    finish:\n      result: result\n"
    )
}

/// Negative: boolean producer, consumer arithmetic via alias.
#[test]
fn test_bool_producer_step_id_producer() -> Result<(), String> {
    let yaml = yaml_bool_producer_consumer("$producer + 2");
    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::TypeMismatch {
                    field,
                    expected,
                    found,
                } = e
                {
                    if *field == "set.value" && *expected == "number" && *found == "boolean" {
                        return Ok(());
                    }
                }
            }
            Err(format!(
                "expected TypeMismatch(set.value, number, boolean), got: {errors:?}"
            ))
        }
        Ok(_) => Err("expected compile error for bool producer arithmetic, got success".to_owned()),
    }
}

/// Negative: consumer arithmetic via $step prefix.
#[test]
fn test_step_prefix_bool_producer() -> Result<(), String> {
    let yaml = yaml_bool_producer_consumer("$step.producer + 2");
    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::TypeMismatch {
                    field,
                    expected,
                    found,
                } = e
                {
                    if *field == "set.value" && *expected == "number" && *found == "boolean" {
                        return Ok(());
                    }
                }
            }
            Err(format!(
                "expected TypeMismatch(set.value, number, boolean), got: {errors:?}"
            ))
        }
        Ok(_) => Err(
            "expected compile error for $step.producer bool arithmetic, got success".to_owned(),
        ),
    }
}

/// Negative: consumer arithmetic via $steps prefix.
#[test]
fn test_steps_prefix_bool_producer() -> Result<(), String> {
    let yaml = yaml_steps_prefix_consumer("$steps.producer + 2");
    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::TypeMismatch {
                    field,
                    expected,
                    found,
                } = e
                {
                    if *field == "set.value" && *expected == "number" && *found == "boolean" {
                        return Ok(());
                    }
                }
            }
            Err(format!(
                "expected TypeMismatch(set.value, number, boolean), got: {errors:?}"
            ))
        }
        Ok(_) => Err(
            "expected compile error for $steps.producer bool arithmetic, got success".to_owned(),
        ),
    }
}

/// Negative: transitive copy of bool, then arithmetic on copied value.
#[test]
fn test_transitive_copy_bool_arithmetic() -> Result<(), String> {
    let yaml = yaml_transitive_copy("$step.producer");
    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::TypeMismatch {
                    field,
                    expected,
                    found,
                } = e
                {
                    if *field == "set.value" && *expected == "number" && *found == "boolean" {
                        return Ok(());
                    }
                }
            }
            Err(format!(
                "expected TypeMismatch(set.value, number, boolean), got: {errors:?}"
            ))
        }
        Ok(_) => Err(
            "expected compile error for transitive copy bool arithmetic, got success".to_owned(),
        ),
    }
}

/// Negative: direct boolean literal arithmetic.
#[test]
fn test_direct_literal_bool_arithmetic() -> Result<(), String> {
    let yaml = yaml_direct_literal();
    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::TypeMismatch {
                    field,
                    expected,
                    found,
                } = e
                {
                    if *field == "set.value" && *expected == "number" && *found == "boolean" {
                        return Ok(());
                    }
                }
            }
            Err(format!(
                "expected TypeMismatch(set.value, number, boolean), got: {errors:?}"
            ))
        }
        Ok(_) => Err(
            "expected compile error for direct literal bool arithmetic, got success".to_owned(),
        ),
    }
}

/// Positive: numeric producer, consumer arithmetic via alias.
#[test]
fn test_numeric_producer_alias() -> Result<(), String> {
    let yaml = yaml_numeric_producer("$producer + 2");
    let mut run = F1SourceRun::new(&yaml)?;
    assert_finished_with_value(&mut run, SlotValue::I64(12))
}

/// Positive: numeric producer, consumer arithmetic via qualified step reference.
#[test]
fn test_numeric_qualified_step_reference() -> Result<(), String> {
    let yaml = yaml_numeric_qualified("$step.producer + 2");
    let mut run = F1SourceRun::new(&yaml)?;
    assert_finished_with_value(&mut run, SlotValue::I64(12))
}
