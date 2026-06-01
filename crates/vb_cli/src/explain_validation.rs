#![forbid(unsafe_code)]
//! Validation error explanation and failure formatting.

fn explain_verification_failure(err: &commands_verify::VerifyError) {
    use commands_verify::VerifyError;
    match err {
        VerifyError::YamlParse(msg) => {
            outln!("YAML Parse Error:");
            outln!("  {msg}");
            outln!("");
            explain_repair_hint(
                "yaml_parse",
                &[
                    "Fix YAML syntax: use spaces for indentation, not tabs",
                    "Ensure all quotes are matched",
                    "Validate the YAML with an external parser",
                ],
            );
        }
        VerifyError::Compile(errors) => {
            outln!("Compilation Error:");
            for e in errors {
                outln!("  - {e}");
            }
            outln!("");
            explain_repair_hint(
                "compilation",
                &[
                    "Fix the compilation errors shown above",
                    "Review the Velvet v1 schema for correct field types",
                ],
            );
        }
        VerifyError::IrValidation(msg) => {
            outln!("IR Validation Error:");
            outln!("  {msg}");
            outln!("");
            explain_repair_hint(
                "ir_validation",
                &[
                    "The compiled workflow has an invalid internal structure",
                    "This usually indicates a bug in the compiler",
                    "Try re-compiling the workflow from source",
                ],
            );
        }
        VerifyError::BudgetPolicy(msg) => {
            outln!("Budget Policy Violation:");
            outln!("  {msg}");
            outln!("");
            explain_repair_hint(
                "budget_policy",
                &[
                    "Reduce the workflow's resource consumption",
                    "Simplify step logic or reduce step count",
                    "Use 'vb verify --profile quick' for faster iteration",
                    "Review the budget policy in the Velvet documentation",
                ],
            );
        }
        VerifyError::StorageError(msg) => {
            outln!("Storage Error:");
            outln!("  {msg}");
            outln!("");
            explain_repair_hint(
                "storage",
                &[
                    "Check that the storage path exists and is writable",
                    "Ensure sufficient disk space is available",
                ],
            );
        }
        VerifyError::ReplayDivergence(msg) => {
            outln!("Replay Divergence:");
            outln!("  {msg}");
            outln!("");
            explain_repair_hint(
                "replay",
                &[
                    "The workflow produces different results on replay",
                    "Ensure all actions are deterministic or properly handled",
                    "Check for non-deterministic data sources",
                ],
            );
        }
    }
}


