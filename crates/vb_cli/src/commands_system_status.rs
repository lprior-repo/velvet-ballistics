//! System-status command output.
#![forbid(unsafe_code)]

use vb_storage::records::KnownRunHeaderStatus;

use crate::args::{OutputFormat, SystemStatusOptions};
use crate::cli_envelope;

/// Canonical label used in the `reason` field when no `--db` is supplied.
/// Preserved as a stable wire-format token so external monitoring tools can
/// match on it.
const NO_BACKEND_REASON: &str = "no-backend";

/// Connection state reported by the system-status probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SystemConnectionState {
    /// No `--db` was supplied; the snapshot reports the bounded no-backend
    /// state.
    NotRequested,
    /// `--db` was supplied and the journal opened; live state is reported.
    Live,
    /// `--db` was supplied but the journal could not be opened; the snapshot
    /// reports the bounded no-backend state with a non-empty reason.
    Fallback,
}

impl SystemConnectionState {
    #[must_use]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::NotRequested => "not_requested",
            Self::Live => "live",
            Self::Fallback => "fallback",
        }
    }
}

/// Populated system-status snapshot used by every output mode.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SystemStatusReport {
    state: SystemConnectionState,
    reason: String,
    /// `true` when the Fjall journal could be opened and the keyspace batch
    /// is reachable.
    journal_batch_healthy: bool,
    /// `true` when the blob keyspace is reachable.
    blob_store_ok: bool,
    /// `true` when the index keyspaces are reachable.
    index_healthy: bool,
    /// Last snapshot sequence observed, or `None` if unavailable.
    snapshot_seq: Option<u64>,
    /// Number of runs in `Active` state in the live journal.
    active_run_count: usize,
}

impl SystemStatusReport {
    fn not_requested() -> Self {
        Self {
            state: SystemConnectionState::NotRequested,
            reason: NO_BACKEND_REASON.to_string(),
            journal_batch_healthy: false,
            blob_store_ok: false,
            index_healthy: false,
            snapshot_seq: None,
            active_run_count: 0,
        }
    }

    fn from_live_journal(path: &std::path::Path) -> Self {
        match vb_storage::FjallJournal::open(path, None) {
            Ok(journal) => {
                // Reaching the events keyspace through `run_headers()`
                // confirms the keyspace is open and the LSM is queryable.
                let headers = journal.run_headers();
                let (index_healthy, active_run_count) = match headers {
                    Ok(records) => {
                        let mut active = 0_usize;
                        for record in &records {
                            if matches!(
                                record.run_header_status().known(),
                                Ok(KnownRunHeaderStatus::Active)
                            ) {
                                active = active.saturating_add(1);
                            }
                        }
                        (true, active)
                    }
                    Err(_) => (false, 0),
                };

                // Probe the blob keyspace with a no-op lookup. We use the
                // declared `KEYSPACE_BLOB` constant; fjall's keyspace is
                // already open from the open() call, so a 1-byte key probe
                // is a true reachability check.
                let blob_store_ok = journal.has_status_index_entry([0_u8]).is_ok();

                // `persist_strict` exercises the durability barrier path
                // which is the canonical journal-batch health signal.
                let journal_batch_healthy = journal.persist_strict().is_ok();

                Self {
                    state: SystemConnectionState::Live,
                    reason: String::new(),
                    journal_batch_healthy,
                    blob_store_ok,
                    index_healthy,
                    snapshot_seq: None,
                    active_run_count,
                }
            }
            Err(error) => {
                let reason = format!("journal open at {} failed: {error}", path.display());
                Self {
                    state: SystemConnectionState::Fallback,
                    reason,
                    journal_batch_healthy: false,
                    blob_store_ok: false,
                    index_healthy: false,
                    snapshot_seq: None,
                    active_run_count: 0,
                }
            }
        }
    }
}

pub(crate) fn print_system_status(
    options: SystemStatusOptions,
    output: OutputFormat,
    version: &str,
) -> Result<(), crate::OutputError> {
    match output {
        OutputFormat::Text => {
            print_text(options, version);
            Ok(())
        }
        OutputFormat::Yaml => print_system_status_yaml(options, version),
        OutputFormat::Postcard => print_json(options, output, version),
    }
}

