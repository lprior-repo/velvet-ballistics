//! File I/O for the verify command.
//!
//! Reads workflow YAML files with error handling appropriate for the verify
//! pipeline.

#![forbid(unsafe_code)]

use std::path::Path;
use std::process::ExitCode;

use crate::args::{LegacyJsonOutput, OutputFormat};
use crate::exit_code::CliExitCode;

use super::error::emit_verify_diagnostic;

/// Read a workflow file for verification.
///
/// When legacy JSON is enabled, reads directly from disk without invoking
/// the full `file_io` pipeline to avoid double-encoding in the output layer.
/// Otherwise delegates to [`crate::file_io::read_file`].
pub(crate) fn read_verify_file(
    workflow: &Path,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> Result<Vec<u8>, ExitCode> {
    if !legacy_json.is_enabled() {
        return crate::file_io::read_file(workflow, output, CliExitCode::ValidationFailed);
    }
    match std::fs::read(workflow) {
        Ok(bytes) => Ok(bytes),
        Err(error) => Err(emit_verify_diagnostic(
            &format!("error reading {}: {error}", workflow.display()),
            CliExitCode::ValidationFailed,
            output,
            legacy_json,
        )),
    }
}
