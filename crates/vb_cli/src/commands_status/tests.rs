//! Tests for the runtime status command modules.

#![forbid(unsafe_code)]
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

use crate::args::StatusOptions;

use super::build::build_status;
use super::types::{CliStatus, DbProbeStatus};

use vb_core::{RunId, WorkflowDigest, WorkflowId};
use vb_storage::records::{KnownRunHeaderStatus, RunHeaderRecord, RunHeaderStatus};

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
    // Mix of pending, active, and finished runs to exercise every branch.
    journal
        .put_run_header(&make_header(1, RunHeaderStatus::PENDING))
        .expect("header 1 should store");
    journal
        .put_run_header(&make_header(2, RunHeaderStatus::ACCEPTED))
        .expect("header 2 should store");
    journal
        .put_run_header(&make_header(3, RunHeaderStatus::ACTIVE))
        .expect("header 3 should store");
    journal
        .put_run_header(&make_header(4, RunHeaderStatus::FINISHED))
        .expect("header 4 should store");
    // 5 is intentionally an unknown status byte (255) to exercise
    // the classify() "Err" branch.
    journal
        .put_run_header(&make_header(5, RunHeaderStatus::from_byte(255)))
        .expect("header 5 should store");
    journal.close().expect("close should succeed");
    (temp, path)
}

#[test]
fn build_status_reports_default_no_runtime_shard() {
    let status = build_status(StatusOptions::default());
    assert_eq!(status.health, "running");
    assert!(status.running);
    assert!(!status.shutting_down);
    assert_eq!(status.command_queue_depth, 0);
    assert_eq!(status.command_queue_capacity, 1024);
    assert_eq!(status.active_runs, 0);
    assert_eq!(status.max_active_runs, 1024);
    assert_eq!(status.trace_capacity, 4096);
    assert_eq!(status.trace_dropped, 0);
    assert_eq!(status.step_budget_per_tick, 1000);
    assert_eq!(status.runtime_policy, "Strict");
    assert_eq!(status.db_probe_status, DbProbeStatus::NotRequested);
    assert!(status.db_probe_reason.is_empty());
}

#[test]
fn build_status_applies_diagnostic_overlays_without_mutation() {
    let status = build_status(StatusOptions {
        active_runs: Some(5),
        queue_depth: Some(3),
        trace_dropped: Some(0),
        db: None,
        emit_yaml: false,
    });
    assert_eq!(status.active_runs, 5);
    assert_eq!(status.command_queue_depth, 3);
    assert_eq!(status.trace_dropped, 0);
    assert_eq!(status.db_probe_status, DbProbeStatus::NotRequested);
}

#[test]
fn build_status_reports_overlay_values_without_silent_clamping() {
    let status = build_status(StatusOptions {
        active_runs: Some(2048),
        queue_depth: Some(2048),
        trace_dropped: Some(7),
        db: None,
        emit_yaml: false,
    });
    assert_eq!(status.active_runs, 2048);
    assert_eq!(status.command_queue_depth, 2048);
    assert_eq!(status.trace_dropped, 7);
}

#[test]
fn build_status_probes_journal_when_db_is_provided() {
    let (_temp, path) = temp_journal();
    let status = build_status(StatusOptions {
        db: Some(path),
        ..StatusOptions::default()
    });
    assert_eq!(status.db_probe_status, DbProbeStatus::Live);
    assert!(status.db_probe_reason.is_empty());
    // 1 pending + 1 accepted = 2 queued; 1 active.
    assert_eq!(status.command_queue_depth, 2);
    assert_eq!(status.active_runs, 1);
}

#[test]
fn build_status_reports_fallback_when_journal_open_fails() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    // Create a regular file at the candidate path; Fjall requires a
    // directory and will fail to open a file as a journal.
    let bad_path = temp.path().join("not_a_directory");
    std::fs::write(&bad_path, b"not a journal").expect("test fixture: file should be written");
    let status = build_status(StatusOptions {
        db: Some(bad_path.clone()),
        ..StatusOptions::default()
    });
    assert_eq!(status.db_probe_status, DbProbeStatus::Fallback);
    assert!(
        !status.db_probe_reason.is_empty(),
        "fallback reason must be populated when journal open fails"
    );
    assert!(
        status.db_probe_reason.contains("journal open"),
        "fallback reason should describe the failure: {0}",
        status.db_probe_reason
    );
    // Active runs fall back to the transient shard default.
    assert_eq!(status.active_runs, 0);
    assert_eq!(status.command_queue_depth, 0);
}

#[test]
fn db_probe_status_name_returns_stable_labels() {
    assert_eq!(
        super::types::db_probe_status_name(DbProbeStatus::NotRequested),
        "not_requested"
    );
    assert_eq!(
        super::types::db_probe_status_name(DbProbeStatus::Live),
        "live"
    );
    assert_eq!(
        super::types::db_probe_status_name(DbProbeStatus::Fallback),
        "fallback"
    );
}

#[test]
fn run_header_status_known_classification_matches_status_byte() {
    // Pin the status-byte mapping that the live probe depends on.
    assert!(matches!(
        KnownRunHeaderStatus::try_from(RunHeaderStatus::PENDING.as_byte()),
        Ok(KnownRunHeaderStatus::Pending)
    ));
    assert!(matches!(
        KnownRunHeaderStatus::try_from(RunHeaderStatus::ACCEPTED.as_byte()),
        Ok(KnownRunHeaderStatus::Accepted)
    ));
    assert!(matches!(
        KnownRunHeaderStatus::try_from(RunHeaderStatus::ACTIVE.as_byte()),
        Ok(KnownRunHeaderStatus::Active)
    ));
    assert!(matches!(
        KnownRunHeaderStatus::try_from(RunHeaderStatus::FINISHED.as_byte()),
        Ok(KnownRunHeaderStatus::Finished)
    ));
}
