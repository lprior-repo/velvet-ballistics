#![forbid(unsafe_code)]
//! Workflow submission command.

use std::process::ExitCode;
use std::io::{self, Write};
use std::sync::Arc;
use std::num::NonZeroUsize;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::args::{ActionRegistryMode, Command, DurabilityMode, OutputFormat, ParseError, StepTarget};
use crate::exit_code::CliExitCode;
use crate::output::{json_error, json_out, output_error_exit, write_stdout_line, write_stderr_line, write_failure_message, write_contract_error_json};
use crate::output_utils::*;
use crate::file_io::{read_file, parse_run_id, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};

pub(crate) fn cmd_submit(
    workflow: &std::path::Path,
    input_bin: &std::path::Path,
    db: &std::path::Path,
    durability: DurabilityMode,
    output: OutputFormat,
) -> ExitCode {
    let _input_data = match read_file(input_bin, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let bytes = match read_file(workflow, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(c) => c,
        Err(errors) => {
            let error_msgs: Vec<String> = errors.0.iter().map(|err| err.to_string()).collect();
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": "compilation failed",
                        "errors": error_msgs
                    }),
                    output,
                );
            } else {
                for err in &errors.0 {
                    crate::errln!("compile error: {err}");
                }
            }
            return CliExitCode::CompileFailed.into();
        }
    };

    let digest = compiled.digest();
    let digest_hex: String = digest
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let step_count = compiled.node_count();

    // Generate run_id from timestamp
    let run_id_num = generate_submit_run_id();
    let run_id = vb_core::RunId::new(run_id_num);

    // Open storage journal and record workflow source + run header
    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            write_failure_message(
                &format!("error opening journal at {}: {e}", db.display()),
                output,
                CliExitCode::StorageError,
            );
            return CliExitCode::StorageError.into();
        }
    };

    // Store the workflow source
    let source_digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(&bytes).into());
    let source_record = vb_storage::WorkflowSourceRecord {
        digest: source_digest,
        source: bytes,
    };
    if let Err(e) = vb_storage::put_workflow_source(&journal, &source_record) {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("workflow source write error: {e}")
                }),
                output,
            );
        } else {
            crate::errln!("workflow source write error: {e}");
        }
        return CliExitCode::StorageError.into();
    }

    // Record the run header
    let accepted_at_ms: u64 = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => match u64::try_from(d.as_millis()) {
            Ok(ms) => ms,
            Err(_) => {
                crate::errln!("warning: system clock value does not fit in u64; using 0");
                0_u64
            }
        },
        Err(_) => 0_u64,
    };
    let header = vb_storage::RunHeaderRecord {
        run: run_id,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: digest,
        status: 0,
        accepted_at_ms,
    };
    if let Err(e) = vb_storage::put_run_header(&journal, &header) {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("run header write error: {e}")
                }),
                output,
            );
        } else {
            crate::errln!("run header write error: {e}");
        }
        return CliExitCode::StorageError.into();
    }
    // Also record submission for durability-aware runbooks before releasing the metadata journal.
    if durability != DurabilityMode::None {
        let event = vb_storage::JournalEvent::RunAccepted {
            run: run_id,
            seq: vb_storage::EventSeq::new(0),
            workflow: digest,
        };
        if let Err(e) = journal.append_strict_batch(&[event]) {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("journal append error: {e}")
                    }),
                    output,
                );
            } else {
                crate::errln!("journal append error: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    }
    drop(journal);

    if output != OutputFormat::Text {
        crate::emit_json_or_return!(
            &serde_json::json!({
                "run_id": run_id.get(),
                "digest": digest_hex,
                "status": "submitted",
                "step_count": step_count
            }),
            output,
        );
    } else {
        crate::outln!("submitted run {}", run_id.get());
        crate::outln!("  digest:     {digest_hex}");
        crate::outln!("  steps:      {step_count}");
        crate::outln!("  durability: {}", durability_as_str(durability));
        crate::outln!("  status:     submitted");
    }

    CliExitCode::Success.into()
}


pub(crate) fn durability_as_str(mode: DurabilityMode) -> &'static str {
    match mode {
        DurabilityMode::Strict => "strict",
        DurabilityMode::Journaled => "journaled",
        DurabilityMode::None => "none",
    }
}


pub(crate) fn generate_submit_run_id() -> u64 {
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return u64::MAX;
    };
    match u64::try_from(now.as_nanos()) {
        Ok(value) => value,
        Err(_) => now.as_secs(),
    }
}

