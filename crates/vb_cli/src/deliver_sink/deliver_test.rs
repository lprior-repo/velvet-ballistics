//! Unit tests for the deliver sink pipeline.
//!
//! These tests exercise every error path and race condition the publish
//! lifecycle can encounter, using `test_support` hooks to inject failures at
//! precise points in the workflow.

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
    clippy::cmp_owned,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::const_is_empty,
    clippy::derivable_impls,
    clippy::duplicated_attributes,
    clippy::enum_variant_names,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::identity_op,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::if_same_then_else,
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
    clippy::manual_contains,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_range_contains,
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
    clippy::multiple_bound_locations,
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
    clippy::option_as_ref_cloned,
    clippy::option_as_ref_deref,
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
    clippy::redundant_field_names,
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
    clippy::too_many_arguments,
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
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    unused_imports,
    dead_code,
    unused_variables
)]

use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::atomic_publish::{
    hashed_temp_name, preferred_temp_name, temp_stage_name, write_json_line,
};
use super::deliver_error::{DeliverSinkError, MAX_TEMP_STAGE_ATTEMPTS};
use super::deliver_target::{DeliverTarget, parse_deliver_target};
use super::deliver_test_support::test_support::{
    self, FinalPathChange, HookConfig, ParentChange, PostCommitParentChange,
};

#[cfg(unix)]
#[test]
fn parse_deliver_target_resolves_parent_symlink_before_storing_new_file_path() -> Result<(), String>
{
    let temp_dir = repo_tempdir("vb-deliver-parse-symlink-")?;
    let real_parent = temp_dir.path().join("real-parent");
    std::fs::create_dir(&real_parent).map_err(|error| error.to_string())?;
    let alias_parent = temp_dir.path().join("alias-parent");
    std::os::unix::fs::symlink(&real_parent, &alias_parent).map_err(|error| error.to_string())?;

    let requested_path = alias_parent.join("agent-context.jsonl");
    let target = format!("file:{}", path_text(&requested_path)?);

    match parse_deliver_target(&target).map_err(|error| error.to_string())? {
        DeliverTarget::NewFile(target) => {
            let expected = std::fs::canonicalize(&real_parent)
                .map_err(|error| error.to_string())?
                .join("agent-context.jsonl");
            if target.delivery_path() == expected {
                Ok(())
            } else {
                Err(format!(
                    "expected resolved delivery path {}, got {}",
                    expected.display(),
                    target.delivery_path().display()
                ))
            }
        }
        DeliverTarget::Stdout => Err(String::from("expected file delivery target")),
    }
}

#[cfg(unix)]
#[test]
fn write_json_line_reports_parent_changed_when_parent_path_swaps_before_write() -> Result<(), String>
{
    let temp_dir = repo_tempdir("vb-deliver-parent-swap-")?;
    let real_parent = temp_dir.path().join("real-parent");
    std::fs::create_dir(&real_parent).map_err(|error| error.to_string())?;
    let alias_parent = temp_dir.path().join("alias-parent");
    std::os::unix::fs::symlink(&real_parent, &alias_parent).map_err(|error| error.to_string())?;

    let requested_path = alias_parent.join("agent-context.jsonl");
    let target = parse_deliver_target(&format!("file:{}", path_text(&requested_path)?))
        .map_err(|error| error.to_string())?;

    let moved_parent = temp_dir.path().join("moved-parent");
    std::fs::rename(&real_parent, &moved_parent).map_err(|error| error.to_string())?;
    std::fs::create_dir(&real_parent).map_err(|error| error.to_string())?;

    match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
        Err(DeliverSinkError::ParentChanged) => {}
        Err(error) => return Err(format!("expected ParentChanged, got {error}")),
        Ok(()) => {
            return Err(String::from(
                "expected ParentChanged after parent path swap",
            ));
        }
    }

    assert_directory_entries_exact(&moved_parent, &[])?;
    assert_directory_entries_exact(&real_parent, &[])
}

