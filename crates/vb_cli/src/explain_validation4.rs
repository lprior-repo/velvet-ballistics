//! Validation error formatting (part 4).
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

pub(crate) fn cmd_graph(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