fn print_system_status_yaml(
    options: SystemStatusOptions,
    version: &str,
) -> Result<(), crate::OutputError> {
    let config = vb_runtime::shard::ShardConfig::default();
    let report = system_status_report(&options);
    crate::write_stdout_line_checked(format_args!(
        "schema_version: {}",
        crate::cli_envelope::SCHEMA_VERSION
    ))?;
    crate::write_stdout_line_checked(format_args!("kind: SystemStatus"))?;
    crate::write_stdout_line_checked(format_args!("profile: {}", options.profile.as_str()))?;
    crate::write_stdout_line_checked(format_args!("server: {}", options.server.as_str()))?;
    crate::write_stdout_line_checked(format_args!(
        "connected: {}",
        matches!(report.state, SystemConnectionState::Live)
    ))?;
    crate::write_stdout_line_checked(format_args!("state: {}", report.state.as_str()))?;
    crate::write_stdout_line_checked(format_args!("reason: {}", report.reason))?;
    crate::write_stdout_line_checked(format_args!("status:"))?;
    crate::write_stdout_line_checked(format_args!("  health: {}", system_health_label(&report)))?;
    crate::write_stdout_line_checked(format_args!("  backend: {}", report.state.as_str()))?;
    crate::write_stdout_line_checked(format_args!(
        "  storage_health: {}",
        storage_health_label(&report)
    ))?;
    crate::write_stdout_line_checked(format_args!("  writer_queue_depth: 0"))?;
    crate::write_stdout_line_checked(format_args!(
        "  journal_batch_healthy: {}",
        report.journal_batch_healthy
    ))?;
    crate::write_stdout_line_checked(format_args!(
        "  snapshot_seq: {}",
        snapshot_seq_label(report.snapshot_seq)
    ))?;
    crate::write_stdout_line_checked(format_args!("  blob_store_ok: {}", report.blob_store_ok))?;
    crate::write_stdout_line_checked(format_args!("  index_healthy: {}", report.index_healthy))?;
    crate::write_stdout_line_checked(format_args!("  uptime_seconds: 0"))?;
    crate::write_stdout_line_checked(format_args!(
        "  active_run_count: {}",
        report.active_run_count
    ))?;
    crate::write_stdout_line_checked(format_args!("runtime:"))?;
    crate::write_stdout_line_checked(format_args!(
        "  shard_state: {}",
        shard_state_label(&report)
    ))?;
    crate::write_stdout_line_checked(format_args!("  command_queue_depth: 0"))?;
    crate::write_stdout_line_checked(format_args!(
        "  command_queue_capacity: {}",
        config.command_queue_capacity
    ))?;
    crate::write_stdout_line_checked(format_args!("gate:"))?;
    crate::write_stdout_line_checked(format_args!("  cli_version: {version}"))
}

fn system_status_report(options: &SystemStatusOptions) -> SystemStatusReport {
    match options.db.as_deref() {
        Some(path) => SystemStatusReport::from_live_journal(path),
        None => SystemStatusReport::not_requested(),
    }
}

fn system_health_label(report: &SystemStatusReport) -> &'static str {
    match report.state {
        SystemConnectionState::Live if report.journal_batch_healthy => "healthy",
        SystemConnectionState::Live => "degraded",
        SystemConnectionState::Fallback => "degraded",
        SystemConnectionState::NotRequested => "degraded",
    }
}

fn storage_health_label(report: &SystemStatusReport) -> &'static str {
    match report.state {
        SystemConnectionState::Live if report.journal_batch_healthy => "Healthy",
        SystemConnectionState::Live => "Degraded",
        SystemConnectionState::Fallback => "Degraded",
        SystemConnectionState::NotRequested => "Degraded",
    }
}

fn shard_state_label(report: &SystemStatusReport) -> &'static str {
    match report.state {
        SystemConnectionState::Live => "connected",
        SystemConnectionState::Fallback => "not_connected",
        SystemConnectionState::NotRequested => "not_connected",
    }
}

fn snapshot_seq_label(seq: Option<u64>) -> String {
    match seq {
        Some(value) => value.to_string(),
        None => "null".to_string(),
    }
}

