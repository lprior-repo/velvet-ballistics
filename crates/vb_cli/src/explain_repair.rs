#![forbid(unsafe_code)]
//! Repair hints and verification failure formatting.

use std::process::ExitCode;
use crate::args::{OutputFormat, ParseError};
use crate::exit_code::CliExitCode;
use crate::output::{json_error, json_out, output_error_exit, write_stdout_line, write_stderr_line, write_failure_message};
use crate::output_utils::*;
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};

pub(crate) fn explain_compile_repair_hint(err: &vb_compile::CompileError) {
    use vb_compile::CompileError;
    let hints: &[&str] = match err {
        CompileError::SourceTooLarge { .. } => &[
            "Split the workflow into smaller sub-workflows",
            "Remove unnecessary comments or whitespace",
        ],
        CompileError::EmptySource => &[
            "Add a Velvet v1 workflow YAML document",
            "Ensure the file is not empty",
        ],
        CompileError::Parse(_) => &[
            "Fix YAML syntax: use spaces for indentation, check quote matching",
            "Validate the YAML with an external parser before compiling",
        ],
        CompileError::DocumentCount { .. } => &[
            "Remove extra YAML document separators (---)",
            "Keep exactly one YAML document per workflow file",
        ],
        CompileError::TopLevelNotMapping => &[
            "Start the workflow with a YAML mapping (key-value pairs)",
            "Example: `name: my-workflow` as the first line",
        ],
        CompileError::NonStringKey { .. } => &[
            "Ensure all mapping keys are quoted strings",
            "YAML keys must be either bare identifiers or quoted strings",
        ],
        CompileError::DuplicateKey { .. } => &[
            "Remove duplicate keys from the YAML mapping",
            "Each key must appear exactly once at its level",
        ],
        CompileError::AliasForbidden { .. } => &[
            "Replace YAML aliases (&alias) with inline values",
            "Velvet workflows do not support YAML anchors or aliases",
        ],
        CompileError::AnchorForbidden { .. } => &[
            "Replace YAML anchors (*alias) with inline values",
            "Velvet workflows do not support YAML anchors or aliases",
        ],
        CompileError::MergeKeyForbidden { .. } => &[
            "Remove merge keys (<<:) from the YAML",
            "Velvet workflows do not support YAML merge keys",
        ],
        CompileError::TagForbidden { .. } => &[
            "Remove YAML tags (!tag) from the document",
            "Velvet workflows do not support YAML tags",
        ],
        CompileError::BadValue => &[
            "Fix the malformed YAML scalar value",
            "Ensure strings are properly quoted if they contain special characters",
        ],
        CompileError::FloatForbidden => &[
            "Replace floating-point numbers with integers or strings",
            "Velvet workflows do not allow float YAML scalars",
        ],
        CompileError::DepthLimit { .. } => &[
            "Reduce nesting depth by flattening the workflow structure",
            "Split nested steps into separate workflow files",
        ],
        CompileError::NodeLimit { .. } => &[
            "Reduce the number of workflow nodes",
            "Split the workflow into multiple smaller workflows",
        ],
        CompileError::SequenceLimit { .. } => &[
            "Shorten the sequence by removing items",
            "Split the sequence into multiple smaller sequences",
        ],
        CompileError::MappingLimit { .. } => &[
            "Reduce the number of entries in the mapping",
            "Split into multiple YAML documents or separate files",
        ],
        CompileError::ScalarLimit { .. } => &[
            "Shorten the scalar value",
            "Move long strings to a separate data file and reference them",
        ],
        CompileError::MissingField { .. } => &[
            "Add the missing required field to the workflow",
            "Check the Velvet v1 schema for required fields",
        ],
        CompileError::UnknownTopLevelField { .. } => &[
            "Remove the unknown field or check for typos",
            "Consult the Velvet v1 schema for valid top-level fields",
        ],
        CompileError::InvalidVersion { .. } => &[
            "Set version to 'velvet-ballistics/v1'",
            "The version field is required at the top level",
        ],
        CompileError::InvalidTriggerCount { .. } => &[
            "Define exactly one trigger in the workflow",
            "Remove extra triggers or merge them into one",
        ],
        CompileError::UnknownTriggerKind { .. } => &[
            "Use a known trigger kind: manual, schedule, or webhook",
            "Check the Velvet v1 schema for valid trigger types",
        ],
        CompileError::TriggerShape { .. } => &[
            "Fix the trigger structure according to the Velvet v1 schema",
            "Triggers must be a mapping with kind and other fields",
        ],
        CompileError::UnknownTriggerField { .. } => &[
            "Remove the unknown trigger field or check for typos",
            "Consult the Velvet v1 schema for valid trigger fields",
        ],
        CompileError::MissingTriggerField { .. } => &[
            "Add the missing required field to the trigger",
            "Check the Velvet v1 schema for required trigger fields",
        ],
        CompileError::InvalidTriggerField { .. } => &[
            "Fix the trigger field value to match the expected type",
            "Consult the Velvet v1 schema for field types",
        ],
        CompileError::FieldShape { .. } => &[
            "Fix the field structure to match the expected shape",
            "Check the Velvet v1 schema for field structures",
        ],
        CompileError::UnknownInputSchemaField { .. } => &[
            "Remove the unknown input schema field or check for typos",
            "Consult the Velvet v1 schema for valid input schema fields",
        ],
        CompileError::InvalidInputSchema { .. } => &[
            "Fix the input schema field to match the expected type",
            "Check the Velvet v1 schema for input schema field types",
        ],
        CompileError::UnsupportedTopLevelResult => &[
            "Remove the top-level result mapping",
            "Results are computed by steps, not declared at the top level",
        ],
        CompileError::EmptySteps => &[
            "Add at least one executable step to the workflow",
            "Steps define what the workflow actually does",
        ],
        CompileError::InvalidName { .. } => &[
            "Use valid Velvet identifiers: lowercase letters, digits, hyphens",
            "Names must start with a letter",
        ],
        CompileError::MissingStepId { .. } => &[
            "Add an 'id' field to the step",
            "Each step must have a unique identifier",
        ],
        CompileError::DuplicateStepId { .. } => {
            &["Give each step a unique ID", "Remove duplicate step IDs"]
        }
        CompileError::StepShape { .. } => &[
            "Make each step a YAML mapping",
            "Steps must be key-value pairs with at least an 'id' and a primitive",
        ],
        CompileError::UnknownStepField { .. } => &[
            "Remove the unknown step field or check for typos",
            "Consult the Velvet v1 schema for valid step fields",
        ],
        CompileError::UnknownStepPrimitiveField { .. } => &[
            "Remove the unknown primitive field or check for typos",
            "Consult the Velvet v1 schema for valid primitive fields",
        ],
        CompileError::MissingStepPrimitive { .. } => &[
            "Add a primitive action to the step (e.g., 'do', 'ask', 'wait')",
            "Each step must have at least one primitive action",
        ],
        CompileError::MultipleStepPrimitives { .. } => &[
            "Keep only one primitive action per step",
            "Split multiple actions into separate steps",
        ],
        CompileError::UnsupportedStepPrimitive { .. } => &[
            "Use a supported primitive: do, ask, wait, finish, retry, parallel, etc.",
            "Check the Velvet v1 schema for supported primitives",
        ],
        CompileError::UnsupportedStepControlField { .. } => &[
            "Remove the unsupported control field",
            "Check the Velvet v1 schema for valid control fields",
        ],
        CompileError::MissingStepField { .. } => &[
            "Add the missing required field to the step",
            "Check the Velvet v1 schema for required step fields",
        ],
        CompileError::StepFieldShape { .. } => &[
            "Fix the step field structure",
            "Check the Velvet v1 schema for field structures",
        ],
        CompileError::StepIndexOutOfRange { .. } => &[
            "Reduce the step index to fit within u16 range",
            "Step indices must be between 0 and 65535",
        ],
        CompileError::SlotIndexOutOfRange { .. } => &[
            "Reduce slot indices to fit within u16 range",
            "Slot indices must be between 0 and 65535",
        ],
        CompileError::BranchTargetOutOfRange { .. } => &[
            "Fix branch targets to reference valid step indices",
            "Branch targets must be valid step indices in the workflow",
        ],
        CompileError::BackwardBranchTarget { .. } => &[
            "Change the branch target to a later step",
            "Forward branches are required in Velvet workflows",
        ],
        CompileError::PrimitiveLoweringLimitExceeded { .. } => &[
            "Reduce the field value to within the limit",
            "Check the Velvet v1 schema for field limits",
        ],
        CompileError::LastStepMustFinish => &[
            "Make the last step a 'finish' primitive",
            "Linear workflows must end with a finish step",
        ],
        CompileError::UnsupportedConstantValue { .. } => &[
            "Use a scalar YAML value (string, number, boolean)",
            "Remove complex nested structures from constant values",
        ],
        CompileError::UnknownReferenceRoot { .. } => &[
            "Use a known reference root: slot, input, env, secrets",
            "Check the Velvet v1 schema for valid reference roots",
        ],
        CompileError::IllegalReference { .. } => &[
            "Remove illegal references",
            "References to runtime state are not allowed in deterministic contexts",
        ],
        CompileError::UnknownReferenceName { .. } => &[
            "Declare the referenced name in the workflow",
            "Check for typos in the reference name",
        ],
        CompileError::UnsupportedAccessorReference { .. } => &[
            "Use a supported accessor format",
            "Check the Velvet v1 schema for accessor syntax",
        ],
        CompileError::UnknownStepTarget { .. } => &[
            "Fix branch targets to reference declared step indices",
            "All branch targets must exist in the workflow",
        ],
        CompileError::UnreachableStep { .. } => &[
            "Connect the unreachable step to the control flow",
            "Remove the unreachable step or add a branch to it",
        ],
        CompileError::TypeMismatch { .. } => &[
            "Fix the type to match the expected type",
            "Check the Velvet v1 schema for type requirements",
        ],
        CompileError::Workflow(_) | CompileError::Validation(_) => &[
            "Fix the workflow or validation error shown above",
            "Review the specific error message for details",
        ],
        _ => &[
            "Review the error message above for details",
            "Check the Velvet v1 schema for correct usage",
        ],
    };
    explain_repair_hint("compilation", hints);
}

/// Emit a structured repair hint header.
pub(crate) fn explain_repair_hint(context: &str, hints: &[&str]) {
    crate::outln!("");
    crate::outln!("Repair hints ({context}):");
    for hint in hints {
        crate::outln!("  - {hint}");
    }
}

/// Explain why a verification gate passed.
pub(crate) fn explain_gate_pass(gate: &str) {
    crate::outln!("  ✓ {gate}");
}

