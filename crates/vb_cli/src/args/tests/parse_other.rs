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
    unused_variables,
)]

use super::*;

#[test]
fn parse_answer_rejects_invalid_slot_with_exact_variant() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "answer",
        "run-1",
        "--slot",
        "not-a-slot",
        "--value",
        "value.bin",
        "--db",
        "test-db",
    ]));

    assert!(
        matches!(parsed, Err(ParseError::InvalidSlot(ref s)) if s == "not-a-slot"),
        "expected InvalidSlot(not-a-slot), got {parsed:?}"
    );
}

#[test]
fn parse_inspect_includes_output_format() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "inspect",
        "42",
        "--db",
        "test-db",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Inspect { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Inspect {
        run_id, db, output, ..
    }) = parsed
    {
        assert_eq!(run_id, "42");
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_help_command() {
    let parsed = parse_args(&args(&["velvet-ballistics", "help"]));
    assert!(matches!(parsed, Ok(Command::Help)));
}

#[test]
fn parse_version_command() {
    let parsed = parse_args(&args(&["velvet-ballistics", "--version"]));
    assert!(matches!(parsed, Ok(Command::Version)));
}

#[test]
fn parse_agent_context_command() {
    let parsed = parse_args(&args(&["velvet-ballistics", "agent-context"]));
    assert!(matches!(
        parsed,
        Ok(Command::AgentContext { deliver: None })
    ));
}

#[test]
fn parse_agent_context_deliver_target() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "agent-context",
        "--deliver",
        "file:/tmp/out.jsonl",
    ]));
    assert!(
        matches!(parsed, Ok(Command::AgentContext { deliver: Some(ref target) }) if target == "file:/tmp/out.jsonl")
    );
}

#[test]
fn parse_agent_context_accepts_webhook_deliver_target_shape() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "agent-context",
        "--deliver",
        "webhook:https://example.invalid/hook",
    ]));
    assert!(
        matches!(parsed, Ok(Command::AgentContext { deliver: Some(ref target) }) if target == "webhook:https://example.invalid/hook")
    );
}

#[test]
fn parse_agent_context_rejects_missing_deliver_target() {
    let parsed = parse_args(&args(&["velvet-ballistics", "agent-context", "--deliver"]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidAgentContextArgument(ref reason)) if reason == "--deliver requires stdout, file:<absolute-path>, or webhook:<url>")
    );
}

#[test]
fn parse_agent_context_rejects_unknown_flag() {
    let parsed = parse_args(&args(&["velvet-ballistics", "agent-context", "--bogus"]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidAgentContextArgument(ref reason)) if reason == "unknown flag --bogus")
    );
}

#[test]
fn parse_diff_requires_both_run_ids_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "diff",
        "1",
        "2",
        "--db",
        "test-db",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Diff { .. })),
        "unexpected: {parsed:?}"
    );
    if let Ok(Command::Diff {
        diff_mode: DiffMode::RunAgainst { run_a, run_b, db },
        output,
    }) = parsed
    {
        assert_eq!(run_a, "1".to_string());
        assert_eq!(run_b, "2".to_string());
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_diff_accepts_json_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "diff",
        "10",
        "20",
        "--db",
        "test-db",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Diff { .. })),
        "unexpected: {parsed:?}"
    );
    if let Ok(Command::Diff { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_diff_requires_db_flag() {
    let parsed = parse_args(&args(&["velvet-ballistics", "diff", "1", "2"]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("--db"))),
        "unexpected: {parsed:?}"
    );
}

#[test]
fn parse_diff_allows_workflow_against_workflow_without_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "diff",
        "current.yaml",
        "--against",
        "previous.yaml",
    ]));
    match parsed {
        Ok(Command::Diff {
            diff_mode: DiffMode::WorkflowAgainst { workflow, against },
            output,
        }) => {
            assert_eq!(workflow, PathBuf::from("current.yaml"));
            assert_eq!(against, PathBuf::from("previous.yaml"));
            assert_eq!(output, OutputFormat::Text);
        }
        other => panic!("expected workflow diff without db to parse, got {other:?}"),
    }
}

#[test]
fn parse_diff_rejects_workflow_against_with_db_hidden_mode() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "diff",
        "current.yaml",
        "--against",
        "123",
        "--db",
        "test-db",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidArgument(ref reason)) if reason == "diff accepts either workflow --against <old-workflow> without --db, or two run IDs plus --db"),
        "unexpected: {parsed:?}"
    );
}

#[test]
fn parse_doctor_without_db_is_stateless_text_mode() {
    let parsed = parse_args(&args(&["velvet-ballistics", "doctor"]));
    assert!(
        matches!(parsed, Ok(Command::Doctor { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Doctor { db, output }) = parsed {
        assert_eq!(db, None);
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_doctor_accepts_optional_db_and_yaml_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "doctor",
        "--db",
        "journal-db",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Doctor { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Doctor { db, output }) = parsed {
        assert_eq!(db, Some(PathBuf::from("journal-db")));
        assert_eq!(output, OutputFormat::Yaml);
    }
}
