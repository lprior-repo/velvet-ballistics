//! Tests for the system-status command.

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

use super::output::system_status_payload;
use super::types::NO_BACKEND_REASON;
use crate::args::{DurabilityMode, SystemStatusOptions, VerifyProfile};
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
