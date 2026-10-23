//! Negative F1 compile-time and type error tests.

use vb_compile::{compile_workflow, CompileError};

use crate::vb_f1_scalar_do_support::compile_f1_workflow;

// Negative: duplicate output name
#[test]
fn test_duplicate_output_name_error() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: dup\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"1\" }\n  - id: s2\n    set: { output: x, value: \"2\" }\n  - id: s3\n    finish: { result: 0 }\n";
    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::DuplicateOutputName { name } = e {
                    if &**name == "x" {
                        return Ok(());
                    }
                }
            }
            Err(format!("expected DuplicateOutputName(x), got: {errors:?}"))
        }
        Ok(_) => Err("expected compile error for duplicate output name, got success".to_owned()),
    }
}

// Negative: undefined name reference in finish
#[test]
fn test_undefined_name_reference_error() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: undef\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"1\" }\n  - id: s2\n    finish: { result: \"missing\" }\n";
    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::UnknownOutputName { name } = e {
                    if &**name == "missing" {
                        return Ok(());
                    }
                }
            }
            Err(format!(
                "expected UnknownOutputName(missing), got: {errors:?}"
            ))
        }
        Ok(_) => Err("expected compile error for undefined name, got success".to_owned()),
    }
}

// Negative: forward reference (future is a later step)
#[test]
fn test_forward_reference_error() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: fwd\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"$future\" }\n  - id: later_step\n    set: { output: result_y, value: \"1\" }\n  - id: s3\n    finish: { result: 0 }\n";
    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::UnknownReferenceName {
                    reference, name, ..
                } = e
                {
                    if &**name == "future" && &**reference == "$future" {
                        return Ok(());
                    }
                }
            }
            Err(format!(
                "expected UnknownReferenceName for future, got: {errors:?}"
            ))
        }
        Ok(_) => Err("expected compile error for forward reference, got success".to_owned()),
    }
}

// Negative: self reference
#[test]
fn test_self_reference_error() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: selfref\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"$x\" }\n  - id: s2\n    finish: { result: 0 }\n";
    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::UnknownReferenceName {
                    reference, name, ..
                } = e
                {
                    if &**name == "x" && &**reference == "$x" {
                        return Ok(());
                    }
                }
            }
            Err(format!(
                "expected UnknownReferenceName for x, got: {errors:?}"
            ))
        }
        Ok(_) => Err("expected compile error for self reference, got success".to_owned()),
    }
}

// Negative: ambiguous bare name vs distinct step-ID
#[test]
fn test_ambiguous_bare_name_error() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: ambig\nwhen: { manual: {} }\nsteps:\n  - id: conflict\n    set: { output: left, value: \"1\" }\n  - id: second\n    set: { output: conflict, value: \"2\" }\n  - id: third\n    set: { output: result, value: \"$conflict\" }\n  - id: s4\n    finish: { result: 0 }\n";
    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::ExpressionLoweringUnsupported { feature } = e {
                    if feature.contains("ambiguous") || feature.contains("Ambiguous") {
                        return Ok(());
                    }
                }
            }
            Err(format!(
                "expected ExpressionLoweringUnsupported for ambiguous, got: {errors:?}"
            ))
        }
        Ok(_) => {
            Err("expected compile error for ambiguous/duplicate reference, got success".to_owned())
        }
    }
}

// Positive: $step.conflict disambiguates to the step (not the output)
#[test]
fn test_step_prefix_disambiguates_to_step() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: disambig\nwhen: { manual: {} }\nsteps:\n  - id: conflict\n    set: { output: left, value: \"41\" }\n  - id: second\n    set: { output: conflict, value: \"43\" }\n  - id: third\n    set: { output: result, value: \"$step.conflict\" }\n  - id: s4\n    finish: { result: 0 }\n";
    let mut run = crate::vb_f1_scalar_do_support::F1SourceRun::new(yaml)?;
    // $step.conflict resolves to the step's primary output: left=41
    crate::vb_f1_scalar_do_support::assert_finished_with_value(
        &mut run,
        vb_core::value::SlotValue::I64(41),
    )
}

// Negative: invalid numeric action ID (negative)
#[test]
fn test_invalid_action_id_negative() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: inv\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"1\" }\n  - id: s2\n    do: { action: \"-1\", input: \"0\" }\n  - id: s3\n    finish: { result: 0 }\n";
    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::StepFieldShape { field, .. } = e {
                    if *field == "action" {
                        return Ok(());
                    }
                }
            }
            Err(format!("expected StepFieldShape(action), got: {errors:?}"))
        }
        Ok(_) => Err("expected compile error for invalid action ID, got success".to_owned()),
    }
}

// Negative: invalid numeric action ID (too large)
#[test]
fn test_invalid_action_id_too_large() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: invlarge\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"1\" }\n  - id: s2\n    do: { action: \"70000\", input: \"0\" }\n  - id: s3\n    finish: { result: 0 }\n";
    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::StepFieldShape { field, .. } = e {
                    if *field == "action" {
                        return Ok(());
                    }
                }
            }
            Err(format!(
                "expected PrimitiveLoweringLimitExceeded(action), got: {errors:?}"
            ))
        }
        Ok(_) => Err("expected compile error for large action ID, got success".to_owned()),
    }
}

// Negative: invalid input slot (negative)
#[test]
fn test_invalid_input_slot_negative() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: invslot\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"1\" }\n  - id: s2\n    do: { action: \"1\", input: \"-1\" }\n  - id: s3\n    finish: { result: 0 }\n";
    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for e in errors.0.iter() {
                if let CompileError::SlotIndexOutOfRange { value } = e {
                    if *value == -1 {
                        return Ok(());
                    }
                }
            }
            Err(format!("expected SlotIndexOutOfRange(-1), got: {errors:?}"))
        }
        Ok(_) => Err("expected compile error for invalid input slot, got success".to_owned()),
    }
}

// Negative: finish with expression (not in bounded F1 slice)
#[test]
fn test_finish_expression_not_supported() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: finishexpr\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"1\" }\n  - id: s2\n    finish: { result: \"x + 1\" }\n";
    match compile_f1_workflow(yaml) {
        Err(_) => Ok(()),
        Ok(_) => Err("expected compile error for finish expression, got success".to_owned()),
    }
}

// Positive: scalar type error (incompatible types in expression)
#[test]
fn test_scalar_type_error_bool_plus_i64() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: typeerr\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"true\" }\n  - id: s2\n    set: { output: result, value: \"$x + 2\" }\n  - id: s3\n    finish: { result: 1 }\n";
    match compile_f1_workflow(yaml) {
        Err(msg)
            if msg.contains("type")
                || msg.contains("Type")
                || msg.contains("incompatible")
                || msg.contains("bool")
                || msg.contains("Bool") =>
        {
            Ok(())
        }
        Err(msg) => Err(format!("expected scalar type error, got: {msg}")),
        Ok(_) => Err("expected compile error for bool + i64, got success".to_owned()),
    }
}
