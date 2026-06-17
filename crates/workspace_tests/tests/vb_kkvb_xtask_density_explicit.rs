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

use std::ffi::OsString;
use std::path::PathBuf;

use xtask::{
    CommandFamily, DeferredReason, OutputFormat, StructuredStatus, WorkspaceManifest, XtaskCommand,
    XtaskCommandError, XtaskEnvironment, assert_runtime_dependency_boundary, parse_xtask_command,
    placeholder_status, render_structured_status, route_command,
};

const FAMILIES: [(&str, CommandFamily); 20] = [
    ("ai-context", CommandFamily::AiContext),
    ("ai-plan", CommandFamily::AiPlan),
    ("ai-check", CommandFamily::AiCheck),
    ("ai-evidence", CommandFamily::AiEvidence),
    ("invariants", CommandFamily::Invariants),
    ("scans", CommandFamily::Scans),
    ("cert-check", CommandFamily::CertCheck),
    ("perf", CommandFamily::Perf),
    ("replay", CommandFamily::Replay),
    ("crash", CommandFamily::Crash),
    ("diff", CommandFamily::Diff),
    ("mutants", CommandFamily::Mutants),
    ("loom", CommandFamily::Loom),
    ("kani", CommandFamily::Kani),
    ("fuzz", CommandFamily::Fuzz),
    ("prop", CommandFamily::Prop),
    ("repro", CommandFamily::Repro),
    ("test-plan", CommandFamily::TestPlan),
    ("review", CommandFamily::Review),
    ("why-failed", CommandFamily::WhyFailed),
];

fn argv(tokens: &[&str]) -> Vec<OsString> {
    tokens.iter().map(OsString::from).collect()
}

fn env_available() -> XtaskEnvironment {
    XtaskEnvironment {
        workspace_root: PathBuf::from("."),
        bead_id: Some("vb-kkvb".to_string()),
        output_format: OutputFormat::JsonLines,
        unavailable_families: Vec::new(),
    }
}

fn env_with_disabled(family: CommandFamily) -> XtaskEnvironment {
    XtaskEnvironment {
        unavailable_families: vec![family],
        ..env_available()
    }
}

#[test]
fn all_command_families_have_exact_public_names_and_parse_forms() {
    for (name, family) in FAMILIES {
        assert_eq!(family.public_name(), name);
        if name == "loom" {
            // loom is classified as Legacy, not Required
            assert_eq!(
                parse_xtask_command(argv(&["xtask", name])),
                Ok(XtaskCommand::Legacy(name))
            );
        } else {
            assert_eq!(
                parse_xtask_command(argv(&["xtask", name])),
                Ok(XtaskCommand::Required(family))
            );
            assert_eq!(
                parse_xtask_command(argv(&["xtask", name, "--bead", "vb-kkvb"])),
                Ok(XtaskCommand::Required(family))
            );
            assert_eq!(
                parse_xtask_command(argv(&["xtask", name, "--format", "jsonl"])),
                Ok(XtaskCommand::Required(family))
            );
        }
    }
}

#[test]
fn all_command_families_reject_invalid_required_options() {
    for (name, _) in FAMILIES {
        if name == "loom" {
            // loom is Legacy, not Required - it doesn't validate --bead/--format options
            continue;
        }
        assert_eq!(
            parse_xtask_command(argv(&["xtask", name, "--bead"])),
            Err(XtaskCommandError::MissingRequiredInput {
                command: name.to_string(),
                input: "bead".to_string(),
            })
        );
        assert_eq!(
            parse_xtask_command(argv(&["xtask", name, "--bead", ""])),
            Err(XtaskCommandError::InvalidInput {
                command: name.to_string(),
                input: "bead".to_string(),
                reason: "bead id must not be empty".to_string(),
            })
        );
        assert_eq!(
            parse_xtask_command(argv(&["xtask", name, "--format"])),
            Err(XtaskCommandError::MissingRequiredInput {
                command: name.to_string(),
                input: "format".to_string(),
            })
        );
        assert_eq!(
            parse_xtask_command(argv(&["xtask", name, "--format", "xml"])),
            Err(XtaskCommandError::InvalidInput {
                command: name.to_string(),
                input: "format".to_string(),
                reason: "unsupported output format: xml".to_string(),
            })
        );
    }
}

