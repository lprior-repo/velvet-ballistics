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
//! Snapshot and determinism tests for agent-context command.

use serde_json::Value;
use std::ffi::OsStr;
use std::process::Output;

fn run_cli(args: &[&OsStr]) -> Output {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"));
    command.args(args);
    let output = command.output();
    assert!(
        output.is_ok(),
        "failed to execute velvet-ballistics: {output:?}"
    );
    output.unwrap_or_else(|_| std::process::abort())
}

fn parse_json(bytes: &[u8], channel: &str) -> Value {
    let parsed = serde_json::from_slice::<Value>(bytes);
    assert!(
        parsed.is_ok(),
        "{channel} must contain valid JSON; bytes={}",
        String::from_utf8_lossy(bytes)
    );
    parsed.unwrap_or(Value::Null)
}

#[test]
fn agent_context_output_is_deterministic() {
    let first = run_cli(&[OsStr::new("agent-context")]);
    let second = run_cli(&[OsStr::new("agent-context")]);

    assert_eq!(first.status.code(), Some(0), "agent-context must exit 0");
    assert_eq!(second.status.code(), Some(0), "agent-context must exit 0");

    let first_stdout = String::from_utf8_lossy(&first.stdout);
    let second_stdout = String::from_utf8_lossy(&second.stdout);

    assert_eq!(
        first_stdout, second_stdout,
        "agent-context output must be deterministic across runs"
    );
}

#[test]
fn agent_context_matches_snapshot() {
    let output = run_cli(&[OsStr::new("agent-context")]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "agent-context must exit 0; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let actual = parse_json(&output.stdout, "stdout");

    let snapshot_bytes = include_bytes!("snapshots/agent_context.json");
    let snapshot = parse_json(snapshot_bytes, "snapshot");

    assert_eq!(
        actual, snapshot,
        "agent-context output must match stored snapshot"
    );
}

#[test]
fn agent_context_has_required_top_level_fields() {
    let output = run_cli(&[OsStr::new("agent-context")]);
    let actual = parse_json(&output.stdout, "stdout");

    assert!(actual.get("schema_version").is_some());
    assert!(actual.get("kind").is_some());
    assert!(actual.get("cli").is_some());
    assert!(actual.get("version").is_some());
    assert!(actual.get("active_gates").is_some());
    assert!(actual.get("known_blockers").is_some());
}

#[test]
fn agent_context_has_agent_context_command() {
    let output = run_cli(&[OsStr::new("agent-context")]);
    let actual = parse_json(&output.stdout, "stdout");

    let commands = actual
        .get("commands")
        .and_then(Value::as_object)
        .expect("commands must be an object");
    assert!(
        commands.contains_key("agent-context"),
        "commands must include agent-context"
    );
}

#[test]
fn agent_context_stderr_is_empty_on_success() {
    let output = run_cli(&[OsStr::new("agent-context")]);

    assert_eq!(output.status.code(), Some(0), "agent-context must exit 0");
    assert!(
        output.stderr.is_empty(),
        "agent-context success must not emit on stderr"
    );
}