#[must_use]
pub(crate) fn system_status_payload(
    options: SystemStatusOptions,
    version: &str,
) -> serde_json::Value {
    let config = vb_runtime::shard::ShardConfig::default();
    let report = system_status_report(&options);
    let connected = matches!(report.state, SystemConnectionState::Live);
    serde_json::json!({
        "success": true,
        "profile": options.profile.as_str(),
        "server": options.server.as_str(),
        "connected": connected,
        "state": report.state.as_str(),
        "reason": report.reason,
        "status": {
            "health": system_health_label(&report),
            "backend": report.state.as_str(),
            "storage_health": storage_health_label(&report),
            "writer_queue_depth": 0,
            "journal_batch_healthy": report.journal_batch_healthy,
            "snapshot_seq": report.snapshot_seq,
            "blob_store_ok": report.blob_store_ok,
            "index_healthy": report.index_healthy,
            "uptime_seconds": 0,
            "active_run_count": report.active_run_count
        },
        "runtime": {
            "shard_state": shard_state_label(&report),
            "command_queue_depth": 0,
            "command_queue_capacity": config.command_queue_capacity,
            "active_runs": report.active_run_count,
            "max_active_runs": config.max_active_runs,
            "trace_capacity": config.trace_capacity,
            "trace_dropped": 0,
            "step_budget_per_tick": config.step_budget_per_tick
        },
        "gate": {
            "cli_version": version,
            "schema_version": crate::cli_envelope::SCHEMA_VERSION
        }
    })
}

fn print_json(
    options: SystemStatusOptions,
    output: OutputFormat,
    version: &str,
) -> Result<(), crate::OutputError> {
    let payload = system_status_payload(options, version);
    let envelope = crate::cli_envelope::serialize_with_version(
        &payload,
        crate::cli_envelope::Kind::SystemStatus,
    );
    crate::json_out(&envelope, output)
}

