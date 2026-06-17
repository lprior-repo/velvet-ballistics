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
    unused_variables,
)]

use std::ffi::OsString;
use std::path::PathBuf;

use xtask::{
    CommandFamily, CommandFamilySpec, DeferredReason, OutputFormat, StructuredStatus,
    WorkspaceManifest, XtaskCommand, XtaskCommandError, XtaskEnvironment,
    assert_runtime_dependency_boundary, parse_xtask_command, placeholder_status,
    render_structured_status, required_command_families, route_command, validate_command_registry,
};

const CONTRACT_NAMES: [&str; 20] = [
    "ai-context",
    "ai-plan",
    "ai-check",
    "ai-evidence",
    "invariants",
    "scans",
    "cert-check",
    "perf",
    "replay",
    "crash",
    "diff",
    "mutants",
    "loom",
    "kani",
    "fuzz",
    "prop",
    "repro",
    "test-plan",
    "review",
    "why-failed",
];

fn argv(tokens: &[&str]) -> Vec<OsString> {
    tokens.iter().map(OsString::from).collect()
}

fn deterministic_environment(unavailable_families: Vec<CommandFamily>) -> XtaskEnvironment {
    XtaskEnvironment {
        workspace_root: PathBuf::from("."),
        bead_id: Some("vb-kkvb".to_string()),
        output_format: OutputFormat::JsonLines,
        unavailable_families,
    }
}

fn required_family_from_name(name: &str) -> CommandFamily {
    match name {
        "ai-context" => CommandFamily::AiContext,
        "ai-plan" => CommandFamily::AiPlan,
        "ai-check" => CommandFamily::AiCheck,
        "ai-evidence" => CommandFamily::AiEvidence,
        "invariants" => CommandFamily::Invariants,
        "scans" => CommandFamily::Scans,
        "cert-check" => CommandFamily::CertCheck,
        "perf" => CommandFamily::Perf,
        "replay" => CommandFamily::Replay,
        "crash" => CommandFamily::Crash,
        "diff" => CommandFamily::Diff,
        "mutants" => CommandFamily::Mutants,
        "loom" => CommandFamily::Loom,
        "kani" => CommandFamily::Kani,
        "fuzz" => CommandFamily::Fuzz,
        "prop" => CommandFamily::Prop,
        "repro" => CommandFamily::Repro,
        "test-plan" => CommandFamily::TestPlan,
        "review" => CommandFamily::Review,
        _ => CommandFamily::WhyFailed,
    }
}

#[test]
fn required_registry_contains_each_contract_command_once_and_sorted() {
    let names: Vec<_> = required_command_families()
        .iter()
        .map(CommandFamilySpec::public_name)
        .collect();
    assert_eq!(names.len(), 20);
    for expected in CONTRACT_NAMES {
        assert_eq!(names.iter().filter(|name| **name == expected).count(), 1);
    }
}

#[test]
fn registry_validation_rejects_duplicate_and_schema_drift() {
    assert_eq!(
        validate_command_registry(required_command_families()).is_ok(),
        true
    );
    assert_eq!(
        validate_command_registry(&[
            CommandFamilySpec::new(
                "ai-context",
                &["command", "status", "message", "next_steps"]
            ),
            CommandFamilySpec::new(
                "ai-context",
                &["command", "status", "message", "next_steps"]
            ),
        ]),
        Err(XtaskCommandError::InternalInvariantViolation {
            invariant: "duplicate command family: ai-context".to_string(),
        })
    );
    assert_eq!(
        validate_command_registry(&[CommandFamilySpec::new(
            "ai-context",
            &["command", "status", "message"]
        )]),
        Err(XtaskCommandError::InternalInvariantViolation {
            invariant: "structured status schema drift: missing next_steps".to_string(),
        })
    );
}

#[test]
fn parser_rejects_unknown_and_invalid_required_inputs() {
    for command in ["unknown", "AiContext", "ai--context"] {
        assert_eq!(
            parse_xtask_command(argv(&["xtask", command])),
            Err(XtaskCommandError::UnknownCommand {
                command: command.to_string()
            })
        );
    }
    assert_eq!(
        parse_xtask_command(argv(&["xtask", "ai-context", "--bead"])),
        Err(XtaskCommandError::MissingRequiredInput {
            command: "ai-context".to_string(),
            input: "bead".to_string()
        })
    );
    assert_eq!(
        parse_xtask_command(argv(&["xtask", "ai-context", "--bead", ""])),
        Err(XtaskCommandError::InvalidInput {
            command: "ai-context".to_string(),
            input: "bead".to_string(),
            reason: "bead id must not be empty".to_string()
        })
    );
    assert_eq!(
        parse_xtask_command(argv(&["xtask", "test-plan", "--format", "xml"])),
        Err(XtaskCommandError::InvalidInput {
            command: "test-plan".to_string(),
            input: "format".to_string(),
            reason: "unsupported output format: xml".to_string()
        })
    );
}

#[test]
fn placeholder_route_and_render_are_deterministic() {
    for name in ["perf", "fuzz", "ai-context"] {
        let family = required_family_from_name(name);
        let expected = StructuredStatus {
            command: name.to_string(),
            status: "deferred".to_string(),
            message: format!("{name} automation deferred: implementation is outside bead vb-kkvb"),
            next_steps: vec![format!("open follow-up bead for {name} engine integration")],
        };
        assert_eq!(
            placeholder_status(family, DeferredReason::NotImplementedInThisBead),
            Ok(expected.clone())
        );
        assert_eq!(
            route_command(
                XtaskCommand::Required(family),
                &deterministic_environment(Vec::new())
            ),
            Ok(expected)
        );
    }
    assert_eq!(
        route_command(
            XtaskCommand::Required(CommandFamily::Perf),
            &deterministic_environment(vec![CommandFamily::Perf])
        ),
        Err(XtaskCommandError::Unavailable {
            command: "perf".to_string(),
            reason: "perf automation is not implemented in bead vb-kkvb".to_string(),
        })
    );
}

#[test]
fn renderer_returns_json_or_exact_failures() {
    let status = StructuredStatus {
        command: "fuzz".to_string(),
        status: "deferred".to_string(),
        message: "fuzz automation deferred: implementation is outside bead vb-kkvb".to_string(),
        next_steps: vec!["open follow-up bead for fuzz engine integration".to_string()],
    };
    assert_eq!(
        render_structured_status(&status, OutputFormat::JsonLines)
            .unwrap_or_default()
            .contains("\"command\":\"fuzz\""),
        true
    );
    assert_eq!(
        render_structured_status(
            &StructuredStatus::with_renderer_failure_for_test(
                "fuzz",
                "deferred",
                "m",
                ["n"],
                "boom"
            ),
            OutputFormat::JsonLines
        ),
        Err(XtaskCommandError::OutputRenderFailed {
            command: "fuzz".to_string(),
            reason: "boom".to_string()
        })
    );
}

#[test]
fn dependency_boundary_rejects_runtime_shell_dependencies() {
    for (crate_name, dependency) in [("vb_core", "clap"), ("vb_runtime", "xtask")] {
        assert_eq!(
            assert_runtime_dependency_boundary(&WorkspaceManifest::from_edges([(
                crate_name, dependency
            )])),
            Err(XtaskCommandError::DependencyBoundaryViolation {
                crate_name: crate_name.to_string(),
                dependency: dependency.to_string()
            })
        );
    }
}
