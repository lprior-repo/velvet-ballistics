//! Repair hints and verification failure formatting.
    outln!("");
    outln!("Repair hints ({context}):");
    for hint in hints {
        outln!("  - {hint}");
    }
}

/// Explain why a verification gate passed.
pub(crate) fn explain_gate_pass(gate: &str) {
    outln!("  ✓ {gate}");
}

/// Explain a verification failure with repair hints.
pub(crate) fn explain_verification_failure(err: &commands_verify::VerifyError) {
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

pub(crate) fn explain_validation_error(err: &vb_validate::ValidationError) {
