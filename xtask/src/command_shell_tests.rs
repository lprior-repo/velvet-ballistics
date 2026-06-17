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

use serde_json::Value;

#[test]
fn required_command_stdout_renders_parseable_json_line_with_exact_fields() -> anyhow::Result<()> {
    let status = xtask::placeholder_status(
        xtask::CommandFamily::AiContext,
        xtask::DeferredReason::NotImplementedInThisBead,
    )?;
    let rendered = xtask::render_structured_status(&status, xtask::OutputFormat::JsonLines)?;
    let parsed: Value = serde_json::from_str(&rendered)?;
    assert_eq!(parsed["command"], Value::String("ai-context".to_string()));
    assert_eq!(parsed["status"], Value::String("deferred".to_string()));
    assert_eq!(
        parsed["next_steps"],
        Value::Array(vec![Value::String(
            "open follow-up bead for ai-context engine integration".to_string()
        )])
    );
    Ok(())
}

#[test]
fn parser_returns_exact_command_error_variants() {
    assert_eq!(
        xtask::parse_xtask_command(["xtask".into()]),
        Err(xtask::XtaskCommandError::MissingRequiredInput {
            command: "xtask".to_string(),
            input: "command".to_string()
        })
    );
    assert_eq!(
        xtask::parse_xtask_command(["xtask".into(), "ai-context".into(), "--bead".into()]),
        Err(xtask::XtaskCommandError::MissingRequiredInput {
            command: "ai-context".to_string(),
            input: "bead".to_string()
        })
    );
    assert_eq!(
        xtask::parse_xtask_command([
            "xtask".into(),
            "ai-context".into(),
            "--bead".into(),
            "".into()
        ]),
        Err(xtask::XtaskCommandError::InvalidInput {
            command: "ai-context".to_string(),
            input: "bead".to_string(),
            reason: "bead id must not be empty".to_string()
        })
    );
    assert_eq!(
        xtask::parse_xtask_command(["xtask".into(), "ai-context".into(), "--format".into()]),
        Err(xtask::XtaskCommandError::MissingRequiredInput {
            command: "ai-context".to_string(),
            input: "format".to_string()
        })
    );
    assert_eq!(
        xtask::parse_xtask_command([
            "xtask".into(),
            "ai-context".into(),
            "--format".into(),
            "yaml".into()
        ]),
        Err(xtask::XtaskCommandError::InvalidInput {
            command: "ai-context".to_string(),
            input: "format".to_string(),
            reason: "unsupported output format: yaml".to_string()
        })
    );
}

#[test]
fn renderer_and_router_return_exact_command_error_variants() {
    let status = xtask::StructuredStatus {
        command: "ai-context".to_string(),
        status: "deferred".to_string(),
        message: String::new(),
        next_steps: Vec::new(),
    };
    assert_eq!(
        xtask::render_structured_status(&status, xtask::OutputFormat::JsonLines),
        Err(xtask::XtaskCommandError::OutputRenderFailed {
            command: "ai-context".to_string(),
            reason: "structured status fields must be non-empty".to_string()
        })
    );
    let env = xtask::XtaskEnvironment {
        workspace_root: std::path::PathBuf::from("."),
        bead_id: None,
        output_format: xtask::OutputFormat::JsonLines,
        unavailable_families: vec![xtask::CommandFamily::AiContext],
    };
    assert_eq!(
        xtask::route_command(
            xtask::XtaskCommand::Required(xtask::CommandFamily::AiContext),
            &env
        ),
        Err(xtask::XtaskCommandError::Unavailable {
            command: "ai-context".to_string(),
            reason: "ai-context automation is not implemented in bead vb-kkvb".to_string()
        })
    );
}
