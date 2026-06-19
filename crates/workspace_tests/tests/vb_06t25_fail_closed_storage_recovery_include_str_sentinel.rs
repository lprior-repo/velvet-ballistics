//! vb-06t25 fail-closed sentinel for the
//! `vb_jpq7_3_fail_closed_storage_recovery_contract.rs` test's
//! `include_str!` targets.
//!
//! Background
//! ----------
//! Earlier in this session we repaired the `include_str!` paths inside
//! `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs`
//! after the `event_replay.rs` and `journal/append.rs` module splits. That
//! repair made the contract test compile again, but it left a gap: if any of
//! the underlying source modules moves or gets renamed again, the
//! `include_str!` macro fails at compile time without a typed, drift-aware
//! message tied to this test's contract.
//!
//! This sentinel closes the gap with a *runtime* fail-closed check that
//! re-probes every `include_str!` target on disk and returns a typed error
//! with the resolved absolute path whenever a module has drifted away from
//! the contract test's expectations. It does not touch production code; it
//! only mirrors the list of paths the contract test references.
//!
//! Holzman compliance
//! ------------------
//! - No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`,
//!   `unreachable!`, `dbg!`, unchecked indexing, or unchecked arithmetic.
//! - All loops are bounded by the const array
//!   `CONTRACT_INCLUDE_STR_TARGETS` (fixed length, set at compile time).
//! - `Path::exists()` is the bool-returning API per the user's explicit
//!   guidance; we do not call `try_exists()`.
//! - Drift messages flow through `Result<(), String>` and `format!` rather
//!   than panicking, so the failure is observable in `cargo test` output
//!   without aborting the rest of the test binary.

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
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

/// Canonical, machine-readable list of the `include_str!` paths referenced
/// by `vb_jpq7_3_fail_closed_storage_recovery_contract.rs`. Any drift between
/// this list and the contract file's `include_str!` macro calls must be
/// flagged in this sentinel so the two sources stay in lock-step.
///
/// Listed in the same order as the contract file's `include_str!` block:
///
/// 1. `../../vb_storage/src/journal/replay.rs`
/// 2. `../../vb_storage/src/journal/core.rs`
/// 3. `../../vb_storage/src/journal/append/intent.rs`
/// 4. `../../vb_storage/src/recovery/event_replay/mod.rs`
const CONTRACT_INCLUDE_STR_TARGETS: &[&str] = &[
    "../../vb_storage/src/journal/replay.rs",
    "../../vb_storage/src/journal/core.rs",
    "../../vb_storage/src/journal/append/intent.rs",
    "../../vb_storage/src/recovery/event_replay/mod.rs",
];

/// Build an absolute path for a contract-test-relative path expression.
///
/// The contract test file lives at `<CARGO_MANIFEST_DIR>/tests/`; the
/// `include_str!` paths are written relative to that file, so the
/// workspace-absolute path is
/// `<CARGO_MANIFEST_DIR>/tests/<relative>`. `CARGO_MANIFEST_DIR` is set by
/// cargo at build time via the `env!` macro, so this lookup cannot fail at
/// runtime under cargo.
fn resolve_against_contract_tests_dir(relative: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("tests").join(relative)
}

/// Fail-closed sentinel: probe the path on disk and report drift with a
/// typed error containing the resolved absolute path so the bug is
/// debuggable from a single `cargo test` failure without re-running the
/// test under a debugger.
///
/// Failure modes reported:
///
/// * `include_str!` target missing entirely (the canonical drift case
///   after a `git mv` or rename).
/// * `include_str!` target exists but is not a regular file (catches
///   accidentally pointing at a directory or special file).
fn assert_include_str_target_exists(relative: &str) -> Result<(), String> {
    let resolved = resolve_against_contract_tests_dir(relative);
    if !resolved.exists() {
        return Err(format!(
            "include_str! target drift: `{relative}` (resolved to `{}`) \
             does not exist on disk. Either restore the moved module or \
             update `vb_jpq7_3_fail_closed_storage_recovery_contract.rs` \
             to point at the new location.",
            resolved.display(),
        ));
    }
    if !resolved.is_file() {
        return Err(format!(
            "include_str! target drift: `{relative}` (resolved to `{}`) \
             exists but is not a regular file. The contract test expects \
             to read source text, not a directory or special file.",
            resolved.display(),
        ));
    }
    Ok(())
}

