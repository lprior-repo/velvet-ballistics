use super::*;

pub(super) fn answer_response_with_identity(
    command: IpcCommand,
    correlation: u64,
    response: IpcResponse,
    run_id: RunId,
    output: OutputFormat,
) -> ExitCode {
    if matches!(&response, IpcResponse::AcceptedRun { .. })
        && (command != ANSWER_COMMAND || correlation != ANSWER_CORRELATION)
    {
        return answer_error(
            &format!("unexpected response: {response:?}"),
            output,
            CliExitCode::RuntimeFailed,
        );
    }
    answer_response(response, run_id, output)
}

pub(super) fn answer_response(
    response: IpcResponse,
    run_id: RunId,
    output: OutputFormat,
) -> ExitCode {
    match response {
        IpcResponse::AcceptedRun {
            run_id: accepted_run,
        } if accepted_run == run_id.get() => answer_accepted(run_id, output),
        IpcResponse::RuntimeError { message } => answer_prefixed_error(
            "runtime error",
            &message,
            output,
            CliExitCode::RuntimeFailed,
        ),
        IpcResponse::PayloadError { message, .. } => answer_prefixed_error(
            "payload error",
            &message,
            output,
            CliExitCode::ValidationFailed,
        ),
        other => answer_error(
            &format!("unexpected response: {other:?}"),
            output,
            CliExitCode::RuntimeFailed,
        ),
    }
}

fn answer_accepted(run_id: RunId, output: OutputFormat) -> ExitCode {
    if output != OutputFormat::Text {
        emit_json_or_return!(
            &serde_json::json!({ "success": true, "run_id": run_id.get() }),
            output
        );
    } else {
        outln!("answer accepted for run {}", run_id.get());
    }
    ExitCode::SUCCESS
}

pub(super) fn answer_error(message: &str, output: OutputFormat, code: CliExitCode) -> ExitCode {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({ "success": false, "error": message }),
            output,
        );
    } else {
        errln!("{message}");
    }
    code.into()
}

fn answer_prefixed_error(
    prefix: &str,
    message: &str,
    output: OutputFormat,
    code: CliExitCode,
) -> ExitCode {
    if output == OutputFormat::Text {
        errln!("{prefix}: {message}");
        return code.into();
    }
    answer_error(message, output, code)
}