fn explain_validation_error(err: &vb_validate::ValidationError) {
    use vb_validate::ValidationError;
    match err {
        ValidationError::DuplicateKey => {
            outln!("Duplicate Key");
            outln!("  A YAML mapping contains duplicate keys, which is not allowed.");
            explain_repair_hint(
                "validation",
                &[
                    "Find and remove duplicate YAML keys",
                    "Each key must be unique at its nesting level",
                ],
            );
        }
        ValidationError::ForbiddenYamlFeature => {
            outln!("Forbidden YAML Feature");
            outln!("  The workflow uses a YAML feature that is not allowed in Velvet.");
            explain_repair_hint(
                "validation",
                &[
                    "Remove YAML anchors, aliases, merge keys, or tags",
                    "Velvet does not support these YAML features",
                ],
            );
        }
        ValidationError::UnknownTopLevelField => {
            outln!("Unknown Top-Level Field");
            outln!("  The workflow contains an unrecognized top-level field.");
            explain_repair_hint(
                "validation",
                &[
                    "Remove or rename the unknown field",
                    "Valid top-level fields: name, version, trigger, steps, input_schema, output_schema",
                ],
            );
        }
        ValidationError::UnknownStepField => {
            outln!("Unknown Step Field");
            outln!("  A step contains an unrecognized field.");
            explain_repair_hint(
                "validation",
                &[
                    "Remove or fix the unknown step field",
                    "Check the Velvet v1 schema for valid step fields",
                ],
            );
        }
        ValidationError::MissingRequiredField { field } => {
            outln!("Missing Required Field");
            outln!("  Required field '{field}' is missing from the workflow.");
            explain_repair_hint(
                "validation",
                &[
                    "Add the missing required field to the workflow",
                    "Check the Velvet v1 schema for required fields",
                ],
            );
        }
        ValidationError::InvalidVersion { version } => {
            outln!("Invalid Version");
            outln!("  Found version '{version}', but Velvet v1 requires 'velvet-ballistics/v1'.");
            explain_repair_hint(
                "validation",
                &[
                    "Set version to 'velvet-ballistics/v1'",
                    "The version field is required and must be the Velvet v1 identifier",
                ],
            );
        }
        ValidationError::InvalidId { id } => {
            outln!("Invalid Identifier");
            outln!("  '{id}' is not a valid Velvet identifier.");
            explain_repair_hint(
                "validation",
                &[
                    "Use valid Velvet identifiers: lowercase letters, digits, hyphens",
                    "Identifiers must start with a letter",
                ],
            );
        }
        ValidationError::ReservedId { id } => {
            outln!("Reserved Identifier");
            outln!("  '{id}' is a reserved identifier and cannot be used.");
            explain_repair_hint(
                "validation",
                &[
                    "Choose a different identifier",
                    "Avoid using reserved words as identifiers",
                ],
            );
        }
        ValidationError::DuplicateId { id } => {
            outln!("Duplicate Identifier");
            outln!("  The identifier '{id}' appears more than once.");
            explain_repair_hint(
                "validation",
                &[
                    "Give each identifier a unique name",
                    "Remove duplicate identifier declarations",
                ],
            );
        }
        ValidationError::MultipleStepPrimitives => {
            outln!("Multiple Step Primitives");
            outln!("  A step contains multiple primitive actions.");
            explain_repair_hint(
                "validation",
                &[
                    "Split the step into multiple separate steps",
                    "Each step should have exactly one primitive action",
                ],
            );
        }
        ValidationError::MissingStepPrimitive => {
            outln!("Missing Step Primitive");
            outln!("  A step is missing its primitive action.");
            explain_repair_hint(
                "validation",
                &[
                    "Add a primitive action to the step (e.g., 'do', 'ask', 'wait')",
                    "Each step must have at least one primitive",
                ],
            );
        }
        ValidationError::UnknownReference { reference } => {
            outln!("Unknown Reference");
            outln!("  Reference '{reference}' is not declared in the workflow.");
            explain_repair_hint(
                "validation",
                &[
                    "Declare the reference or check the spelling",
                    "References must be defined before use",
                ],
            );
        }
        ValidationError::FutureReference { reference } => {
            outln!("Future Reference");
            outln!("  Reference '{reference}' refers to a step that hasn't been defined yet.");
            explain_repair_hint(
                "validation",
                &[
                    "Move the reference to after the step it refers to",
                    "References can only point to previously defined steps",
                ],
            );
        }
        ValidationError::SecretNotDeclared { secret } => {
            outln!("Undeclared Secret");
            outln!("  Secret '{secret}' is referenced but not declared in the workflow secrets.");
            explain_repair_hint(
                "validation",
                &[
                    "Add the secret to the workflow's secrets section",
                    "Secrets must be declared before they can be referenced",
                ],
            );
        }
        ValidationError::DirectRuntimeReference => {
            outln!("Direct Runtime Reference");
            outln!("  References to runtime state are not allowed in this context.");
            explain_repair_hint(
                "validation",
                &[
                    "Remove the runtime reference",
                    "Use declared references instead of direct runtime access",
                ],
            );
        }
        ValidationError::InvalidThenTarget => {
            outln!("Invalid Branch Target");
            outln!("  A 'then' branch targets an invalid step.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the branch target to reference a valid step ID",
                    "Branch targets must point to existing steps",
                ],
            );
        }
        ValidationError::ControlFlowCycle => {
            outln!("Control Flow Cycle");
            outln!("  The workflow contains a cycle in its control flow graph.");
            explain_repair_hint(
                "validation",
                &[
                    "Remove cyclic dependencies between steps",
                    "Break cycles by introducing suspension points",
                    "Consider using 'choose' for conditional branching instead",
                ],
            );
        }
        ValidationError::UnreachableStep { step } => {
            outln!("Unreachable Step");
            outln!("  Step '{step}' cannot be reached from the workflow entry.");
            explain_repair_hint(
                "validation",
                &[
                    "Connect the step to the control flow",
                    "Remove the unreachable step if it's not needed",
                ],
            );
        }
        ValidationError::InvalidChoose => {
            outln!("Invalid Choose");
            outln!("  The 'choose' (conditional) construct is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'choose' construct structure",
                    "Choose requires 'when' conditions and 'then' branches",
                ],
            );
        }
        ValidationError::InvalidForEach => {
            outln!("Invalid ForEach");
            outln!("  The 'for_each' loop construct is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'for_each' construct structure",
                    "ForEach requires an 'over' iterable and a 'do' body",
                ],
            );
        }
        ValidationError::InvalidTogether => {
            outln!("Invalid Together");
            outln!("  The 'together' (parallel) construct is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'together' construct structure",
                    "Together requires a 'do' block with parallel steps",
                ],
            );
        }
        ValidationError::InvalidCollect => {
            outln!("Invalid Collect");
            outln!("  The 'collect' pagination construct is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'collect' construct structure",
                    "Collect requires an 'over' iterable and pagination settings",
                ],
            );
        }
        ValidationError::InvalidReduce => {
            outln!("Invalid Reduce");
            outln!("  The 'reduce' fold construct is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'reduce' construct structure",
                    "Reduce requires 'over' iterable, 'initial', and 'do' body",
                ],
            );
        }
        ValidationError::InvalidRepeat => {
            outln!("Invalid Repeat");
            outln!("  The 'repeat' loop construct is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'repeat' construct structure",
                    "Repeat requires 'times' or 'until'/'while' conditions",
                ],
            );
        }
        ValidationError::InvalidWait => {
            outln!("Invalid Wait");
            outln!("  The 'wait' step is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'wait' step structure",
                    "Wait may require a 'for' duration or 'until' condition",
                ],
            );
        }
        ValidationError::InvalidAsk => {
            outln!("Invalid Ask");
            outln!("  The 'ask' (interaction) step is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'ask' step structure",
                    "Ask requires a 'prompt' and may have 'choices'",
                ],
            );
        }
        ValidationError::InvalidFinish => {
            outln!("Invalid Finish");
            outln!("  The 'finish' step is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'finish' step structure",
                    "Finish may require 'result' or 'error' fields",
                ],
            );
        }
        ValidationError::InvalidRetry => {
            outln!("Invalid Retry");
            outln!("  The 'retry' construct is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'retry' construct structure",
                    "Retry requires 'do' body and may have 'times' or 'until'",
                ],
            );
        }
        ValidationError::InvalidOnError => {
            outln!("Invalid OnError");
            outln!("  The 'on_error' error handler is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the 'on_error' handler structure",
                    "OnError requires 'do' body and may have 'max_attempts'",
                ],
            );
        }
        ValidationError::SecretResultLeak => {
            outln!("Secret Result Leak");
            outln!("  A secret value may be exposed in the workflow result.");
            explain_repair_hint(
                "validation",
                &[
                    "Exclude secret values from the workflow result",
                    "Use slot references that don't expose secret data",
                ],
            );
        }
        ValidationError::TypeMismatch { expected, found } => {
            outln!("Type Mismatch");
            outln!("  Expected type: {expected}");
            outln!("  Found type: {found}");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the value type to match the expected type",
                    "Check the Velvet v1 schema for type requirements",
                ],
            );
        }
        ValidationError::PayloadTooLarge => {
            outln!("Payload Too Large");
            outln!("  The workflow payload exceeds size limits.");
            explain_repair_hint(
                "validation",
                &[
                    "Reduce the workflow size by removing unnecessary content",
                    "Split the workflow into smaller sub-workflows",
                ],
            );
        }
        ValidationError::LimitRequired { resource } => {
            outln!("Limit Required");
            outln!("  Resource '{resource}' requires an explicit limit.");
            explain_repair_hint(
                "validation",
                &[
                    "Add an explicit limit for the resource",
                    "Check the Velvet v1 schema for limit requirements",
                ],
            );
        }
        ValidationError::LimitExceeded { resource } => {
            outln!("Limit Exceeded");
            outln!("  Resource '{resource}' has exceeded its configured limit.");
            explain_repair_hint(
                "validation",
                &[
                    "Increase the resource limit or reduce consumption",
                    "Check the Velvet v1 schema for limit values",
                ],
            );
        }
        ValidationError::UnsupportedTrigger { trigger } => {
            outln!("Unsupported Trigger");
            outln!("  Trigger type '{trigger}' is not supported.");
            explain_repair_hint(
                "validation",
                &[
                    "Use a supported trigger type: manual, schedule, webhook",
                    "Check the Velvet v1 schema for supported triggers",
                ],
            );
        }
        ValidationError::HttpTriggerOutOfCore => {
            outln!("HTTP Trigger Out of Core");
            outln!("  HTTP triggers are not available in the core runtime.");
            explain_repair_hint(
                "validation",
                &[
                    "Use a different trigger type for core runtime",
                    "HTTP triggers require the extended runtime",
                ],
            );
        }
        ValidationError::ExpressionStackExceeded { declared, limit } => {
            outln!("Expression Stack Exceeded");
            outln!("  Expression stack depth {declared} exceeds limit {limit}.");
            explain_repair_hint(
                "validation",
                &[
                    "Simplify nested expressions",
                    "Break complex expressions into separate steps",
                ],
            );
        }
        ValidationError::ExpressionStackMismatch {
            expr_index,
            declared,
            computed,
        } => {
            outln!("Expression Stack Mismatch");
            outln!(
                "  Expression {expr_index}: declared {declared} stack slots, computed {computed}."
            );
            explain_repair_hint(
                "validation",
                &[
                    "Fix the expression to declare the correct number of stack slots",
                    "Check expression syntax for stack manipulation operations",
                ],
            );
        }
        ValidationError::AccessorSlotOutOfRange {
            accessor_index,
            slot,
            slot_count,
        } => {
            outln!("Accessor Slot Out of Range");
            outln!(
                "  Accessor {accessor_index} references slot {slot}, but slot_count is {slot_count}."
            );
            explain_repair_hint(
                "validation",
                &[
                    "Fix the slot reference to be within slot_count",
                    "Slot indices are zero-based",
                ],
            );
        }
        ValidationError::AccessorPathInvalid {
            accessor_index,
            segment_index,
        } => {
            outln!("Accessor Path Invalid");
            outln!("  Accessor {accessor_index} has invalid segment at index {segment_index}.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the accessor path syntax",
                    "Check the Velvet v1 schema for accessor path format",
                ],
            );
        }
        ValidationError::SlotReferenceOutOfRange {
            slot,
            slot_count,
            context,
        } => {
            outln!("Slot Reference Out of Range");
            outln!(
                "  Slot {slot} is out of range (slot_count={slot_count}) in context: {context}."
            );
            explain_repair_hint(
                "validation",
                &[
                    "Fix the slot reference to be within the valid range",
                    "Ensure the slot exists in the workflow's slot schema",
                ],
            );
        }
        ValidationError::LoopBodyStepOutOfRange {
            step,
            node_count,
            source_node,
            label: _,
        } => {
            outln!("Loop Body Step Out of Range");
            outln!(
                "  Step {step}: loop body step out of range (node_count={node_count}, source_node={source_node})."
            );
            explain_repair_hint(
                "validation",
                &[
                    "Fix loop body step references to be within node_count",
                    "Ensure loop body steps exist in the workflow",
                ],
            );
        }
        ValidationError::SlotDependencyCycle { slot, chain } => {
            outln!("Slot Dependency Cycle");
            outln!("  Slot {slot} has a dependency cycle: {chain}.");
            explain_repair_hint(
                "validation",
                &[
                    "Break the slot dependency cycle",
                    "Remove circular dependencies between slots",
                ],
            );
        }
        ValidationError::NodeKindConstraintViolation { node_index, detail } => {
            outln!("Node Kind Constraint Violation");
            outln!("  Node {node_index}: {detail}.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the node to comply with its kind constraints",
                    "Check the Velvet v1 schema for node kind rules",
                ],
            );
        }
        ValidationError::ActionContractMissing {
            action_id,
            node_index,
        } => {
            outln!("Action Contract Missing");
            outln!(
                "  Do node {node_index} references action_id {action_id}, which has no contract."
            );
            explain_repair_hint(
                "validation",
                &[
                    "Register an action contract for action_id {action_id}",
                    "All Do nodes must reference registered action contracts",
                ],
            );
        }
        ValidationError::ActionContractOrphan { action_id } => {
            outln!("Action Contract Orphan");
            outln!("  Action contract {action_id} has no corresponding Do node.");
            explain_repair_hint(
                "validation",
                &[
                    "Remove the orphan action contract",
                    "Or add a Do node that uses this action_id",
                ],
            );
        }
        ValidationError::SlotTypeInconsistency { slot } => {
            outln!("Slot Type Inconsistency");
            outln!("  Slot {slot} has writers with incompatible type kinds.");
            explain_repair_hint(
                "validation",
                &[
                    "Ensure all writers to this slot produce the same type",
                    "Fix type mismatches between step outputs",
                ],
            );
        }
        ValidationError::NonDeterministicPath { from_node, to_node } => {
            outln!("Non-Deterministic Path");
            outln!("  Path from node {from_node} to {to_node} contains no suspension point.");
            explain_repair_hint(
                "validation",
                &[
                    "Add a suspension point (ask, wait, or retry) to the path",
                    "Non-deterministic paths without suspension points cause replay issues",
                ],
            );
        }
        ValidationError::AccessorPathTooDeep {
            accessor_index,
            depth,
            max,
        } => {
            outln!("Accessor Path Too Deep");
            outln!(
                "  Accessor {accessor_index} has depth {depth}, which exceeds the maximum {max}."
            );
            explain_repair_hint(
                "validation",
                &[
                    "Simplify the accessor path",
                    "Reduce nesting depth in the path",
                ],
            );
        }
        ValidationError::AccessorSymbolOutOfBounds {
            accessor_index,
            segment_index,
            symbol,
            symbols_count,
        } => {
            outln!("Accessor Symbol Out of Bounds");
            outln!(
                "  Accessor {accessor_index} segment {segment_index}: symbol {symbol} is out of bounds (symbols_count={symbols_count})."
            );
            explain_repair_hint(
                "validation",
                &[
                    "Fix the symbol index to be within symbols_count",
                    "Symbol indices are zero-based",
                ],
            );
        }
        ValidationError::CapabilityNameEmpty {
            action_id,
            capability_index,
        } => {
            outln!("Capability Name Empty");
            outln!("  Action {action_id}: capability {capability_index} has an empty name.");
            explain_repair_hint(
                "validation",
                &[
                    "Provide a non-empty name for the capability",
                    "Capability names must be non-empty strings",
                ],
            );
        }
        ValidationError::CapabilityNameTooLong {
            action_id,
            capability_index,
            len,
            max,
        } => {
            outln!("Capability Name Too Long");
            outln!(
                "  Action {action_id}: capability {capability_index} name length {len} exceeds max {max}."
            );
            explain_repair_hint(
                "validation",
                &[
                    "Shorten the capability name",
                    "Capability names have a maximum length",
                ],
            );
        }
        ValidationError::CapabilityNameInvalid {
            action_id,
            capability_index,
            name,
        } => {
            outln!("Capability Name Invalid");
            outln!("  Action {action_id}: capability {capability_index} name '{name}' is invalid.");
            explain_repair_hint(
                "validation",
                &[
                    "Use valid capability name characters",
                    "Check the Velvet v1 schema for naming rules",
                ],
            );
        }
        ValidationError::CapabilityActionMismatch {
            contract_action_id,
            capability_action_id,
            capability_index,
        } => {
            outln!("Capability Action Mismatch");
            outln!(
                "  Contract action {contract_action_id} != capability action {capability_action_id} at index {capability_index}."
            );
            explain_repair_hint(
                "validation",
                &[
                    "Ensure capability action_ids match the contract",
                    "Fix the capability action_id at index {capability_index}",
                ],
            );
        }
        ValidationError::CapabilityDuplicate {
            action_id,
            first_index,
            duplicate_index,
            name,
        } => {
            outln!("Capability Duplicate");
            outln!(
                "  Action {action_id}: capability '{name}' first at {first_index}, duplicate at {duplicate_index}."
            );
            explain_repair_hint(
                "validation",
                &[
                    "Remove duplicate capability names",
                    "Each capability name must be unique within an action",
                ],
            );
        }
        ValidationError::MissingSchemaVersion => {
            outln!("Missing Schema Version");
            outln!("  The workflow does not declare a schema version.");
            explain_repair_hint(
                "validation",
                &[
                    "Add a schema version to the workflow",
                    "Check the Velvet v1 schema for version requirements",
                ],
            );
        }
        ValidationError::CueVetFailed { file } => {
            outln!("CUE Vet Failed");
            outln!("  The CUE schema validation failed for '{file}'.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix CUE schema violations in the file",
                    "Check the CUE schema for the expected structure",
                ],
            );
        }
        ValidationError::VersionMonotonicityBreach {
            file,
            expected,
            actual,
        } => {
            outln!("Version Monotonicity Breach");
            outln!("  File '{file}': version {actual} is not >= expected {expected}.");
            explain_repair_hint(
                "validation",
                &[
                    "Ensure version numbers are monotonically increasing",
                    "Update '{file}' to have version >= {expected}",
                ],
            );
        }
        _ => {
            outln!("Unknown Validation Error");
            outln!("  {err}");
        }
    }
}