#[test]
fn write_json_line_reports_existing_file_when_rival_created_after_parse_and_cleans_stage()
-> Result<(), String> {
    let temp_dir = repo_tempdir("vb-deliver-rival-file-")?;
    let deliver_path = temp_dir.path().join("agent-context.jsonl");
    let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
        .map_err(|error| error.to_string())?;
    std::fs::write(&deliver_path, "rival file\n").map_err(|error| error.to_string())?;

    match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
        Err(DeliverSinkError::ExistingFile) => {}
        Err(error) => return Err(format!("expected ExistingFile race error, got {error}")),
        Ok(()) => return Err(String::from("expected ExistingFile race error")),
    }

    let rival_contents =
        std::fs::read_to_string(&deliver_path).map_err(|error| error.to_string())?;
    if rival_contents != String::from("rival file\n") {
        return Err(format!(
            "expected rival file contents to remain unchanged, got {rival_contents:?}"
        ));
    }

    assert_directory_entries_exact(temp_dir.path(), &["agent-context.jsonl"])?;
    assert_no_stage_name_exists(&deliver_path)
}

#[test]
fn write_json_line_surfaces_existing_file_after_linkat_exist_when_temp_unlink_also_fails()
-> Result<(), String> {
    // Covers the cleanup-failure-after-linkat-EXIST branch:
    // `linkat(parent, temp, parent, final)` returns EXIST because the
    // rival pre-created the final file, then `cleanup_unpublished_temp_file`
    // is forced to fail its unlinkat via the test hook. The contract is
    // that the *original* semantic error (`ExistingFile`) is surfaced
    // rather than the unlinkat error.
    let temp_dir = repo_tempdir("vb-deliver-rival-file-cleanup-fail-")?;
    let deliver_path = temp_dir.path().join("agent-context.jsonl");
    let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
        .map_err(|error| error.to_string())?;
    std::fs::write(&deliver_path, "rival file\n").map_err(|error| error.to_string())?;
    let _hooks = test_support::install(HookConfig {
        cleanup_failures: vec![OsString::from(".agent-context.jsonl.tmp")],
        ..Default::default()
    });

    match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
        Err(DeliverSinkError::ExistingFile) => {}
        Err(error) => {
            return Err(format!(
                "expected ExistingFile surfaced after cleanup failure, got {error}"
            ));
        }
        Ok(()) => {
            return Err(String::from(
                "expected ExistingFile surfaced after cleanup failure",
            ));
        }
    }

    let rival_contents =
        std::fs::read_to_string(&deliver_path).map_err(|error| error.to_string())?;
    if rival_contents != String::from("rival file\n") {
        return Err(format!(
            "expected rival file contents to remain unchanged, got {rival_contents:?}"
        ));
    }
    Ok(())
}

#[test]
fn write_json_line_reports_staging_unavailable_when_all_stage_names_are_taken() -> Result<(), String>
{
    let temp_dir = repo_tempdir("vb-deliver-stage-exhaust-")?;
    let deliver_path = temp_dir.path().join("agent-context.jsonl");
    occupy_all_stage_names(&deliver_path)?;
    let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
        .map_err(|error| error.to_string())?;

    match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
        Err(DeliverSinkError::StagingUnavailable) => {}
        Err(error) => {
            return Err(format!(
                "expected StagingUnavailable after exhausting stage names, got {error}"
            ));
        }
        Ok(()) => return Err(String::from("expected staging-unavailable error")),
    }

    if deliver_path.exists() {
        return Err(format!(
            "expected no delivered file after stage exhaustion, found {}",
            deliver_path.display()
        ));
    }

    Ok(())
}

#[cfg(unix)]
#[test]
fn parse_deliver_target_reports_parent_changed_when_parent_inode_changes_during_validation()
-> Result<(), String> {
    let temp_dir = repo_tempdir("vb-deliver-parent-changed-")?;
    let parent = temp_dir.path().join("deliver-parent");
    std::fs::create_dir(&parent).map_err(|error| error.to_string())?;
    let moved_parent = temp_dir.path().join("moved-parent");
    let _hooks = test_support::install(HookConfig {
        parent_change: Some(ParentChange::ReplaceOpenedPathWithNewDirectory {
            moved_to: moved_parent,
        }),
        ..Default::default()
    });

    let deliver_path = parent.join("agent-context.jsonl");
    match parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?)) {
        Err(DeliverSinkError::ParentChanged) => {}
        Err(error) => {
            return Err(format!(
                "expected ParentChanged during validation, got {error}"
            ));
        }
        Ok(_) => return Err(String::from("expected ParentChanged during validation")),
    }

    assert_directory_entries_exact(temp_dir.path(), &["deliver-parent", "moved-parent"])
}

