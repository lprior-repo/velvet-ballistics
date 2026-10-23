#![forbid(unsafe_code)]
//! F1 named reference resolution order: named errors before type errors.
//! Verifies that forward and self reference errors are raised before
//! arithmetic type errors, ensuring the compiler checks name scope first.
//!
//! Tests are grounded in actual Runtime API and compile_workflow harness.
//! No changes to existing tests or production code.

use vb_compile::{compile_workflow, CompileError};

mod vb_f1_scalar_do_support;

// Test 1: Forward alias reference (output kind)
// Consumer references $future before future is defined in a later step.
// Expect UnknownReferenceName(kind="output") BEFORE any type error.
#[test]
fn test_forward_alias_reference_error_output_kind() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\n\
        name: fwd_alias\n\
        when: { manual: {} }\n\
        steps:\n\
          - id: consumer_first\n\
            set: { output: result, value: \"$future + true\" }\n\
          - id: future_id\n\
            set: { output: future, value: \"10\" }\n\
          - id: finish_step\n\
            finish: { result: result }\n";

    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::UnknownReferenceName {
                    kind, reference, name, ..
                } = e {
                    if &**name == "future"
                        && &**reference == "$future"
                        && *kind == "output"
                    {
                        return Ok(());
                    }
                }
                // Fail if we see a TypeMismatch before the reference error
                if matches!(e, CompileError::TypeMismatch { .. }) {
                    return Err(format!(
                        "got TypeMismatch before UnknownReferenceName: {errors:?}"
                    ));
                }
            }
            Err(format!(
                "expected UnknownReferenceName(kind=output, name=future), got: {errors:?}"
            ))
        }
        Ok(_) => Err("expected compile error for forward alias reference, got success".to_owned()),
    }
}

// Test 2: Forward explicit step reference (step kind)
// Consumer references $step.future_id before step future_id is defined.
// Expect UnknownReferenceName(kind="step") BEFORE any type error.
#[test]
fn test_forward_explicit_step_reference_error_step_kind() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\n\
        name: fwd_step\n\
        when: { manual: {} }\n\
        steps:\n\
          - id: consumer_first\n\
            set: { output: result, value: \"$step.future_id + true\" }\n\
          - id: future_id\n\
            set: { output: future, value: \"10\" }\n\
          - id: finish_step\n\
            finish: { result: result }\n";

    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::UnknownReferenceName {
                    kind, reference, name, ..
                } = e {
                    if &**name == "future_id"
                        && &**reference == "$step.future_id"
                        && *kind == "step"
                    {
                        return Ok(());
                    }
                }
                // Fail if we see a TypeMismatch before the reference error
                if matches!(e, CompileError::TypeMismatch { .. }) {
                    return Err(format!(
                        "got TypeMismatch before UnknownReferenceName: {errors:?}"
                    ));
                }
            }
            Err(format!(
                "expected UnknownReferenceName(kind=step, name=future_id), got: {errors:?}"
            ))
        }
        Ok(_) => Err("expected compile error for forward step reference, got success".to_owned()),
    }
}

// Test 3: Self alias reference (output kind)
// Step defines self_value with expression $self_value + true (references itself).
// Expect UnknownReferenceName(kind="output") BEFORE any type error.
#[test]
fn test_self_alias_reference_error_output_kind() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\n\
        name: selfref\n\
        when: { manual: {} }\n\
        steps:\n\
          - id: self_step\n\
            set: { output: self_value, value: \"$self_value + true\" }\n\
          - id: finish_step\n\
            finish: { result: self_value }\n";

    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::UnknownReferenceName {
                    kind, reference, name, ..
                } = e {
                    if &**name == "self_value"
                        && &**reference == "$self_value"
                        && *kind == "output"
                    {
                        return Ok(());
                    }
                }
                // Fail if we see a TypeMismatch before the reference error
                if matches!(e, CompileError::TypeMismatch { .. }) {
                    return Err(format!(
                        "got TypeMismatch before UnknownReferenceName: {errors:?}"
                    ));
                }
            }
            Err(format!(
                "expected UnknownReferenceName(kind=output, name=self_value), got: {errors:?}"
            ))
        }
        Ok(_) => Err("expected compile error for self alias reference, got success".to_owned()),
    }
}

// Test 4: Positive - producer before consumer
// Producer defines future=10, then consumer uses $future + 2.
// Expect success with value 12 (unchanged IR execution).
#[test]
fn test_producer_before_consumer_success() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\n\
        name: fwd_ok\n\
        when: { manual: {} }\n\
        steps:\n\
          - id: producer\n\
            set: { output: future, value: \"10\" }\n\
          - id: consumer\n\
            set: { output: result, value: \"$future + 2\" }\n\
          - id: finish_step\n\
            finish: { result: result }\n";

    let mut run = vb_f1_scalar_do_support::F1SourceRun::new(yaml)?;
    vb_f1_scalar_do_support::assert_finished_with_value(
        &mut run,
        vb_core::value::SlotValue::I64(12),
    )
}
