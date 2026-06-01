#![forbid(unsafe_code)]
//! Explain workflow compilation errors.

use std::process::ExitCode;
use crate::args::{OutputFormat, ParseError};
use crate::exit_code::CliExitCode;
use crate::output::{json_error, json_out, output_error_exit, write_stdout_line, write_stderr_line, write_failure_message};
use crate::output_utils::*;
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::explain_repair::explain_repair_hint;
use crate::explain_validation::explain_validation_error;

/// Explain workflow compilation errors related to step validation.
pub(crate) fn explain_step_errors(err: &vb_compile::CompileError) {
    use vb_compile::CompileError;
    match err {
        CompileError::StepMissingField { step, field } => {
            outln!("Step Missing Field");
            outln!("  Step {step} is missing required field '{field}'.");
        }
        CompileError::StepFieldShape {
            step,
            field,
            expected: _,
        } => {
            outln!("Invalid Step Field Shape");
            outln!("  Step {step} field '{field}' has wrong structure.");
        }
        CompileError::StepIndexOutOfRange { value } => {
            outln!("Step Index Out of Range");
            outln!("  Step index {value} exceeds the u16 representation limit.");
        }
        CompileError::SlotIndexOutOfRange { value } => {
            outln!("Slot Index Out of Range");
            outln!("  Slot index {value} is outside the valid u16 range.");
        }
        CompileError::BranchTargetOutOfRange { value } => {
            outln!("Branch Target Out of Range");
            outln!("  Branch target {value} is outside the valid u16 range.");
        }
        CompileError::BackwardBranchTarget { step, target } => {
            outln!("Backward Branch Target");
            outln!("  Step {step} branches to {target}, but forward branches are required.");
        }
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive,
            field,
            value,
            limit,
        } => {
            outln!("Primitive Limit Exceeded");
            outln!(
                "  Primitive '{primitive}' field '{field}' value {value} exceeds limit {limit}."
            );
        }
        CompileError::LastStepMustFinish => {
            outln!("Last Step Must Finish");
            outln!("  The final step in a linear workflow must be a 'finish' step.");
        }
        CompileError::UnsupportedConstantValue { step } => {
            outln!("Unsupported Constant Value");
            outln!("  Step {step} constant value must be a scalar YAML value.");
        }
        CompileError::UnknownReferenceRoot { reference, root } => {
            outln!("Unknown Reference Root");
            outln!("  Reference '{reference}' uses unknown root '{root}'.");
        }
        CompileError::IllegalReference { reference } => {
            outln!("Illegal Reference");
            outln!("  Reference '{reference}' is not allowed in deterministic workflows.");
        }
        CompileError::UnknownReferenceName {
            kind,
            reference,
            name,
        } => {
            outln!("Unknown Reference");
            outln!("  Reference '{reference}' refers to unknown {kind} '{name}'.");
        }
        CompileError::UnsupportedAccessorReference {
            reference,
            root,
            path,
        } => {
            outln!("Unsupported Accessor Reference");
            outln!(
                "  Accessor reference '{reference}' (root: {root}, path: {path}) is not supported."
            );
        }
        CompileError::UnknownStepTarget { step, target } => {
            outln!("Unknown Step Target");
            outln!("  Step {step} branches to undeclared step index {target}.");
        }
        CompileError::UnreachableStep { step } => {
            outln!("Unreachable Step");
            outln!("  Step {step} cannot be reached from the workflow entry point.");
        }
        CompileError::TypeMismatch {
            field,
            expected,
            found,
        } => {
            outln!("Type Mismatch");
            outln!("  Field '{field}': expected {expected}, but found {found}.");
        }
        CompileError::Workflow(e) => {
            outln!("Workflow IR Validation Error");
            outln!("  {e}");
        }
        CompileError::Validation(e) => {
            super::explain_validation::explain_validation_error(e);
        }
        _ => {
            outln!("Compilation Error");
            outln!("  {err}");
        }
    }
    super::explain_reports::explain_compile_repair_hint(err);
}
