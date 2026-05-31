//! Workflow verification command and helpers.
pub(crate) fn cmd_verify(
    workflow: &std::path::Path,
    profile: VerifyProfile,
    output: OutputFormat,
) -> ExitCode {
    let bytes = match read_file(workflow, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let text = match std::str::from_utf8(&bytes) {
        Ok(t) => t,
        Err(e) => {
            write_failure_message(
                &format!("file is not valid UTF-8: {e}"),
                output,
                CliExitCode::ValidationFailed,
            );
            return CliExitCode::ValidationFailed.into();
        }
    };

    match commands_verify::run_verification(text, &bytes, profile) {
        Ok(result) => {
            if output != OutputFormat::Text {
                emit_json_or_return!(&verify_success_report(&result, profile), output);
            } else {
                outln!("verification certificate");
                outln!("  digest:  {}", result.digest_hex);
                outln!("  profile: {}", profile.as_str());
                outln!("  nodes:   {}", result.node_count);
                outln!("  checks:  {}", result.checks.len());
                for check in &result.checks {
                    outln!("    - {check}");
                }
                if !result.warnings.is_empty() {
                    outln!("  warnings: {}", result.warnings.len());
                    for warning in &result.warnings {
                        outln!("    - {warning}");
                    }
                }
                outln!("verified");
            }
            ExitCode::SUCCESS
        }