#[test]
fn write_json_line_returns_sync_error_when_rollback_after_parent_sync_failure_is_durable()
-> Result<(), String> {
    let temp_dir = repo_tempdir("vb-deliver-parent-sync-rollback-")?;
    let deliver_path = temp_dir.path().join("agent-context.jsonl");
    let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
        .map_err(|error| error.to_string())?;
    let sync_error = DeliverSinkError::Io(std::io::ErrorKind::PermissionDenied);
    let _hooks = test_support::install(HookConfig {
        sync_results: VecDeque::from([Err(sync_error), Ok(())]),
        ..Default::default()
    });

    match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
        Err(DeliverSinkError::Io(std::io::ErrorKind::PermissionDenied)) => {}
        Err(error) => {
            return Err(format!(
                "expected original parent sync error after durable rollback, got {error}"
            ));
        }
        Ok(()) => return Err(String::from("expected parent sync failure")),
    }

    if deliver_path.exists() {
        return Err(format!(
            "expected durable rollback to remove final path, found {}",
            deliver_path.display()
        ));
    }

    assert_directory_entries_exact(temp_dir.path(), &[])?;
    assert_no_stage_name_exists(&deliver_path)
}

#[cfg(unix)]
#[test]
fn write_json_line_reports_parent_changed_when_parent_path_swaps_after_staging_before_linkat()
-> Result<(), String> {
    let temp_dir = repo_tempdir("vb-deliver-pre-link-parent-swap-")?;
    let real_parent = temp_dir.path().join("real-parent");
    let replacement_parent = temp_dir.path().join("replacement-parent");
    let moved_parent = temp_dir.path().join("moved-parent");
    std::fs::create_dir(&real_parent).map_err(|error| error.to_string())?;
    std::fs::create_dir(&replacement_parent).map_err(|error| error.to_string())?;

    let deliver_path = real_parent.join("agent-context.jsonl");
    let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
        .map_err(|error| error.to_string())?;
    let _hooks = test_support::install(HookConfig {
        before_link_parent_change: Some(PostCommitParentChange::ReplaceResolvedPathWithSymlink {
            moved_to: moved_parent.clone(),
            replacement: replacement_parent.clone(),
        }),
        ..Default::default()
    });

    match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
        Err(DeliverSinkError::ParentChanged) => {}
        Err(error) => return Err(format!("expected ParentChanged, got {error}")),
        Ok(()) => return Err(String::from("expected ParentChanged after pre-link swap")),
    }

    if deliver_path.exists() {
        return Err(format!(
            "expected no delivered file after pre-link swap, found {}",
            deliver_path.display()
        ));
    }

    assert_directory_entries_exact(&moved_parent, &[])?;
    assert_directory_entries_exact(&replacement_parent, &[])?;
    assert_no_stage_name_exists(&moved_parent.join("agent-context.jsonl"))
}

#[cfg(unix)]
#[test]
fn write_json_line_reports_parent_changed_and_rolls_back_when_parent_path_swaps_after_link_sync_before_temp_cleanup()
-> Result<(), String> {
    let temp_dir = repo_tempdir("vb-deliver-post-link-parent-swap-")?;
    let real_parent = temp_dir.path().join("real-parent");
    let replacement_parent = temp_dir.path().join("replacement-parent");
    let moved_parent = temp_dir.path().join("moved-parent");
    std::fs::create_dir(&real_parent).map_err(|error| error.to_string())?;
    std::fs::create_dir(&replacement_parent).map_err(|error| error.to_string())?;

    let deliver_path = real_parent.join("agent-context.jsonl");
    let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
        .map_err(|error| error.to_string())?;
    let _hooks = test_support::install(HookConfig {
        after_link_sync_parent_change: Some(
            PostCommitParentChange::ReplaceResolvedPathWithSymlink {
                moved_to: moved_parent.clone(),
                replacement: replacement_parent.clone(),
            },
        ),
        ..Default::default()
    });

    match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
        Err(DeliverSinkError::ParentChanged) => {}
        Err(error) => return Err(format!("expected ParentChanged, got {error}")),
        Ok(()) => {
            return Err(String::from(
                "expected ParentChanged after post-link parent swap",
            ));
        }
    }

    let moved_path = moved_parent.join("agent-context.jsonl");
    if deliver_path.exists() {
        return Err(format!(
            "expected rollback to remove final path after post-link swap, found {}",
            deliver_path.display()
        ));
    }
    if moved_path.exists() {
        return Err(format!(
            "expected rollback to remove moved final path after post-link swap, found {}",
            moved_path.display()
        ));
    }

    assert_directory_entries_exact(&moved_parent, &[])?;
    assert_directory_entries_exact(&replacement_parent, &[])?;
    assert_no_stage_name_exists(&moved_path)
}

