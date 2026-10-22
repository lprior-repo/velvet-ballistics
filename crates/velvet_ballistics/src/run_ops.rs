//! Module: run_ops

use crate::app_impl::prelude::*;

pub(crate) fn cmd_retry(
    run_id: &str,
    requested_step: u16,
    db: &std::path::Path,
    output: OutputFormat,
) -> ExitCode {
    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };

    // Open the journal once; it is used both to read evidence and to execute the
    // recovered retry attempt through the storage-backed runtime.
    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            report_storage_open_error(&e, db, output);
            return CliExitCode::StorageError.into();
        }
    };

    let events = match journal.events_for_run(rid) {
        Ok(ev) => ev,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error reading run {run_id}: {e}")
                    }),
                    output,
                );
            } else {
                errln!("error reading run {run_id}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    };

    if events.is_empty() {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({ "success": false, "error": format!("run {run_id} not found") }),
                output,
            );
        } else {
            errln!("run {run_id}: no events found");
        }
        return CliExitCode::StorageError.into();
    }

    let analysis = commands_journal::analyze_retry(&events);
    if !analysis.can_retry {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({ "success": false, "error": format!("run {run_id} {}", analysis.reason) }),
                output,
            );
        } else {
            errln!("run {run_id} {}", analysis.reason);
        }
        return CliExitCode::LifecycleError.into();
    }

    // The requested step must be the step actually observed as failed in the journal.
    // It is never inferred from a previous successful step plus one, and a `u16::MAX`
    // requested step is rejected by exact comparison rather than silently clamped.
    let observed_failed_step = match analysis.failed_at_step {
        Some(step) => step,
        None => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("run {run_id} failed but no observed failed step"),
                        "observed_failed_step": Option::<u16>::None,
                        "requested_step": requested_step
                    }),
                    output,
                );
            } else {
                errln!("run {run_id} failed but no observed failed step");
            }
            return CliExitCode::LifecycleError.into();
        }
    };

    if observed_failed_step != requested_step {
        let message = format!(
            "run {run_id} failed at observed step {observed_failed_step}; requested step {requested_step} is stale"
        );
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": message,
                    "observed_failed_step": observed_failed_step,
                    "last_successful_step": analysis.last_successful_step,
                    "requested_step": requested_step
                }),
                output,
            );
        } else {
            errln!("{message}");
        }
        return CliExitCode::LifecycleError.into();
    }

    // Load the latest durable retry checkpoint produced at the failure boundary.
    let checkpoint = match load_latest_retry_checkpoint(&events, rid, requested_step) {
        Ok(checkpoint) => checkpoint,
        Err(message) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": message,
                        "observed_failed_step": observed_failed_step,
                        "requested_step": requested_step
                    }),
                    output,
                );
            } else {
                errln!("{message}");
            }
            return CliExitCode::LifecycleError.into();
        }
    };

    // The next durable sequence is the last evidence sequence plus one (checked).
    let next_seq = match events.last().map(vb_storage::JournalEvent::seq) {
        Some(seq) => match seq.get().checked_add(1).map(vb_storage::EventSeq::new) {
            Some(next) => next,
            None => {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": format!("run {run_id} journal sequence overflow")
                        }),
                        output,
                    );
                } else {
                    errln!("run {run_id}: journal sequence overflow");
                }
                return CliExitCode::StorageError.into();
            }
        },
        None => vb_storage::EventSeq::ZERO,
    };

    let Some(shard_count) = NonZeroUsize::new(1) else {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": "runtime configuration error: shard count must be non-zero"
                }),
                output,
            );
        } else {
            errln!("runtime configuration error: shard count must be non-zero");
        }
        return CliExitCode::RuntimeFailed.into();
    };

    let config = vb_runtime::shard::ShardConfig::default();
    let shared = vb_runtime::journal::StorageRuntimeJournal::shared_journaled(Arc::new(journal));
    let mut runtime = vb_runtime::runtime::Runtime::new(shard_count, config, shared);

    if let Err(e) = runtime.recover_retry(rid, checkpoint, next_seq) {
        report_runtime_error(format_args!("retry recovery failed: {e}"), output);
        return CliExitCode::RuntimeFailed.into();
    }
    if let Err(e) = runtime.tick_all() {
        report_runtime_error(format_args!("runtime tick error: {e}"), output);
        return CliExitCode::RuntimeFailed.into();
    }

    let counters = runtime.counters_snapshot();
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": counters.runs_failed == 0,
                "run_id": rid.get(),
                "requested_step": requested_step,
                "status": if counters.runs_failed != 0 { "failed" } else if counters.runs_completed != 0 { "completed" } else { "accepted" },
                "runtime": {
                    "completed": counters.runs_completed,
                    "failed": counters.runs_failed
                }
            }),
            output,
        );
    } else {
        outln!(
            "run {}: retried step {} completed={} failed={}",
            rid.get(),
            requested_step,
            counters.runs_completed,
            counters.runs_failed
        );
        if counters.runs_failed != 0 {
            errln!("run failed after retry");
        } else if counters.runs_completed != 0 {
            outln!("run completed");
        } else {
            outln!("run accepted but not terminal after one runtime tick");
        }
    }

    if counters.runs_failed != 0 {
        return CliExitCode::RuntimeFailed.into();
    }

    ExitCode::SUCCESS
}

/// Returns the latest durable retry checkpoint for `run` whose `retry_step` equals
/// `requested_step`. Rejects a missing, undecodable, wrong-run, or wrong-step checkpoint.
fn load_latest_retry_checkpoint(
    events: &[vb_storage::JournalEvent],
    run: vb_core::RunId,
    requested_step: u16,
) -> Result<vb_runtime::run_checkpoint::RunCheckpointV1, String> {
    let latest = events
        .iter()
        .rev()
        .find(|event| matches!(event, vb_storage::JournalEvent::RetryCheckpoint { .. }));

    let vb_storage::JournalEvent::RetryCheckpoint {
        run: checkpoint_run,
        step,
        payload_digest,
        payload,
        ..
    } = latest.ok_or_else(|| "run has no durable retry checkpoint".to_string())?
    else {
        return Err("run has no durable retry checkpoint".to_string());
    };

    if *checkpoint_run != run {
        return Err(format!(
            "retry checkpoint belongs to run {}, expected {}",
            checkpoint_run.get(),
            run.get()
        ));
    }
    if step.get() != requested_step {
        return Err(format!(
            "retry checkpoint step {} does not match requested step {requested_step}",
            step.get()
        ));
    }

    // Reject a tampered payload before decoding: the durable digest must match the
    // recorded payload bytes (fail-closed integrity check).
    let observed_digest = *blake3::hash(payload).as_bytes();
    if observed_digest != *payload_digest {
        return Err("retry checkpoint payload digest mismatch".to_string());
    }

    let checkpoint: vb_runtime::run_checkpoint::RunCheckpointV1 = postcard::from_bytes(payload)
        .map_err(|e| format!("retry checkpoint decode failed: {e}"))?;

    if checkpoint.run_id != run {
        return Err(format!(
            "retry checkpoint payload belongs to run {}, expected {}",
            checkpoint.run_id.get(),
            run.get()
        ));
    }
    if checkpoint.retry_step.get() != requested_step {
        return Err(format!(
            "retry checkpoint payload step {} does not match requested step {requested_step}",
            checkpoint.retry_step.get()
        ));
    }
    Ok(checkpoint)
}