#[test]
fn all_command_families_route_and_render_deferred_json() {
    for (name, family) in FAMILIES {
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
            route_command(XtaskCommand::Required(family), &env_available()),
            Ok(expected.clone())
        );
        assert_eq!(
            route_command(XtaskCommand::Required(family), &env_with_disabled(family)),
            Err(XtaskCommandError::Unavailable {
                command: name.to_string(),
                reason: format!("{name} automation is not implemented in bead vb-kkvb"),
            })
        );
        assert_eq!(
            render_structured_status(&expected, OutputFormat::JsonLines),
            Ok(format!(
                "{{\"command\":\"{name}\",\"status\":\"deferred\",\"message\":\"{name} automation deferred: implementation is outside bead vb-kkvb\",\"next_steps\":[\"open follow-up bead for {name} engine integration\"]}}\n"
            ))
        );
    }
}

#[test]
fn top_level_and_legacy_commands_classify_exactly() {
    assert_eq!(
        parse_xtask_command(argv(&["xtask", "--help"])),
        Ok(XtaskCommand::Help)
    );
    assert_eq!(
        parse_xtask_command(argv(&["xtask", "-h"])),
        Ok(XtaskCommand::Help)
    );
    assert_eq!(
        parse_xtask_command(argv(&["xtask", "--version"])),
        Ok(XtaskCommand::Version)
    );
    assert_eq!(
        parse_xtask_command(argv(&["xtask", "-V"])),
        Ok(XtaskCommand::Version)
    );
    for name in [
        "ui-snapshot",
        "ui-tokens",
        "ui-overlap-check",
        "ai-fast",
        "ai-deep",
        "ai-release",
    ] {
        assert_eq!(
            parse_xtask_command(argv(&["xtask", name])),
            Ok(XtaskCommand::Legacy(name))
        );
    }
}

#[test]
fn parser_rejects_malformed_commands_without_normalization_shortcuts() {
    for command in [
        "", " ", "AI-plan", "-ai-plan", "ai-plan-", "ai--plan", "ai_plan", "åi-plan",
    ] {
        assert_eq!(
            parse_xtask_command(argv(&["xtask", command])),
            Err(XtaskCommandError::UnknownCommand {
                command: command.to_string()
            })
        );
    }
    assert_eq!(
        parse_xtask_command(argv(&["xtask"])),
        Err(XtaskCommandError::MissingRequiredInput {
            command: "xtask".to_string(),
            input: "command".to_string(),
        })
    );
}

#[test]
fn renderer_preserves_json_escaping_and_rejects_incomplete_status() {
    let quoted = StructuredStatus {
        command: "ai-context\"quoted".to_string(),
        status: "deferred".to_string(),
        message: "line\nnext".to_string(),
        next_steps: vec!["next".to_string()],
    };
    let rendered = render_structured_status(&quoted, OutputFormat::JsonLines).unwrap_or_default();
    assert_eq!(rendered.contains("ai-context\\\"quoted"), true);
    assert_eq!(rendered.contains("line\\nnext"), true);
    assert_eq!(
        render_structured_status(
            &StructuredStatus {
                command: "ai-context".into(),
                status: "deferred".into(),
                message: String::new(),
                next_steps: vec!["next".into()]
            },
            OutputFormat::JsonLines
        ),
        Err(XtaskCommandError::OutputRenderFailed {
            command: "ai-context".to_string(),
            reason: "structured status fields must be non-empty".to_string(),
        })
    );
}

#[test]
fn runtime_dependency_boundary_accepts_and_rejects_declared_edges() {
    for (crate_name, dep) in [
        ("vb_core", "xtask"),
        ("vb_storage", "toml"),
        ("vb_ipc", "reqwest"),
        ("vb_runtime", "serde_yaml"),
    ] {
        assert_eq!(
            assert_runtime_dependency_boundary(&WorkspaceManifest::from_edges([(crate_name, dep)])),
            Err(XtaskCommandError::DependencyBoundaryViolation {
                crate_name: crate_name.to_string(),
                dependency: dep.to_string(),
            })
        );
    }
    for (crate_name, dep) in [
        ("vb_ui", "clap"),
        ("vb_doc", "toml"),
        ("vb_runtime", "fjall"),
        ("vb_storage", "bytes"),
    ] {
        assert_eq!(
            assert_runtime_dependency_boundary(&WorkspaceManifest::from_edges([(crate_name, dep)])),
            Ok(())
        );
    }
}