#[test]
fn write_json_line_reports_publish_state_unknown_after_parent_sync_failure_when_rollback_is_not_durable()
-> Result<(), String> {
    let temp_dir = repo_tempdir("vb-deliver-parent-sync-failure-")?;
    let deliver_path = temp_dir.path().join("agent-context.jsonl");
    let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
        .map_err(|error| error.to_string())?;
    let sync_error = DeliverSinkError::Io(std::io::ErrorKind::PermissionDenied);
    let _hooks = test_support::install(HookConfig {
        sync_results: VecDeque::from([Err(sync_error), Err(sync_error)]),
        ..Default::default()
    });

    match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
        Err(DeliverSinkError::PublishStateUnknown) => {}
        Err(error) => {
            return Err(format!(
                "expected PublishStateUnknown after undurable rollback, got {error}"
            ));
        }
        Ok(()) => return Err(String::from("expected parent sync failure")),
    }

    if deliver_path.exists() {
        return Err(format!(
            "expected final file rollback after parent sync failure, found {}",
            deliver_path.display()
        ));
    }

    assert_directory_entries_exact(temp_dir.path(), &[])?;
    assert_no_stage_name_exists(&deliver_path)
}

#[test]
fn write_json_line_reports_publish_state_unknown_when_temp_unlink_fails_after_publish()
-> Result<(), String> {
    let temp_dir = repo_tempdir("vb-deliver-temp-unlink-failure-")?;
    let deliver_path = temp_dir.path().join("agent-context.jsonl");
    let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
        .map_err(|error| error.to_string())?;
    let _hooks = test_support::install(HookConfig {
        cleanup_failures: vec![OsString::from(".agent-context.jsonl.tmp")],
        sync_results: VecDeque::from([Ok(())]),
        ..Default::default()
    });

    match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
        Err(DeliverSinkError::PublishStateUnknown) => {}
        Err(error) => {
            return Err(format!(
                "expected PublishStateUnknown after temp unlink failure, got {error}"
            ));
        }
        Ok(()) => {
            return Err(String::from(
                "expected PublishStateUnknown after temp unlink failure",
            ));
        }
    }

    assert_json_line_file_equals(&deliver_path, &serde_json::json!({"kind": "AgentContext"}))?;
    assert_directory_entries_exact(
        temp_dir.path(),
        &[".agent-context.jsonl.tmp", "agent-context.jsonl"],
    )
}

#[test]
fn write_json_line_reports_publish_state_unknown_when_temp_unlink_sync_fails_after_publish()
-> Result<(), String> {
    let temp_dir = repo_tempdir("vb-deliver-temp-unlink-sync-failure-")?;
    let deliver_path = temp_dir.path().join("agent-context.jsonl");
    let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
        .map_err(|error| error.to_string())?;
    let sync_error = DeliverSinkError::Io(std::io::ErrorKind::PermissionDenied);
    let _hooks = test_support::install(HookConfig {
        sync_results: VecDeque::from([Ok(()), Err(sync_error)]),
        ..Default::default()
    });

    match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
        Err(DeliverSinkError::PublishStateUnknown) => {}
        Err(error) => {
            return Err(format!(
                "expected PublishStateUnknown after temp unlink sync failure, got {error}"
            ));
        }
        Ok(()) => {
            return Err(String::from(
                "expected PublishStateUnknown after temp unlink sync failure",
            ));
        }
    }

    assert_json_line_file_equals(&deliver_path, &serde_json::json!({"kind": "AgentContext"}))?;
    assert_directory_entries_exact(temp_dir.path(), &["agent-context.jsonl"])?;
    assert_no_stage_name_exists(&deliver_path)
}

