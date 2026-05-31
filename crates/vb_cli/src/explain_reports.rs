//! Error and verification failure report formatting.
    use vb_compile::CompileError;
    match err {
        CompileError::SourceTooLarge { actual, limit } => {
            outln!("Source Too Large");
            outln!("  The workflow YAML source is {actual} bytes, exceeds limit of {limit}.");
        }
        CompileError::EmptySource => {
            outln!("Empty Source");
            outln!("  The workflow file contains no YAML document.");
        }
        CompileError::Parse(e) => {
            outln!("YAML Parse Error");
            outln!("  The YAML parser rejected the document: {e}");
        }
        CompileError::DocumentCount { count } => {
            outln!("Multiple YAML Documents");
            outln!("  Expected exactly one YAML document, but found {count}.");
        }
        CompileError::TopLevelNotMapping => {
            outln!("Invalid Top-Level Structure");
            outln!("  The top-level YAML document must be a mapping.");
        }
        CompileError::NonStringKey { mark } => {
            outln!("Non-String Key");
            outln!("  A mapping key at position {mark:?} is not a string.");
        }
        CompileError::DuplicateKey { key, mark } => {
            outln!("Duplicate Key");
            outln!("  The YAML mapping contains duplicate key '{key}' at {mark:?}.");
        }
        CompileError::AliasForbidden { mark } => {
            outln!("YAML Alias Forbidden");
            outln!("  YAML aliases are not allowed at {mark:?}.");
        }
        CompileError::AnchorForbidden { mark } => {
            outln!("YAML Anchor Forbidden");
            outln!("  YAML anchors are not allowed at {mark:?}.");
        }
        CompileError::MergeKeyForbidden { mark } => {
            outln!("YAML Merge Key Forbidden");
            outln!("  YAML merge keys are not allowed at {mark:?}.");
        }
        CompileError::TagForbidden { mark } => {
            outln!("YAML Tag Forbidden");
            outln!("  YAML tags are not allowed at {mark:?}.");
        }
        CompileError::BadValue => {
            outln!("Invalid YAML Scalar");
            outln!("  A YAML scalar value is malformed.");
        }
        CompileError::FloatForbidden => {
            outln!("Floating-Point Numbers Forbidden");
            outln!("  Floating-point YAML scalars are not allowed.");
        }
        CompileError::DepthLimit { depth, limit } => {
            outln!("Nesting Depth Exceeded");
            outln!("  YAML nesting depth of {depth} exceeds limit of {limit}.");
        }
        CompileError::NodeLimit { limit } => {
            outln!("YAML Node Limit Exceeded");
            outln!("  The workflow exceeds node limit of {limit}.");
        }
        CompileError::SequenceLimit { actual, limit } => {
            outln!("Sequence Too Long");
            outln!("  A sequence has {actual} items, exceeding limit of {limit}.");
        }
        CompileError::MappingLimit { actual, limit } => {
            outln!("Mapping Too Large");
            outln!("  A mapping has {actual} entries, exceeding limit of {limit}.");
        }
        CompileError::ScalarLimit { actual, limit } => {
            outln!("Scalar Too Long");
            outln!("  A scalar is {actual} chars, exceeding limit of {limit}.");
        }
        CompileError::MissingField { field } => {
            outln!("Missing Required Field");
            outln!("  Required workflow field '{field}' is missing.");
        }
        CompileError::UnknownTopLevelField { field } => {
            outln!("Unknown Workflow Field");
            outln!("  '{field}' is not a recognized Velvet workflow field.");
        }
        CompileError::InvalidVersion { actual } => {
            outln!("Invalid Workflow Version");
            outln!("  Found version '{actual}', but Velvet v1 requires 'velvet-ballistics/v1'.");
        }
        CompileError::InvalidTriggerCount { count } => {
            outln!("Invalid Trigger Count");
            outln!("  Workflow must declare exactly one trigger, but found {count}.");
        }
        CompileError::UnknownTriggerKind { trigger } => {
            outln!("Unknown Trigger Kind");
            outln!("  Trigger kind '{trigger}' is not recognized.");
        }
        CompileError::TriggerShape {
            trigger,
            expected: _,
        } => {
            outln!("Invalid Trigger Shape");
            outln!("  Trigger '{trigger}' has the wrong structure.");
        }
        CompileError::UnknownTriggerField { trigger, field } => {
            outln!("Unknown Trigger Field");
            outln!("  Trigger '{trigger}' has unknown field '{field}'.");
        }
        CompileError::MissingTriggerField { trigger, field } => {
            outln!("Missing Trigger Field");
            outln!("  Trigger '{trigger}' is missing required field '{field}'.");
        }
        CompileError::InvalidTriggerField {
            trigger,
            field,
            expected: _,
        } => {
            outln!("Invalid Trigger Field");
            outln!("  Trigger '{trigger}' field '{field}' is invalid.");
        }
        CompileError::FieldShape { field, expected: _ } => {
            outln!("Invalid Field Shape");
            outln!("  Field '{field}' has the wrong structure.");
        }
        CompileError::UnknownInputSchemaField { field } => {
            outln!("Unknown Input Schema Field");
            outln!("  '{field}' is not a recognized input schema field.");
        }
        CompileError::InvalidInputSchema { field, expected: _ } => {
            outln!("Invalid Input Schema");
            outln!("  Input schema field '{field}' is invalid.");
        }
        CompileError::UnsupportedTopLevelResult => {
            outln!("Unsupported Top-Level Result");
            outln!("  Non-empty top-level result mapping is not supported.");
        }
        CompileError::EmptySteps => {
            outln!("Empty Steps");
            outln!("  Workflow must contain at least one executable step.");
        }
        CompileError::InvalidName { field, value } => {
            outln!("Invalid Name");
            outln!("  '{value}' is not a valid Velvet v1 name for {field}.");
        }
        CompileError::MissingStepId { step } => {
            outln!("Missing Step ID");
            outln!("  Step at index {step} is missing its required 'id' field.");
        }
        CompileError::DuplicateStepId { id } => {
            outln!("Duplicate Step ID");
            outln!("  Step ID '{id}' appears more than once in the workflow.");
        }
        CompileError::StepShape { step } => {
            outln!("Invalid Step Shape");
            outln!("  Step at index {step} must be a YAML mapping.");
        }
        CompileError::UnknownStepField { step, field } => {
            outln!("Unknown Step Field");
            outln!("  Step {step} has unknown field '{field}'.");
        }
        CompileError::UnknownStepPrimitiveField {
            step,
            primitive,
            field,
        } => {
            outln!("Unknown Primitive Field");
            outln!("  Step {step} primitive '{primitive}' has unknown field '{field}'.");
        }
        CompileError::MissingStepPrimitive { step } => {
            outln!("Missing Step Primitive");
            outln!("  Step {step} is missing a primitive action.");
        }
        CompileError::MultipleStepPrimitives { step } => {
            outln!("Multiple Step Primitives");
            outln!("  Step {step} contains multiple primitive fields.");
        }
        CompileError::UnsupportedStepPrimitive { step, primitive } => {
            outln!("Unsupported Step Primitive");
            outln!("  Step {step} primitive '{primitive}' is not supported.");
        }
        CompileError::UnsupportedStepControlField { step, field } => {
            outln!("Unsupported Step Control Field");
            outln!("  Step {step} control field '{field}' is not supported.");
        }
        CompileError::MissingStepField { step, field } => {
            outln!("Missing Step Field");
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
            explain_validation_error(e);
        }
        _ => {
            outln!("Compilation Error");
            outln!("  {err}");
        }
    }
    explain_compile_repair_hint(err);
}

/// Emit a structured repair hint for compilation errors.
pub(crate) fn explain_compile_repair_hint(err: &vb_compile::CompileError) {
