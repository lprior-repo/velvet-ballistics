//! Validation error formatting (part 1).
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