#[test]
fn write_json_line_reports_publish_state_unknown_when_rival_unlinks_final_path_after_publish()
-> Result<(), String> {
    let temp_dir = repo_tempdir("vb-deliver-post-commit-final-unlink-")?;
    let deliver_path = temp_dir.path().join("agent-context.jsonl");
    let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
        .map_err(|error| error.to_string())?;
    let _hooks = test_support::install(HookConfig {
        post_commit_final_path_change: Some(FinalPathChange::UnlinkFinalPath),
        sync_results: VecDeque::from([Ok(()), Ok(())]),
        ..Default::default()
    });

    match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
        Err(DeliverSinkError::PublishStateUnknown) => {}
        Err(error) => {
            return Err(format!(
                "expected PublishStateUnknown after rival final unlink, got {error}"
            ));
        }
        Ok(()) => {
            return Err(String::from(
                "expected PublishStateUnknown after rival final unlink",
            ));
        }
    }

    if deliver_path.exists() {
        return Err(format!(
            "expected rival final unlink to remove {}, but it still exists",
            deliver_path.display()
        ));
    }

    assert_directory_entries_exact(temp_dir.path(), &[])?;
    assert_no_stage_name_exists(&deliver_path)
}

#[test]
fn write_json_line_reports_publish_state_unknown_when_rival_replaces_final_path_after_publish()
-> Result<(), String> {
    let temp_dir = repo_tempdir("vb-deliver-post-commit-final-replace-")?;
    let deliver_path = temp_dir.path().join("agent-context.jsonl");
    let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
        .map_err(|error| error.to_string())?;
    let _hooks = test_support::install(HookConfig {
        post_commit_final_path_change: Some(FinalPathChange::ReplaceFinalPath),
        sync_results: VecDeque::from([Ok(()), Ok(())]),
        ..Default::default()
    });

    match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
        Err(DeliverSinkError::PublishStateUnknown) => {}
        Err(error) => {
            return Err(format!(
                "expected PublishStateUnknown after rival final replace, got {error}"
            ));
        }
        Ok(()) => {
            return Err(String::from(
                "expected PublishStateUnknown after rival final replace",
            ));
        }
    }

    let rival_contents =
        std::fs::read_to_string(&deliver_path).map_err(|error| error.to_string())?;
    if rival_contents != String::from("rival replacement\n") {
        return Err(format!(
            "expected rival replacement to occupy final path, got {rival_contents:?}"
        ));
    }

    assert_directory_entries_exact(temp_dir.path(), &["agent-context.jsonl"])?;
    assert_no_stage_name_exists(&deliver_path)
}

#[test]
fn write_json_line_reports_publish_state_unknown_when_rollback_leaves_temp_link()
-> Result<(), String> {
    let temp_dir = repo_tempdir("vb-deliver-temp-rollback-link-")?;
    let deliver_path = temp_dir.path().join("agent-context.jsonl");
    let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
        .map_err(|error| error.to_string())?;
    let sync_error = DeliverSinkError::Io(std::io::ErrorKind::PermissionDenied);
    let _hooks = test_support::install(HookConfig {
        cleanup_failures: vec![OsString::from(".agent-context.jsonl.tmp")],
        sync_results: VecDeque::from([Err(sync_error), Ok(())]),
        ..Default::default()
    });

    match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
        Err(DeliverSinkError::PublishStateUnknown) => {}
        Err(error) => {
            return Err(format!(
                "expected PublishStateUnknown after incomplete rollback, got {error}"
            ));
        }
        Ok(()) => {
            return Err(String::from(
                "expected PublishStateUnknown after incomplete rollback",
            ));
        }
    }

    if deliver_path.exists() {
        return Err(format!(
            "expected rollback to remove final path {}, but it remains",
            deliver_path.display()
        ));
    }

    let temp_stage_path = temp_dir.path().join(".agent-context.jsonl.tmp");
    assert_json_line_file_equals(
        &temp_stage_path,
        &serde_json::json!({"kind": "AgentContext"}),
    )?;
    assert_directory_entries_exact(temp_dir.path(), &[".agent-context.jsonl.tmp"])
}

