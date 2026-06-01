#![forbid(unsafe_code)]
//! Action specification type definitions and query functions.

use std::process::ExitCode;
use crate::args::{ActionRegistryMode, Command, DurabilityMode, EmitTarget, EventStatus, OutputFormat, ParseError, StepTarget, VALID_COMMANDS, VerifyProfile};
use crate::exit_code::CliExitCode;
use crate::output::{OutputError, json_error, json_out, output_error_exit, write_contract_error_json, write_failure_message, write_stderr_line, write_stderr_best_effort, write_stdout_line, write_stdout_line_checked, write_structured_stderr};
use crate::output_utils::*;
use crate::file_io::{read_file, parse_run_id, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_error_stderr, write_help_stdout, write_version_stdout};

pub(crate) fn registered_cli_actions() -> vb_core::action::ActionResult<crate::ActionRegistry> {
    crate::base_actions::cli_action_list()
}

pub(crate) fn cli_action_specs() -> &'static [CliActionSpec] {
    &[]
}

pub(crate) fn action_contract(
    action_name: &vb_core::action::ActionName,
    registry: &crate::ActionRegistry,
) -> vb_core::action::ActionResult<crate::ActionContract> {
    registry.resolve_by_name(action_name).map(|_| {
        crate::ActionContract {
            id: vb_core::action::ActionId::new(0),
            name: action_name.clone(),
            idempotency: vb_core::action::Idempotency::NotIdempotent,
            retry_safety: vb_core::action::RetrySafety::NotRetrySafe,
            side_effect: vb_core::action::SideEffect::HasSideEffect,
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            required_capabilities: Vec::new(),
            example_input_schema: "",
            example_output_schema: "",
        }
    })
}

pub(crate) fn action_idempotency_name(value: vb_core::action::Idempotency) -> &'static str {
    use vb_core::action::Idempotency;
    match value {
        Idempotency::Idempotent => "idempotent",
        Idempotency::NotIdempotent => "not-idempotent",
    }
}

pub(crate) fn action_retry_safety_name(value: vb_core::action::RetrySafety) -> &'static str {
    use vb_core::action::RetrySafety;
    match value {
        RetrySafety::RetrySafe => "retry-safe",
        RetrySafety::NotRetrySafe => "not-retry-safe",
    }
}

pub(crate) fn action_side_effect_name(value: vb_core::action::SideEffect) -> &'static str {
    use vb_core::action::SideEffect;
    match value {
        SideEffect::NoSideEffect => "no-side-effect",
        SideEffect::HasSideEffect => "has-side-effect",
    }
}

pub(crate) fn action_failure_code_names() -> &'static [&'static str] {
    &[]
}

pub(crate) fn action_idempotency_rule(value: vb_core::action::Idempotency) -> &'static str {
    use vb_core::action::Idempotency;
    match value {
        Idempotency::Idempotent => "can-retry-on-failure",
        Idempotency::NotIdempotent => "must-not-retry-on-failure",
    }
}
