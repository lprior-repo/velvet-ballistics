#![forbid(unsafe_code)]

use super::failure::{replay_failure_outcome, replay_no_recovery_outcome};
use super::write_locked_read_surface;
use crate::args::OutputFormat;
use crate::exit_code::CliExitCode;

#[test]
fn replay_no_recovery_outcome_is_validation_failure() {
    let outcome = replay_no_recovery_outcome("42");

    assert_eq!(outcome.code, CliExitCode::ValidationFailed);
    assert_eq!(outcome.message, "run 42: no events found");
}

#[test]
fn replay_divergence_outcome_preserves_typed_code() {
    let error = vb_storage::recovery::RecoveryError::ReplayDivergence {
        step: vb_core::StepIdx::ZERO,
        detail: String::from("storage validation compile text"),
    };
    let outcome = replay_failure_outcome("7", &error, None);

    assert_eq!(outcome.code, CliExitCode::ReplayDivergence);
    assert!(outcome.message.contains("error replaying run 7"));
}

#[test]
fn write_locked_read_surface_text_returns_success_not_failure() {
    let code = write_locked_read_surface("events", "42", OutputFormat::Text);
    assert_eq!(code, std::process::ExitCode::SUCCESS);
}

#[test]
fn write_locked_read_surface_yaml_returns_success() {
    let code = write_locked_read_surface("inspect", "7", OutputFormat::Yaml);
    assert_eq!(code, std::process::ExitCode::SUCCESS);
}

#[test]
fn write_locked_read_surface_distinct_commands_share_signature() {
    let events = write_locked_read_surface("events", "1", OutputFormat::Text);
    let inspect = write_locked_read_surface("inspect", "1", OutputFormat::Text);
    let replay = write_locked_read_surface("replay", "1", OutputFormat::Text);
    assert_eq!(events, inspect);
    assert_eq!(events, replay);
}
