//! CLI dispatcher and shared implementation prelude.
//!
//! This module re-exports the items used by `main.rs` and by the split
//! submodules under `use crate::app_impl::prelude::*;` so that the
//! `prelude::*` glob import keeps resolving.

#![forbid(unsafe_code)]

use std::process::ExitCode;

pub(crate) mod prelude {
    pub(crate) use std::num::NonZeroUsize;
    pub(crate) use std::process::ExitCode;
    pub(crate) use std::sync::Arc;
    pub(crate) use std::time::{Instant, SystemTime, UNIX_EPOCH};

    pub(crate) use crate::action::{
        write_action_inspect, write_action_registry, write_action_registry_uninitialized,
    };
    pub(crate) use crate::action_specs::{
        action_contract_detail, action_table_rows, registered_cli_actions,
    };
    pub(crate) use crate::app_impl::{
        compile_errors_message, HELP, INPUT_MAPPING_DECODE_FAILED_MESSAGE,
        INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE, INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE,
        VERSION,
    };
    pub(crate) use crate::args::{
        self, ActionRegistryMode, Command, DurabilityMode, EmitTarget, EventStatus, OutputFormat,
        StepTarget,
    };
    pub(crate) use crate::constants::VERSION as VERSION_CONST;
    pub(crate) use crate::exit_code::CliExitCode;
    pub(crate) use crate::file_io::{
        parse_run_id, read_file, read_journal_events, report_storage_open_error,
    };
    pub(crate) use crate::io_helpers::{
        exit_from_io, unique_doctor_run_id, write_help_stdout, write_parse_error_stderr,
        write_version_stdout,
    };
    pub(crate) use crate::output::{
        json_error, json_out_exit, output_error_exit, write_contract_error_json,
        write_failure_message, write_json_pretty_stdout,
    };
    pub(crate) use crate::run::report_compiled_ir_store_error;
    pub(crate) use crate::run_cancel::run_is_terminal;
    pub(crate) use crate::run_compiled::{
        map_runtime_inputs, runtime_config_for_durability, runtime_journal_for_mode,
    };
    pub(crate) use crate::run_compiled_runtime::{
        open_storage_runtime_journal, run_compiled_workflow,
    };
    pub(crate) use crate::step_helpers::{
        compile_bytes_json, decode_step_inputs, execute_step_isolated,
    };
    pub(crate) use crate::step_helpers_display::{error_name, print_step_result};
    pub(crate) use crate::{
        agent_context, cli_envelope, commands_diff, commands_incident, commands_journal,
        commands_status, commands_system_status, commands_workflow, deliver_sink,
    };
    pub(crate) use vb_ipc::client::IpcClient;
    pub(crate) use vb_ipc::{IpcCommand, IpcPayload};
    pub(crate) use vb_runtime::action::ActionRegistry;
}

pub(crate) const HELP: &str = "velvet-ballistics - compiled workflow runtime";

pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) const INPUT_MAPPING_DECODE_FAILED_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input-bin decode failed";
pub(crate) const INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input slot count exceeds workflow slot count";
pub(crate) const INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input slot index out of range";

pub(crate) fn compile_errors_message(errors: &[vb_compile::CompileError]) -> String {
    crate::validate::compile_errors_message(errors)
}

pub(crate) fn run_from_env() -> ExitCode {
    crate::dispatcher::run_from_env()
}
