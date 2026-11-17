    use super::*;

    #[test]
    fn matching_answer_run_identity_succeeds() {
        for id in [0, 1, u64::MAX] {
            assert_eq!(
                answer_response(
                    IpcResponse::AcceptedRun { run_id: id },
                    RunId::new(id),
                    OutputFormat::Text,
                ),
                ExitCode::SUCCESS,
            );
        }
    }

    #[test]
    fn mismatched_answer_run_identity_is_runtime_failure() {
        for (requested, accepted) in [(1, 2), (0, u64::MAX), (u64::MAX, 0)] {
            assert_eq!(
                answer_response(
                    IpcResponse::AcceptedRun { run_id: accepted },
                    RunId::new(requested),
                    OutputFormat::Text,
                ),
                ExitCode::from(CliExitCode::RuntimeFailed),
            );
        }
    }

    #[test]
    fn runtime_error_response_preserves_exit_status() {
        assert_eq!(
            answer_response(
                IpcResponse::RuntimeError {
                    message: String::from("runtime rejected answer"),
                },
                RunId::new(1),
                OutputFormat::Text,
            ),
            ExitCode::from(CliExitCode::RuntimeFailed),
        );
    }

    #[test]
    fn payload_error_response_preserves_exit_status() {
        assert_eq!(
            answer_response(
                IpcResponse::PayloadError {
                    diagnostic: 1,
                    message: String::from("invalid answer"),
                },
                RunId::new(1),
                OutputFormat::Text,
            ),
            ExitCode::from(CliExitCode::ValidationFailed),
        );
    }

    #[test]
    fn unrelated_response_preserves_runtime_failure() {
        assert_eq!(
            answer_response(IpcResponse::Healthy, RunId::new(1), OutputFormat::Text),
            ExitCode::from(CliExitCode::RuntimeFailed),
        );
    }

    #[test]
    fn accepted_answer_requires_matching_command_correlation_and_run() {
        for (command, correlation, accepted, expected) in [
            (IpcCommand::AnswerAsk, 0, 17, ExitCode::SUCCESS),
            (IpcCommand::Health, 0, 17, ExitCode::from(CliExitCode::RuntimeFailed)),
            (IpcCommand::AnswerAsk, 1, 17, ExitCode::from(CliExitCode::RuntimeFailed)),
            (IpcCommand::AnswerAsk, u64::MAX, 17, ExitCode::from(CliExitCode::RuntimeFailed)),
            (IpcCommand::AnswerAsk, 0, 18, ExitCode::from(CliExitCode::RuntimeFailed)),
        ] {
            assert_eq!(
                answer_response_with_identity(
                    command, correlation, IpcResponse::AcceptedRun { run_id: accepted },
                    RunId::new(17), OutputFormat::Text,
                ),
                expected,
            );
        }
    }

    #[test]
    fn nonaccepted_answer_responses_keep_error_mapping_with_unmatched_headers() {
        for (response, expected) in [
            (
                IpcResponse::RuntimeError { message: String::from("runtime rejection") },
                ExitCode::from(CliExitCode::RuntimeFailed),
            ),
            (
                IpcResponse::PayloadError { diagnostic: 1, message: String::from("bad payload") },
                ExitCode::from(CliExitCode::ValidationFailed),
            ),
            (IpcResponse::Healthy, ExitCode::from(CliExitCode::RuntimeFailed)),
        ] {
            assert_eq!(
                answer_response_with_identity(
                    IpcCommand::Health, u64::MAX, response, RunId::new(17), OutputFormat::Text,
                ),
                expected,
            );
        }
    }