#[cfg(unix)]
#[test]
fn write_json_line_preserves_published_file_when_parent_path_swaps_after_final_sync()
-> Result<(), String> {
    let temp_dir = repo_tempdir("vb-deliver-post-commit-parent-swap-")?;
    let real_parent = temp_dir.path().join("real-parent");
    let replacement_parent = temp_dir.path().join("replacement-parent");
    let moved_parent = temp_dir.path().join("moved-parent");
    std::fs::create_dir(&real_parent).map_err(|error| error.to_string())?;
    std::fs::create_dir(&replacement_parent).map_err(|error| error.to_string())?;

    let deliver_path = real_parent.join("agent-context.jsonl");
    let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
        .map_err(|error| error.to_string())?;
    let _hooks = test_support::install(HookConfig {
        post_commit_parent_change: Some(PostCommitParentChange::ReplaceResolvedPathWithSymlink {
            moved_to: moved_parent.clone(),
            replacement: replacement_parent.clone(),
        }),
        sync_results: VecDeque::from([Ok(()), Ok(())]),
        ..Default::default()
    });

    match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
        Err(DeliverSinkError::PublishStateUnknown) => {}
        Err(error) => {
            return Err(format!(
                "expected PublishStateUnknown after post-commit parent swap, got {error}"
            ));
        }
        Ok(()) => {
            return Err(String::from(
                "expected PublishStateUnknown after post-commit parent swap",
            ));
        }
    }

    let moved_path = moved_parent.join("agent-context.jsonl");
    assert_json_line_file_equals(&moved_path, &serde_json::json!({"kind": "AgentContext"}))?;
    if deliver_path.exists() {
        return Err(format!(
            "expected replaced path to stay empty after parent swap, found {}",
            deliver_path.display()
        ));
    }

    assert_directory_entries_exact(&moved_parent, &["agent-context.jsonl"])?;
    assert_directory_entries_exact(&replacement_parent, &[])?;
    assert_no_stage_name_exists(&moved_path)
}

#[test]
fn write_json_line_reports_publish_state_unknown_when_final_cleanup_fails_after_parent_sync_failure()
-> Result<(), String> {
    let temp_dir = repo_tempdir("vb-deliver-parent-cleanup-failure-")?;
    let deliver_path = temp_dir.path().join("agent-context.jsonl");
    let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
        .map_err(|error| error.to_string())?;
    let _hooks = test_support::install(HookConfig {
        cleanup_failures: vec![OsString::from("agent-context.jsonl")],
        sync_results: VecDeque::from([Err(DeliverSinkError::Io(
            std::io::ErrorKind::PermissionDenied,
        ))]),
        ..Default::default()
    });

    match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
        Err(DeliverSinkError::PublishStateUnknown) => {}
        Err(error) => {
            return Err(format!(
                "expected PublishStateUnknown after forced cleanup failure, got {error}"
            ));
        }
        Ok(()) => {
            return Err(String::from(
                "expected PublishStateUnknown after forced cleanup failure",
            ));
        }
    }

    if !deliver_path.exists() {
        return Err(format!(
            "expected final path to remain after forced cleanup failure, missing {}",
            deliver_path.display()
        ));
    }

    assert_json_line_file_equals(&deliver_path, &serde_json::json!({"kind": "AgentContext"}))?;
    assert_directory_entries_exact(temp_dir.path(), &["agent-context.jsonl"])?;
    assert_no_stage_name_exists(&deliver_path)
}

