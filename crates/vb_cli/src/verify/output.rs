//! Machine-readable output helpers for verify.
//!
//! Wraps the output module's JSON emission functions with the verify-specific
//! signature (taking `OutputFormat` and `LegacyJsonOutput`).

#![forbid(unsafe_code)]

use crate::args::{LegacyJsonOutput, OutputFormat};

/// Write a machine-readable JSON value to stdout.
pub(crate) fn emit_verify_machine_stdout(
    value: &serde_json::Value,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> Result<(), crate::output::OutputError> {
    if legacy_json.is_enabled() {
        crate::output::write_legacy_json_stdout(value, legacy_json)
    } else {
        crate::output::json_out(value, output)
    }
}