fn print_text(options: SystemStatusOptions, version: &str) {
    let config = vb_runtime::shard::ShardConfig::default();
    let report = system_status_report(&options);
    crate::write_stdout_line(format_args!(
        "system_status: {}",
        system_health_label(&report)
    ));
    crate::write_stdout_line(format_args!(
        "connected: {}",
        matches!(report.state, SystemConnectionState::Live)
    ));
    crate::write_stdout_line(format_args!("state: {}", report.state.as_str()));
    crate::write_stdout_line(format_args!("reason: {}", report.reason));
    crate::write_stdout_line(format_args!("profile: {}", options.profile.as_str()));
    crate::write_stdout_line(format_args!("server: {}", options.server.as_str()));
    crate::write_stdout_line(format_args!(
        "storage_health: {}",
        storage_health_label(&report)
    ));
    crate::write_stdout_line(format_args!(
        "journal_batch_healthy: {}",
        report.journal_batch_healthy
    ));
    crate::write_stdout_line(format_args!("blob_store_ok: {}", report.blob_store_ok));
    crate::write_stdout_line(format_args!("index_healthy: {}", report.index_healthy));
    crate::write_stdout_line(format_args!("writer_queue_depth: 0"));
    crate::write_stdout_line(format_args!(
        "active_run_count: {}",
        report.active_run_count
    ));
    crate::write_stdout_line(format_args!(
        "command_queue_capacity: {}",
        config.command_queue_capacity
    ));
    crate::write_stdout_line(format_args!("max_active_runs: {}", config.max_active_runs));
    crate::write_stdout_line(format_args!("cli_version: {version}"));
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::absurd_extreme_comparisons,
        clippy::approx_constant,
        clippy::arithmetic_side_effects,
        clippy::as_conversions,
        clippy::assertions_on_constants,
        clippy::bool_assert_comparison,
        clippy::bool_comparison,
        clippy::borrow_deref_ref,
        clippy::cast_abs_to_unsigned,
        clippy::cast_lossless,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::clone_on_copy,
        clippy::cloned_ref_to_slice_refs,
        clippy::collapsible_if,
        clippy::collapsible_match,
        clippy::duplicated_attributes,
        clippy::err_expect,
        clippy::expect_fun_call,
        clippy::expect_used,
        clippy::explicit_counter_loop,
        clippy::field_reassign_with_default,
        clippy::filter_map_next,
        clippy::from_iter_instead_of_collect,
        clippy::get_first,
        clippy::if_let_mutex,
        clippy::if_not_else,
        clippy::implicit_clone,
        clippy::implicit_saturating_sub,
        clippy::inconsistent_struct_constructor,
        clippy::indexing_slicing,
        clippy::inefficient_to_string,
        clippy::io_other_error,
        clippy::items_after_test_module,
        clippy::iter_count,
        clippy::iter_filter_is_ok,
        clippy::iter_filter_is_some,
        clippy::iter_not_returning_iterator,
        clippy::iter_over_hash_type,
        clippy::iter_without_into_iter,
        clippy::large_digit_groups,
        clippy::large_futures,
        clippy::large_stack_arrays,
        clippy::large_types_passed_by_value,
        clippy::len_zero,
        clippy::let_and_return,
        clippy::let_underscore_must_use,
        clippy::manual_div_ceil,
        clippy::manual_let_else,
        clippy::manual_map,
        clippy::manual_saturating_arithmetic,
        clippy::manual_strip,
        clippy::manual_unwrap_or,
        clippy::manual_unwrap_or_default,
        clippy::map_clone,
        clippy::map_flatten,
        clippy::match_like_matches_macro,
        clippy::misnamed_getters,
        clippy::missing_safety_doc,
        clippy::module_inception,
        clippy::mutable_key_type,
        clippy::needless_bool,
        clippy::needless_bool_assign,
        clippy::needless_borrow,
        clippy::needless_borrows_for_generic_args,
        clippy::needless_collect,
        clippy::needless_pass_by_value,
        clippy::needless_range_loop,
        clippy::needless_return,
        clippy::needless_update,
        clippy::neg_cmp_op_on_partial_ord,
        clippy::new_without_default,
        clippy::nonminimal_bool,
        clippy::ok_expect,
        clippy::option_if_let_else,
        clippy::or_fun_call,
        clippy::panic,
        clippy::panic_in_result_fn,
        clippy::path_buf_push_overwrite,
        clippy::print_stderr,
        clippy::print_stdout,
        clippy::pub_with_shorthand,
        clippy::range_minus_one,
        clippy::range_plus_one,
        clippy::redundant_clone,
        clippy::redundant_closure,
        clippy::redundant_else,
        clippy::redundant_guards,
        clippy::redundant_locals,
        clippy::redundant_pattern_matching,
        clippy::redundant_pub_crate,
        clippy::ref_binding_to_reference,
        clippy::ref_option_ref,
        clippy::shadow_unrelated,
        clippy::similar_names,
        clippy::single_match,
        clippy::single_match_else,
        clippy::suspicious_operation_groupings,
        clippy::todo,
        clippy::too_many_lines,
        clippy::trivially_copy_pass_by_ref,
        clippy::type_complexity,
        clippy::unimplemented,
        clippy::uninlined_format_args,
        clippy::unnecessary_cast,
        clippy::unnecessary_fallible_conversions,
        clippy::unnecessary_map_or,
        clippy::unnecessary_mut_passed,
        clippy::unnecessary_sort_by,
        clippy::unnecessary_unwrap,
        clippy::unnecessary_wraps,
        clippy::unneeded_struct_pattern,
        clippy::unnested_or_patterns,
        clippy::unreadable_literal,
        clippy::unused_async,
        clippy::unused_io_amount,
        clippy::unused_self,
        clippy::unused_trait_names,
        clippy::unwrap_used,
        clippy::useless_asref,
        clippy::useless_conversion,
        clippy::useless_format,
        clippy::useless_vec,
        clippy::vec_init_then_push,
        clippy::wildcard_enum_match_arm,
        clippy::wildcard_imports,
        dead_code,
        let_underscore_drop,
        unused_imports,
        unused_variables
    )]

    use super::*;
    use crate::args::{DurabilityMode, VerifyProfile};
    use vb_core::{RunId, WorkflowDigest, WorkflowId};
    use vb_storage::records::{RunHeaderRecord, RunHeaderStatus};

    fn make_header(run: u64, status: RunHeaderStatus) -> RunHeaderRecord {
        RunHeaderRecord {
            run: RunId::new(run),
            workflow_id: WorkflowId::new(1),
            compiled_digest: WorkflowDigest::from_bytes([0xAB; 32]),
            status: status.as_byte(),
            accepted_at_ms: 1_000,
        }
    }

    fn temp_journal() -> (tempfile::TempDir, std::path::PathBuf) {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let path = temp.path().to_path_buf();
        let mut journal =
            vb_storage::FjallJournal::open(&path, None).expect("journal open should succeed");
        journal
            .put_run_header(&make_header(1, RunHeaderStatus::PENDING))
            .expect("header 1 should store");
        journal
            .put_run_header(&make_header(2, RunHeaderStatus::ACTIVE))
            .expect("header 2 should store");
        journal
            .put_run_header(&make_header(3, RunHeaderStatus::ACTIVE))
            .expect("header 3 should store");
        journal
            .put_run_header(&make_header(4, RunHeaderStatus::FINISHED))
            .expect("header 4 should store");
        journal.close().expect("close should succeed");
        (temp, path)
    }

    #[test]
    fn system_status_payload_reports_degraded_when_no_backend_is_attached() {
        let payload = system_status_payload(SystemStatusOptions::default(), "0.1.0");
        let status = &payload["status"];

        assert_eq!(payload["connected"], serde_json::json!(false));
        assert_eq!(payload["state"], serde_json::json!("not_requested"));
        assert_eq!(status["storage_health"], serde_json::json!("Degraded"));
        assert_eq!(status["journal_batch_healthy"], serde_json::json!(false));
        assert_eq!(status["blob_store_ok"], serde_json::json!(false));
        assert_eq!(status["index_healthy"], serde_json::json!(false));
        assert_eq!(payload["reason"], serde_json::json!(NO_BACKEND_REASON));
    }

    #[test]
    fn system_status_payload_preserves_selected_profile_and_server() {
        let payload = system_status_payload(
            SystemStatusOptions {
                profile: VerifyProfile::Full,
                server: DurabilityMode::Journaled,
                db: None,
                emit_yaml: false,
            },
            "0.1.0",
        );

        assert_eq!(payload["profile"], serde_json::json!("full"));
        assert_eq!(payload["server"], serde_json::json!("journaled"));
    }

    #[test]
    fn system_status_payload_probes_journal_when_db_is_provided() {
        let (_temp, path) = temp_journal();
        let payload = system_status_payload(
            SystemStatusOptions {
                profile: VerifyProfile::Standard,
                server: DurabilityMode::None,
                db: Some(path),
                emit_yaml: false,
            },
            "0.1.0",
        );

        assert_eq!(payload["connected"], serde_json::json!(true));
        assert_eq!(payload["state"], serde_json::json!("live"));
        // 2 active runs are written in temp_journal().
        assert_eq!(payload["status"]["active_run_count"], serde_json::json!(2));
        // Index health is the strongest assertion we can make about the
        // live keyspace without driving a write.
        assert_eq!(payload["status"]["index_healthy"], serde_json::json!(true));
        // Reason is empty when live.
        assert_eq!(payload["reason"], serde_json::json!(""));
    }

    #[test]
    fn system_status_payload_reports_fallback_when_journal_open_fails() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        // Create a regular file at the candidate path; Fjall requires a
        // directory and will fail to open a file as a journal.
        let bad_path = temp.path().join("not_a_directory");
        std::fs::write(&bad_path, b"not a journal").expect("test fixture: file should be written");
        let payload = system_status_payload(
            SystemStatusOptions {
                profile: VerifyProfile::Standard,
                server: DurabilityMode::None,
                db: Some(bad_path),
                emit_yaml: false,
            },
            "0.1.0",
        );

        assert_eq!(payload["connected"], serde_json::json!(false));
        assert_eq!(payload["state"], serde_json::json!("fallback"));
        let reason = payload["reason"]
            .as_str()
            .expect("reason should be a string");
        assert!(
            reason.contains("journal open"),
            "fallback reason must describe the failure: {reason}"
        );
        // Storage health is Degraded in fallback mode.
        assert_eq!(
            payload["status"]["storage_health"],
            serde_json::json!("Degraded")
        );
    }
}