#[test]
fn created_file_mode_is_owner_only() -> Result<(), String> {
    // Drives the production `openat(... OFlags::CREATE | OFlags::EXCL,
    // MODE)` path on a real file, then `fstat`s the resulting descriptor
    // to confirm the kernel-observed mode is owner-only.
    let temp_dir = repo_tempdir("vb-deliver-mode-")?;
    let parent_fd = rustix::fs::openat(
        rustix::fs::CWD,
        temp_dir.path(),
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY,
        rustix::fs::Mode::empty(),
    )
    .map_err(|error| error.to_string())?;
    let file_name = std::ffi::OsStr::new("vb-deliver-mode-probe");
    let file = rustix::fs::openat(
        &parent_fd,
        file_name,
        rustix::fs::OFlags::WRONLY
            | rustix::fs::OFlags::CREATE
            | rustix::fs::OFlags::EXCL
            | rustix::fs::OFlags::CLOEXEC,
        super::deliver_error::MODE,
    )
    .map(std::fs::File::from)
    .map_err(|error| error.to_string())?;
    let stat = rustix::fs::fstat(&file).map_err(|error| error.to_string())?;
    let mode = stat.st_mode & 0o777;
    if mode & 0o600 == 0o600 && mode & 0o077 == 0 {
        Ok(())
    } else {
        Err(format!(
            "expected owner-only mode 0o600, got {mode:o} (full mode 0o{:o})",
            stat.st_mode & 0o7777
        ))
    }
}

// ---------------------------------------------------------------------------
// Test utilities
// ---------------------------------------------------------------------------

fn path_text(path: &std::path::Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| String::from("test path must be UTF-8"))
}

fn repo_temp_root() -> Result<PathBuf, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/deliver-sink-tmp");
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    std::fs::canonicalize(&root).map_err(|error| error.to_string())
}

fn repo_tempdir(prefix: &str) -> Result<tempfile::TempDir, String> {
    let root = repo_temp_root()?;
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(root)
        .map_err(|error| error.to_string())
}

fn exact_json_line_bytes(value: &serde_json::Value) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn assert_json_line_file_equals(
    path: &std::path::Path,
    expected_value: &serde_json::Value,
) -> Result<(), String> {
    let actual = std::fs::read(path).map_err(|error| error.to_string())?;
    let expected = exact_json_line_bytes(expected_value)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected exact JSONL bytes {:?} at {}, got {:?}",
            expected,
            path.display(),
            actual
        ))
    }
}

fn assert_directory_entries_exact(directory: &Path, expected: &[&str]) -> Result<(), String> {
    let mut actual_entries = Vec::new();
    for entry_result in std::fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry_result.map_err(|error| error.to_string())?;
        actual_entries.push(entry.file_name().to_string_lossy().into_owned());
    }
    actual_entries.sort();

    let mut expected_entries = expected
        .iter()
        .map(|entry| String::from(*entry))
        .collect::<Vec<_>>();
    expected_entries.sort();

    if actual_entries == expected_entries {
        Ok(())
    } else {
        Err(format!(
            "expected directory entries {:?} at {}, got {:?}",
            expected_entries,
            directory.display(),
            actual_entries
        ))
    }
}

fn occupy_all_stage_names(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| String::from("deliver path is missing parent"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| String::from("deliver path is missing file name"))?;
    let resolved_parent = std::fs::canonicalize(parent).map_err(|error| error.to_string())?;
    let resolved_path = resolved_parent.join(file_name);
    let base_names = [
        preferred_temp_name(file_name),
        hashed_temp_name(&resolved_path),
        OsString::from(".tmp"),
        OsString::from(".t"),
    ];

    for base_name in base_names {
        for attempt in 0..MAX_TEMP_STAGE_ATTEMPTS {
            let candidate = resolved_parent.join(temp_stage_name(&base_name, attempt));
            std::fs::write(&candidate, b"occupied stage\n").map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

fn assert_no_stage_name_exists(path: &std::path::Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| String::from("deliver path is missing parent"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| String::from("deliver path is missing file name"))?;
    let resolved_parent = std::fs::canonicalize(parent).map_err(|error| error.to_string())?;
    let resolved_path = resolved_parent.join(file_name);
    let base_names = [
        preferred_temp_name(file_name),
        hashed_temp_name(&resolved_path),
        OsString::from(".tmp"),
        OsString::from(".t"),
    ];

    for base_name in base_names {
        for attempt in 0..MAX_TEMP_STAGE_ATTEMPTS {
            let candidate = resolved_parent.join(temp_stage_name(&base_name, attempt));
            if candidate.exists() {
                return Err(format!(
                    "expected no leaked stage file, found {}",
                    candidate.display()
                ));
            }
        }
    }

    Ok(())
}
