#![forbid(unsafe_code)]
//! Explain workflow compilation errors.

use crate::args::{OutputFormat, ParseError};
use crate::exit_code::CliExitCode;
use crate::explain_repair::explain_repair_hint;
use crate::explain_validation::explain_validation_error;
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::output::{
    json_error, json_out, output_error_exit, write_failure_message, write_stderr_line,
    write_stdout_line,
};
use crate::output_utils::*;
use std::process::ExitCode;

/// Explain workflow compilation errors related to step validation.
pub(crate) fn explain_step_errors(err: &vb_compile::CompileError) {
    use vb_compile::CompileError;
    match err {
        CompileError::MissingStepField { step, field } => {
            crate::outln!("Step Missing Field");
            crate::outln!("  Step {step} is missing required field '{field}'.");
        }
        CompileError::StepFieldShape {
            step,
            field,
            expected: _,
        } => {
            crate::outln!("Invalid Step Field Shape");
            crate::outln!("  Step {step} field '{field}' has wrong structure.");
        }
        CompileError::StepIndexOutOfRange { value } => {
            crate::outln!("Step Index Out of Range");
            crate::outln!("  Step index {value} exceeds the u16 representation limit.");
        }
        CompileError::SlotIndexOutOfRange { value } => {
            crate::outln!("Slot Index Out of Range");
            crate::outln!("  Slot index {value} is outside the valid u16 range.");
        }
        CompileError::BranchTargetOutOfRange { value } => {
            crate::outln!("Branch Target Out of Range");
            crate::outln!("  Branch target {value} is outside the valid u16 range.");
        }
        CompileError::BackwardBranchTarget { step, target } => {
            crate::outln!("Backward Branch Target");
            crate::outln!("  Step {step} branches to {target}, but forward branches are required.");
        }
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive,
            field,
            value,
            limit,
        } => {
            crate::outln!("Primitive Limit Exceeded");
            crate::outln!(
                "  Primitive '{primitive}' field '{field}' value {value} exceeds limit {limit}."
            );
        }
        CompileError::LastStepMustFinish => {
            crate::outln!("Last Step Must Finish");
            crate::outln!("  The final step in a linear workflow must be a 'finish' step.");
        }
        CompileError::UnsupportedConstantValue { step } => {
            crate::outln!("Unsupported Constant Value");
            crate::outln!("  Step {step} constant value must be a scalar YAML value.");
        }
        CompileError::UnknownReferenceRoot { reference, root } => {
            crate::outln!("Unknown Reference Root");
            crate::outln!("  Reference '{reference}' uses unknown root '{root}'.");
        }
        CompileError::IllegalReference { reference } => {
            crate::outln!("Illegal Reference");
            crate::outln!("  Reference '{reference}' is not allowed in deterministic workflows.");
        }
        CompileError::UnknownReferenceName {
            kind,
            reference,
            name,
        } => {
            crate::outln!("Unknown Reference");
            crate::outln!("  Reference '{reference}' refers to unknown {kind} '{name}'.");
        }
        CompileError::UnsupportedAccessorReference {
            reference,
            root,
            path,
        } => {
            crate::outln!("Unsupported Accessor Reference");
            crate::outln!(
                "  Accessor reference '{reference}' (root: {root}, path: {path}) is not supported."
            );
        }
        CompileError::UnknownStepTarget { step, target } => {
            crate::outln!("Unknown Step Target");
            crate::outln!("  Step {step} branches to undeclared step index {target}.");
        }
        CompileError::UnreachableStep { step } => {
            crate::outln!("Unreachable Step");
            crate::outln!("  Step {step} cannot be reached from the workflow entry point.");
        }
        CompileError::TypeMismatch {
            field,
            expected,
            found,
        } => {
            crate::outln!("Type Mismatch");
            crate::outln!("  Field '{field}': expected {expected}, but found {found}.");
        }
        CompileError::Workflow(e) => {
            crate::outln!("Workflow IR Validation Error");
            crate::outln!("  {e}");
        }
        CompileError::Validation(e) => {
            super::explain_validation::explain_validation_error(e);
        }
        _ => {
            crate::outln!("Compilation Error");
            crate::outln!("  {err}");
        }
    }
    crate::explain_reports::explain_compile_repair_hint(err);
}
