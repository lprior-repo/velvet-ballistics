#![forbid(unsafe_code)]
//! Explain workflow compilation errors.
//!
//! This module owns the human-readable explanation of every
//! [`vb_compile::CompileError`] variant.  The `explain_error` function is
//! a single exhaustive `match` that formats each error kind for the
//! user and delegates repair hints to `explain_reports`.

use crate::explain_repair::explain_compile_repair_hint;
use crate::explain_validation::validation::explain_validation_error;

/// Explain a [`vb_compile::CompileError`] in human-readable form.
///
/// This is the canonical error formatter for the compile phase.
pub(crate) fn explain_error(err: &vb_compile::CompileError) {
    use vb_compile::CompileError;
    match err {
        CompileError::SourceTooLarge { actual, limit } => {
            crate::outln!("Source Too Large");
            crate::outln!(
                "  The workflow YAML source is {actual} bytes, exceeds limit of {limit}."
            );
        }
        CompileError::EmptySource => {
            crate::outln!("Empty Source");
            crate::outln!("  The workflow file contains no YAML document.");
        }
        CompileError::Parse(e) => {
            crate::outln!("YAML Parse Error");
            crate::outln!("  The YAML parser rejected the document: {e}");
        }
        CompileError::DocumentCount { count } => {
            crate::outln!("Multiple YAML Documents");
            crate::outln!("  Expected exactly one YAML document, but found {count}.");
        }
        CompileError::TopLevelNotMapping => {
            crate::outln!("Invalid Top-Level Structure");
            crate::outln!("  The top-level YAML document must be a mapping.");
        }
        CompileError::NonStringKey { mark } => {
            crate::outln!("Non-String Key");
            crate::outln!("  A mapping key at position {mark:?} is not a string.");
        }
        CompileError::DuplicateKey { key, mark } => {
            crate::outln!("Duplicate Key");
            crate::outln!("  The YAML mapping contains duplicate key '{key}' at {mark:?}.");
        }
        CompileError::AliasForbidden { mark } => {
            crate::outln!("YAML Alias Forbidden");
            crate::outln!("  YAML aliases are not allowed at {mark:?}.");
        }
        CompileError::AnchorForbidden { mark } => {
            crate::outln!("YAML Anchor Forbidden");
            crate::outln!("  YAML anchors are not allowed at {mark:?}.");
        }
        CompileError::MergeKeyForbidden { mark } => {
            crate::outln!("YAML Merge Key Forbidden");
            crate::outln!("  YAML merge keys are not allowed at {mark:?}.");
        }
        CompileError::TagForbidden { mark } => {
            crate::outln!("YAML Tag Forbidden");
            crate::outln!("  YAML tags are not allowed at {mark:?}.");
        }
        CompileError::BadValue => {
            crate::outln!("Invalid YAML Scalar");
            crate::outln!("  A YAML scalar value is malformed.");
        }
        CompileError::FloatForbidden => {
            crate::outln!("Floating-Point Numbers Forbidden");
            crate::outln!("  Floating-point YAML scalars are not allowed.");
        }
        CompileError::DepthLimit { depth, limit } => {
            crate::outln!("Nesting Depth Exceeded");
            crate::outln!("  YAML nesting depth of {depth} exceeds limit of {limit}.");
        }
        CompileError::NodeLimit { limit } => {
            crate::outln!("YAML Node Limit Exceeded");
            crate::outln!("  The workflow exceeds node limit of {limit}.");
        }
        CompileError::SequenceLimit { actual, limit } => {
            crate::outln!("Sequence Too Long");
            crate::outln!("  A sequence has {actual} items, exceeding limit of {limit}.");
        }
        CompileError::MappingLimit { actual, limit } => {
            crate::outln!("Mapping Too Large");
            crate::outln!("  A mapping has {actual} entries, exceeding limit of {limit}.");
        }
        CompileError::ScalarLimit { actual, limit } => {
            crate::outln!("Scalar Too Long");
            crate::outln!("  A scalar is {actual} chars, exceeding limit of {limit}.");
        }
        CompileError::MissingField { field } => {
            crate::outln!("Missing Required Field");
            crate::outln!("  Required workflow field '{field}' is missing.");
        }
        CompileError::UnknownTopLevelField { field } => {
            crate::outln!("Unknown Workflow Field");
            crate::outln!("  '{field}' is not a recognized Velvet workflow field.");
        }
        CompileError::InvalidVersion { actual } => {
            crate::outln!("Invalid Workflow Version");
            crate::outln!(
                "  Found version '{actual}', but Velvet v1 requires 'velvet-ballistics/v1'."
            );
        }
        CompileError::InvalidTriggerCount { count } => {
            crate::outln!("Invalid Trigger Count");
            crate::outln!("  Workflow must declare exactly one trigger, but found {count}.");
        }
        CompileError::UnknownTriggerKind { trigger } => {
            crate::outln!("Unknown Trigger Kind");
            crate::outln!("  Trigger kind '{trigger}' is not recognized.");
        }
        CompileError::TriggerShape {
            trigger,
            expected: _,
        } => {
            crate::outln!("Invalid Trigger Shape");
            crate::outln!("  Trigger '{trigger}' has the wrong structure.");
        }
        CompileError::UnknownTriggerField { trigger, field } => {
            crate::outln!("Unknown Trigger Field");
            crate::outln!("  Trigger '{trigger}' has unknown field '{field}'.");
        }
        CompileError::MissingTriggerField { trigger, field } => {
            crate::outln!("Missing Trigger Field");
            crate::outln!("  Trigger '{trigger}' is missing required field '{field}'.");
        }
        CompileError::InvalidTriggerField {
            trigger,
            field,
            expected: _,
        } => {
            crate::outln!("Invalid Trigger Field");
            crate::outln!("  Trigger '{trigger}' field '{field}' is invalid.");
        }
        CompileError::FieldShape { field, expected: _ } => {
            crate::outln!("Invalid Field Shape");
            crate::outln!("  Field '{field}' has the wrong structure.");
        }
        CompileError::UnknownInputSchemaField { field } => {
            crate::outln!("Unknown Input Schema Field");
            crate::outln!("  '{field}' is not a recognized input schema field.");
        }
        CompileError::InvalidInputSchema { field, expected: _ } => {
            crate::outln!("Invalid Input Schema");
            crate::outln!("  Input schema field '{field}' is invalid.");
        }
        CompileError::UnsupportedTopLevelResult => {
            crate::outln!("Unsupported Top-Level Result");
            crate::outln!("  Non-empty top-level result mapping is not supported.");
        }
        CompileError::EmptySteps => {
            crate::outln!("Empty Steps");
            crate::outln!("  Workflow must contain at least one executable step.");
        }
        CompileError::InvalidName { field, value } => {
            crate::outln!("Invalid Name");
            crate::outln!("  '{value}' is not a valid Velvet v1 name for {field}.");
        }
        CompileError::MissingStepId { step } => {
            crate::outln!("Missing Step ID");
            crate::outln!("  Step at index {step} is missing its required 'id' field.");
        }
        CompileError::DuplicateStepId { id } => {
            crate::outln!("Duplicate Step ID");
            crate::outln!("  Step ID '{id}' appears more than once in the workflow.");
        }
        CompileError::StepShape { step } => {
            crate::outln!("Invalid Step Shape");
            crate::outln!("  Step at index {step} must be a YAML mapping.");
        }
        CompileError::UnknownStepField { step, field } => {
            crate::outln!("Unknown Step Field");
            crate::outln!("  Step {step} has unknown field '{field}'.");
        }
        CompileError::UnknownStepPrimitiveField {
            step,
            primitive,
            field,
        } => {
            crate::outln!("Unknown Primitive Field");
            crate::outln!("  Step {step} primitive '{primitive}' has unknown field '{field}'.");
        }
        CompileError::MissingStepPrimitive { step } => {
            crate::outln!("Missing Step Primitive");
            crate::outln!("  Step {step} is missing a primitive action.");
        }
        CompileError::MultipleStepPrimitives { step } => {
            crate::outln!("Multiple Step Primitives");
            crate::outln!("  Step {step} contains multiple primitive fields.");
        }
        CompileError::UnsupportedStepPrimitive { step, primitive } => {
            crate::outln!("Unsupported Step Primitive");
            crate::outln!("  Step {step} primitive '{primitive}' is not supported.");
        }
        CompileError::UnsupportedStepControlField { step, field } => {
            crate::outln!("Unsupported Step Control Field");
            crate::outln!("  Step {step} control field '{field}' is not supported.");
        }
        CompileError::MissingStepField { step, field } => {
            crate::outln!("Missing Step Field");
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
            explain_validation_error(e);
        }
        _ => {
            crate::outln!("Compilation Error");
            crate::outln!("  {err}");
        }
    }
    explain_compile_repair_hint(err);
}