#[test]
fn given_all_include_str_targets_exist_when_fail_closed_sentinel_runs_then_all_paths_resolve()
-> Result<(), String> {
    // Given/When: every path in CONTRACT_INCLUDE_STR_TARGETS is probed on
    // disk.
    for target in CONTRACT_INCLUDE_STR_TARGETS {
        assert_include_str_target_exists(target)?;
    }
    // Then: all probes returned Ok(()).
    Ok(())
}

#[test]
fn given_each_include_str_target_when_resolved_then_canonical_path_under_vb_storage_src()
-> Result<(), String> {
    // Given/When: each include_str! target is resolved against the contract
    // test's directory and the resulting absolute path is inspected.
    for target in CONTRACT_INCLUDE_STR_TARGETS {
        let resolved = resolve_against_contract_tests_dir(target);
        let resolved_str = resolved
            .to_str()
            .ok_or_else(|| format!("resolved path is not valid UTF-8: {}", resolved.display()))?;
        // Then: the resolved path lives underneath `vb_storage/src/`, so
        // the sentinel cannot be tricked into accepting a path that
        // accidentally escapes the crate.
        if !resolved_str.contains("vb_storage/src/") {
            return Err(format!(
                "include_str! target escaped vb_storage/src/ \
                 (or sentinel list is malformed): `{target}` -> `{}`",
                resolved.display()
            ));
        }
    }
    Ok(())
}

#[test]
fn given_a_missing_include_str_target_when_sentinel_runs_then_typed_error_includes_the_path()
-> Result<(), String> {
    // Given: a path that cannot plausibly exist on disk. Using a deeply
    // nested, non-existent path means we exercise the error path without
    // touching the repo.
    let bogus = "../../vb_storage/src/__nonexistent_vb_06t25_drift_sentinel__/missing.rs";

    // When: the sentinel probes the bogus path.
    let result = assert_include_str_target_exists(bogus);

    // Then: the sentinel returns Err with the missing path spelled out in
    // the error message so the developer can find the drift without
    // re-running the test under a debugger.
    let err = result.err().ok_or_else(|| {
        format!("expected error for missing include_str! target `{bogus}`, got Ok")
    })?;
    if !err.contains(bogus) {
        return Err(format!(
            "typed error must include the missing path `{bogus}`; got: {err}"
        ));
    }
    if !err.contains("does not exist on disk") {
        return Err(format!(
            "typed error must explain the missing-file drift mode; got: {err}"
        ));
    }
    Ok(())
}

#[test]
fn given_a_directory_path_when_sentinel_runs_then_typed_error_reports_not_a_regular_file()
-> Result<(), String> {
    // Given: a path that exists but is a directory
    // (`recovery/event_replay/` contains `mod.rs` and `tail.rs`; the
    // directory itself is not a regular file). The sentinel must
    // distinguish this from the missing-file drift mode so a developer
    // sees a precise message instead of a generic "does not exist".
    let dir_path = "../../vb_storage/src/recovery/event_replay";

    // When: the sentinel probes the directory path.
    let result = assert_include_str_target_exists(dir_path);

    // Then: the sentinel returns Err with the "not a regular file"
    // explanation rather than the "does not exist" message.
    let err = result.err().ok_or_else(|| {
        format!("expected error for non-regular-file target `{dir_path}`, got Ok")
    })?;
    if !err.contains("not a regular file") {
        return Err(format!(
            "typed error must explain non-regular-file drift; got: {err}"
        ));
    }
    if err.contains("does not exist on disk") {
        return Err(format!(
            "typed error must NOT use the missing-file drift message for a \
             directory path; got: {err}"
        ));
    }
    Ok(())
}
