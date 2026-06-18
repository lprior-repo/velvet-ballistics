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
    unused_variables
)]

use crate::agent_context::build;
use crate::args::run_ops::CANCEL_REASON_MAX_CHARS;
use serde_json::Value;

#[test]
fn build_has_versioned_schema_and_yaml_emit_flag() {
    let context = build("0.1.0");

    assert_eq!(
        context
            .get("schema_version")
            .and_then(serde_json::Value::as_str),
        Some("1")
    );
    assert_eq!(
        context
            .get("agent_contract")
            .and_then(|contract| contract.get("structured_output_flag"))
            .and_then(serde_json::Value::as_str),
        Some("--emit yaml")
    );
}

#[test]
fn context_exposes_agent_context_command() {
    let context = build("0.1.0");
    let command = context
        .get("commands")
        .and_then(|commands| commands.get("agent-context"));

    assert!(command.is_some());
}

#[test]
fn build_has_agent_context_kind() {
    let context = build("0.1.0");
    assert_eq!(
        context.get("kind").and_then(Value::as_str),
        Some("AgentContext")
    );
}

#[test]
fn build_has_cli_name() {
    let context = build("0.1.0");
    assert_eq!(
        context.get("cli").and_then(Value::as_str),
        Some("velvet-ballistics")
    );
}

#[test]
fn build_has_binary_aliases() {
    let context = build("0.1.0");
    let aliases = context
        .get("binary_aliases")
        .and_then(Value::as_array)
        .expect("binary_aliases must be an array");
    assert!(
        aliases
            .iter()
            .any(|v| v.as_str() == Some("velvet-ballistics"))
    );
    assert!(!aliases.iter().any(|v| v.as_str() == Some("vb")));
}

#[test]
fn build_has_language_version() {
    let context = build("0.1.0");
    assert_eq!(
        context.get("language_version").and_then(Value::as_str),
        Some("velvet-ballistics/v1")
    );
}

#[test]
fn build_version_matches_input() {
    let context = build("1.2.3");
    assert_eq!(
        context.get("version").and_then(Value::as_str),
        Some("1.2.3")
    );
}

#[test]
fn agent_contract_has_required_fields() {
    let context = build("0.1.0");
    let contract = context
        .get("agent_contract")
        .and_then(Value::as_object)
        .expect("agent_contract must be an object");

    assert!(contract.contains_key("non_interactive_by_default"));
    assert!(contract.contains_key("prompt_bypass_flag"));
    assert!(contract.contains_key("structured_output_flag"));
    assert!(contract.contains_key("machine_output_flag"));
    assert!(!contract.contains_key("streaming_output_flag"));
    assert!(contract.contains_key("stdout"));
    assert!(contract.contains_key("stderr"));
    assert!(contract.contains_key("ansi_when_non_tty"));
    assert!(contract.contains_key("bounded_output_required"));
    assert!(contract.contains_key("destructive_operations_require_explicit_flag"));
    assert!(contract.contains_key("mutation_responses_return_identifiers"));
}

#[test]
fn vocabulary_policy_has_canonical_flags() {
    let context = build("0.1.0");
    let policy = context
        .get("vocabulary_policy")
        .and_then(Value::as_object)
        .expect("vocabulary_policy must be an object");

    assert!(policy.contains_key("canonical_output_flag"));
    assert!(policy.contains_key("canonical_output_values"));
    assert!(policy.contains_key("canonical_destructive_bypass_flag"));
    assert!(policy.contains_key("canonical_resource_verbs"));
    assert!(policy.contains_key("banned_verbs"));
    assert!(policy.contains_key("banned_flags"));
}

#[test]
fn exit_codes_covers_zero_through_eight() {
    let context = build("0.1.0");
    let exit_codes = context
        .get("exit_codes")
        .and_then(Value::as_object)
        .expect("exit_codes must be an object");

    for code in 0..=8 {
        let key = format!("{}", code);
        assert!(
            exit_codes.contains_key(&key),
            "exit code {} must be defined",
            code
        );
    }
}

#[test]
fn enums_has_emit_compile_emit_durability_verify_profile() {
    let context = build("0.1.0");
    let enums = context
        .get("enums")
        .and_then(Value::as_object)
        .expect("enums must be an object");

    assert!(enums.contains_key("emit"));
    assert!(enums.contains_key("compile_emit"));
    assert!(enums.contains_key("durability"));
    assert!(enums.contains_key("verify_profile"));
}

#[test]
fn commands_has_validate() {
    let context = build("0.1.0");
    let commands = context
        .get("commands")
        .and_then(Value::as_object)
        .expect("commands must be an object");

    assert!(commands.contains_key("validate"));
}

#[test]
fn commands_has_compile() {
    let context = build("0.1.0");
    let commands = context
        .get("commands")
        .and_then(Value::as_object)
        .expect("commands must be an object");

    assert!(commands.contains_key("compile"));
}

#[test]
fn commands_has_run() {
    let context = build("0.1.0");
    let commands = context
        .get("commands")
        .and_then(Value::as_object)
        .expect("commands must be an object");

    assert!(commands.contains_key("run"));
}

#[test]
fn planned_agent_primitives_has_async_wait_flag() {
    let context = build("0.1.0");
    let primitives = context
        .get("planned_agent_primitives")
        .and_then(Value::as_object)
        .expect("planned_agent_primitives must be an object");

    assert!(primitives.contains_key("async_wait_flag"));
    assert!(primitives.contains_key("jobs_commands"));
    assert!(primitives.contains_key("profile_commands"));
    assert!(primitives.contains_key("delivery_flag"));
    assert!(primitives.contains_key("feedback_command"));
}

#[test]
fn active_gates_has_validation() {
    let context = build("0.1.0");
    let gates = context
        .get("active_gates")
        .and_then(Value::as_object)
        .expect("active_gates must be an object");

    assert!(gates.contains_key("validation"));
    assert!(gates.contains_key("verification"));
    assert!(gates.contains_key("compilation"));
    assert!(gates.contains_key("admission"));
    assert!(gates.contains_key("durability"));
}

#[test]
fn known_blockers_has_policy_category() {
    let context = build("0.1.0");
    let blockers = context
        .get("known_blockers")
        .and_then(Value::as_object)
        .expect("known_blockers must be an object");

    assert!(blockers.contains_key("policy"));
    assert!(blockers.contains_key("resource"));
    assert!(blockers.contains_key("capability"));
}

#[test]
fn known_blockers_policy_covers_all_exit_codes() {
    let context = build("0.1.0");
    let blockers = context
        .get("known_blockers")
        .and_then(Value::as_object)
        .expect("known_blockers must be an object");
    let policy = blockers
        .get("policy")
        .and_then(Value::as_array)
        .expect("policy must be an array");

    assert_eq!(policy.len(), 8);
}

#[test]
fn known_blockers_resource_has_three_entries() {
    let context = build("0.1.0");
    let blockers = context
        .get("known_blockers")
        .and_then(Value::as_object)
        .expect("known_blockers must be an object");
    let resource = blockers
        .get("resource")
        .and_then(Value::as_array)
        .expect("resource must be an array");

    assert_eq!(resource.len(), 3);
}

#[test]
fn known_blockers_capability_has_three_entries() {
    let context = build("0.1.0");
    let blockers = context
        .get("known_blockers")
        .and_then(Value::as_object)
        .expect("known_blockers must be an object");
    let capability = blockers
        .get("capability")
        .and_then(Value::as_array)
        .expect("capability must be an array");

    assert_eq!(capability.len(), 3);
}

#[test]
fn build_is_deterministic() {
    let first = build("0.1.0");
    let second = build("0.1.0");
    assert_eq!(first, second);
}

#[test]
fn build_with_empty_version() {
    let context = build("");
    assert_eq!(context.get("version").and_then(Value::as_str), Some(""));
}

#[test]
fn build_with_arbitrary_version() {
    let context = build("abc-123.def_456");
    assert_eq!(
        context.get("version").and_then(Value::as_str),
        Some("abc-123.def_456")
    );
}

// ── Extended tests ──────────────────────────────────────────────

#[test]
fn build_has_all_expected_top_level_keys() {
    let context = build("0.1.0");
    let expected = [
        "active_gates",
        "agent_contract",
        "binary_aliases",
        "cli",
        "commands",
        "enums",
        "exit_codes",
        "kind",
        "known_blockers",
        "language_version",
        "planned_agent_primitives",
        "schema_version",
        "version",
        "vocabulary_policy",
    ];
    let obj = context.as_object().expect("top-level must be an object");
    for key in &expected {
        assert!(
            obj.contains_key(*key),
            "top-level key '{}' must be present",
            key
        );
    }
    assert_eq!(obj.len(), expected.len(), "no unexpected top-level keys");
}

#[test]
fn build_serializes_to_valid_json() {
    let context = build("0.1.0");
    let encoded = serde_json::to_string(&context).expect("must serialize to JSON");
    let parsed: Value =
        serde_json::from_str(&encoded).expect("serialized output must be valid JSON");
    assert_eq!(parsed, context);
}

#[test]
fn build_output_is_not_null() {
    let context = build("0.1.0");
    assert!(!context.is_null());
}

#[test]
fn build_with_semver_like_version() {
    let context = build("2.0.0-rc.1+build.2024");
    assert_eq!(
        context.get("version").and_then(Value::as_str),
        Some("2.0.0-rc.1+build.2024")
    );
}

#[test]
fn build_with_unicode_version() {
    let context = build("vérsîon-テスト");
    assert_eq!(
        context.get("version").and_then(Value::as_str),
        Some("vérsîon-テスト")
    );
}

#[test]
fn active_gates_validation_is_required_and_has_gate() {
    let context = build("0.1.0");
    let gates = context
        .get("active_gates")
        .and_then(Value::as_object)
        .expect("active_gates must be an object");

    let validation = gates
        .get("validation")
        .and_then(Value::as_object)
        .expect("validation must be an object");
    assert_eq!(
        validation.get("required").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        validation.get("gate").and_then(Value::as_str),
        Some("vb_validate")
    );
}

#[test]
fn active_gates_durability_is_not_required() {
    let context = build("0.1.0");
    let gates = context
        .get("active_gates")
        .and_then(Value::as_object)
        .expect("active_gates must be an object");

    let durability = gates
        .get("durability")
        .and_then(Value::as_object)
        .expect("durability must be an object");
    assert_eq!(
        durability.get("required").and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn active_gates_all_required_gates_are_true() {
    let context = build("0.1.0");
    let gates = context
        .get("active_gates")
        .and_then(Value::as_object)
        .expect("active_gates must be an object");

    let required_gates = ["validation", "verification", "compilation", "admission"];
    for gate_name in &required_gates {
        let gate = gates
            .get(*gate_name)
            .and_then(Value::as_object)
            .unwrap_or_else(|| panic!("gate '{}' must be an object", gate_name));
        assert_eq!(
            gate.get("required").and_then(Value::as_bool),
            Some(true),
            "gate '{}' must be required",
            gate_name
        );
    }
}

#[test]
fn known_blockers_policy_entries_have_category_and_exit_code() {
    let context = build("0.1.0");
    let policy = context
        .pointer("/known_blockers/policy")
        .and_then(Value::as_array)
        .expect("policy must be an array");

    let expected: [(i64, &str); 8] = [
        (1, "validation_failed"),
        (2, "verification_failed"),
        (3, "compile_failed"),
        (4, "runtime_failed"),
        (5, "storage_error"),
        (6, "ipc_error"),
        (7, "action_policy_error"),
        (8, "replay_divergence"),
    ];

    for (i, entry) in policy.iter().enumerate() {
        let obj = entry.as_object().expect("policy entry must be an object");
        assert_eq!(
            obj.get("category").and_then(Value::as_str),
            Some(expected[i].1)
        );
        assert_eq!(
            obj.get("exit_code").and_then(Value::as_i64),
            Some(expected[i].0)
        );
    }
}

#[test]
fn known_blockers_resource_entries_have_category() {
    let context = build("0.1.0");
    let resource = context
        .pointer("/known_blockers/resource")
        .and_then(Value::as_array)
        .expect("resource must be an array");

    let expected = [
        "slot_count_exceeded",
        "input_index_out_of_range",
        "journal_capacity",
    ];
    for (i, entry) in resource.iter().enumerate() {
        assert_eq!(
            entry.get("category").and_then(Value::as_str),
            Some(expected[i])
        );
    }
}

#[test]
fn known_blockers_capability_entries_have_category() {
    let context = build("0.1.0");
    let capability = context
        .pointer("/known_blockers/capability")
        .and_then(Value::as_array)
        .expect("capability must be an array");

    let expected = [
        "unregistered_action",
        "missing_capability",
        "capability_mismatch",
    ];
    for (i, entry) in capability.iter().enumerate() {
        assert_eq!(
            entry.get("category").and_then(Value::as_str),
            Some(expected[i])
        );
    }
}

#[test]
fn exit_codes_have_correct_description_mapping() {
    let context = build("0.1.0");
    let exit_codes = context
        .get("exit_codes")
        .and_then(Value::as_object)
        .expect("exit_codes must be an object");

    let expected: [(i64, &str); 9] = [
        (0, "success"),
        (1, "validation failed"),
        (2, "verification failed"),
        (3, "compile failed"),
        (4, "runtime failed"),
        (5, "storage error"),
        (6, "ipc error"),
        (7, "action policy error"),
        (8, "replay divergence"),
    ];

    for (code, desc) in &expected {
        let key = code.to_string();
        assert_eq!(exit_codes.get(&key).and_then(Value::as_str), Some(*desc));
    }
}

#[test]
fn enums_emit_has_correct_values() {
    let context = build("0.1.0");
    let emit = context
        .pointer("/enums/emit")
        .and_then(Value::as_array)
        .expect("emit must be an array");
    let values: Vec<&str> = emit.iter().filter_map(Value::as_str).collect();
    assert_eq!(values, vec!["text", "yaml", "postcard"]);
}

#[test]
fn enums_compile_emit_has_correct_values() {
    let context = build("0.1.0");
    let emit = context
        .pointer("/enums/compile_emit")
        .and_then(Value::as_array)
        .expect("compile_emit must be an array");
    let values: Vec<&str> = emit.iter().filter_map(Value::as_str).collect();
    assert_eq!(values, vec!["ir", "yaml", "postcard"]);
}

#[test]
fn enums_durability_has_correct_values() {
    let context = build("0.1.0");
    let durability = context
        .pointer("/enums/durability")
        .and_then(Value::as_array)
        .expect("durability must be an array");
    let values: Vec<&str> = durability.iter().filter_map(Value::as_str).collect();
    assert_eq!(values, vec!["strict", "journaled", "none"]);
}

#[test]
fn enums_verify_profile_has_correct_values() {
    let context = build("0.1.0");
    let verify = context
        .pointer("/enums/verify_profile")
        .and_then(Value::as_array)
        .expect("verify_profile must be an array");
    let values: Vec<&str> = verify.iter().filter_map(Value::as_str).collect();
    assert_eq!(values, vec!["quick", "standard", "full"]);
}

#[test]
fn commands_includes_all_expected_names() {
    let context = build("0.1.0");
    let commands = context
        .get("commands")
        .and_then(Value::as_object)
        .expect("commands must be an object");

    let expected = [
        "action inspect",
        "action list",
        "agent-context",
        "ai-context",
        "answer",
        "bench-run",
        "cancel",
        "compile",
        "diff",
        "doctor",
        "events",
        "explain",
        "graph",
        "help",
        "incident",
        "inspect",
        "ipc-serve",
        "replay",
        "resume",
        "retry",
        "run",
        "run-compiled",
        "simulate",
        "status",
        "submit",
        "system status",
        "trace",
        "validate",
        "version",
        "verify",
    ];
    for name in &expected {
        assert!(
            commands.contains_key(*name),
            "command '{}' must be present",
            name
        );
    }
}

#[test]
fn command_count_is_stable() {
    let context = build("0.1.0");
    let commands = context
        .get("commands")
        .and_then(Value::as_object)
        .expect("commands must be an object");
    assert_eq!(commands.len(), 30);
}

#[test]
fn command_agent_context_has_summary_outputs_and_deliver_flag() {
    let context = build("0.1.0");
    let cmd = context
        .pointer("/commands/agent-context")
        .and_then(Value::as_object)
        .expect("agent-context command must be an object");
    let flags = cmd
        .get("flags")
        .and_then(Value::as_object)
        .expect("agent-context flags must be an object");
    let deliver = flags
        .get("--deliver")
        .and_then(Value::as_object)
        .expect("agent-context --deliver must be an object");

    assert_eq!(
        cmd.get("summary").and_then(Value::as_str),
        Some("Emit this versioned machine-readable CLI schema.")
    );
    let outputs = cmd
        .get("outputs")
        .and_then(Value::as_array)
        .expect("outputs must be an array");
    assert_eq!(
        outputs.iter().filter_map(Value::as_str).collect::<Vec<_>>(),
        vec!["json"]
    );
    assert_eq!(deliver.get("type").and_then(Value::as_str), Some("string"));
    assert_eq!(
        deliver
            .get("accepted_forms")
            .and_then(Value::as_array)
            .expect("agent-context accepted_forms must be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["stdout", "file:<absolute-path>", "webhook:<url>"]
    );
    assert_eq!(
        deliver
            .get("currently_refused_forms")
            .and_then(Value::as_array)
            .expect("agent-context currently_refused_forms must be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["webhook:<url>"]
    );
}

#[test]
fn command_compile_has_emit_flag_required() {
    let context = build("0.1.0");
    let flags = context
        .pointer("/commands/compile/flags")
        .and_then(Value::as_object)
        .expect("flags must be an object");
    let emit = flags
        .get("--emit")
        .and_then(Value::as_object)
        .expect("--emit must be an object");
    assert_eq!(emit.get("required").and_then(Value::as_bool), Some(true));
    assert_eq!(emit.get("type").and_then(Value::as_str), Some("enum"));
}

#[test]
fn command_events_exposes_status_and_limit_filters() {
    let context = build("0.1.0");
    let flags = context
        .pointer("/commands/events/flags")
        .and_then(Value::as_object)
        .expect("events flags must be an object");

    assert_eq!(
        flags
            .get("--status")
            .and_then(Value::as_object)
            .expect("events --status must be an object")
            .get("values")
            .and_then(Value::as_array)
            .expect("events --status values must be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec![
            "pending",
            "active",
            "waiting_answer",
            "cancelled",
            "completed",
            "failed"
        ]
    );
    assert_eq!(
        flags
            .get("--limit")
            .and_then(Value::as_object)
            .and_then(|flag| flag.get("type"))
            .and_then(Value::as_str),
        Some("i64")
    );
}

#[test]
fn command_retry_exposes_optional_step_filter() {
    let context = build("0.1.0");
    let flags = context
        .pointer("/commands/retry/flags")
        .and_then(Value::as_object)
        .expect("retry flags must be an object");

    assert_eq!(
        flags
            .get("--step")
            .and_then(Value::as_object)
            .and_then(|flag| flag.get("type"))
            .and_then(Value::as_str),
        Some("u16")
    );
    assert_eq!(
        flags
            .get("--step")
            .and_then(Value::as_object)
            .and_then(|flag| flag.get("required"))
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn command_trace_exposes_all_parser_filters() {
    let context = build("0.1.0");
    let flags = context
        .pointer("/commands/trace/flags")
        .and_then(Value::as_object)
        .expect("trace flags must be an object");

    for (flag_name, expected_type) in [
        ("--step", "u16"),
        ("--action", "u16"),
        ("--since-seq", "u64"),
        ("--until-seq", "u64"),
        ("--limit", "usize"),
    ] {
        assert_eq!(
            flags
                .get(flag_name)
                .and_then(Value::as_object)
                .and_then(|flag| flag.get("type"))
                .and_then(Value::as_str),
            Some(expected_type),
            "trace flag {flag_name} must match parser type"
        );
    }
    assert_eq!(
        flags
            .get("--status")
            .and_then(Value::as_object)
            .expect("trace --status must be an object")
            .get("values")
            .and_then(Value::as_array)
            .expect("trace --status values must be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec![
            "pending",
            "active",
            "waiting_answer",
            "cancelled",
            "completed",
            "failed"
        ]
    );
}

#[test]
fn command_doctor_db_flag_is_optional() {
    let context = build("0.1.0");
    let db = context
        .pointer("/commands/doctor/flags/--db")
        .and_then(Value::as_object)
        .expect("doctor --db must be an object");

    assert_eq!(db.get("type").and_then(Value::as_str), Some("path"));
    assert_eq!(db.get("required").and_then(Value::as_bool), Some(false));
}

#[test]
fn command_run_has_input_bin_flag_required() {
    let context = build("0.1.0");
    let flags = context
        .pointer("/commands/run/flags")
        .and_then(Value::as_object)
        .expect("flags must be an object");
    let input_bin = flags
        .get("--input-bin")
        .and_then(Value::as_object)
        .expect("--input-bin must be an object");
    assert_eq!(
        input_bin.get("required").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(input_bin.get("type").and_then(Value::as_str), Some("path"));
}

#[test]
fn command_ipc_serve_has_socket_and_db_flags_required() {
    let context = build("0.1.0");
    let flags = context
        .pointer("/commands/ipc-serve/flags")
        .and_then(Value::as_object)
        .expect("flags must be an object");
    for flag_name in &["--socket", "--db"] {
        let flag = flags
            .get(*flag_name)
            .and_then(Value::as_object)
            .unwrap_or_else(|| panic!("{} must be an object", flag_name));
        assert_eq!(flag.get("required").and_then(Value::as_bool), Some(true));
        assert_eq!(flag.get("type").and_then(Value::as_str), Some("path"));
    }
}

#[test]
fn command_replay_has_run_id_positional() {
    let context = build("0.1.0");
    let pos = context
        .pointer("/commands/replay/positionals")
        .and_then(Value::as_array)
        .expect("positionals must be an array");
    assert_eq!(
        pos.iter().filter_map(Value::as_str).collect::<Vec<_>>(),
        vec!["run_id"]
    );
}

#[test]
fn command_cancel_matches_cli_shape() {
    let context = build("0.1.0");
    let positionals = context
        .pointer("/commands/cancel/positionals")
        .and_then(Value::as_array)
        .expect("cancel positionals must be an array");
    let flags = context
        .pointer("/commands/cancel/flags")
        .and_then(Value::as_object)
        .expect("cancel flags must be an object");
    let db = flags
        .get("--db")
        .and_then(Value::as_object)
        .expect("cancel --db must be an object");
    let emit = flags
        .get("--emit")
        .and_then(Value::as_object)
        .expect("cancel --emit must be an object");
    let reason = flags
        .get("--reason")
        .and_then(Value::as_object)
        .expect("cancel --reason must be an object");

    assert_eq!(
        positionals
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["run_id"]
    );
    assert_eq!(db.get("type").and_then(Value::as_str), Some("path"));
    assert_eq!(db.get("required").and_then(Value::as_bool), Some(true));
    assert_eq!(reason.get("type").and_then(Value::as_str), Some("string"));
    assert_eq!(
        reason.get("length_unit").and_then(Value::as_str),
        Some("characters")
    );
    assert_eq!(
        reason.get("max_length").and_then(Value::as_u64),
        Some(CANCEL_REASON_MAX_CHARS as u64)
    );
    assert_eq!(emit.get("type").and_then(Value::as_str), Some("enum"));
    assert_eq!(emit.get("default").and_then(Value::as_str), Some("text"));
}

#[test]
fn command_cancel_exposes_parser_reason_bound() {
    let context = build("0.1.0");
    let reason = context
        .pointer("/commands/cancel/flags/--reason")
        .and_then(Value::as_object)
        .expect("cancel --reason must be an object");

    assert_eq!(reason.get("type").and_then(Value::as_str), Some("string"));
    assert_eq!(
        reason.get("length_unit").and_then(Value::as_str),
        Some("characters")
    );
    assert_eq!(
        reason.get("max_length").and_then(Value::as_u64),
        Some(CANCEL_REASON_MAX_CHARS as u64)
    );
}

#[test]
fn agent_context_help_reports_text_only_output_and_real_aliases() {
    let context = build("0.1.0");
    let command = context
        .pointer("/commands/help")
        .and_then(Value::as_object)
        .expect("help command must be an object");

    assert_eq!(
        command.get("summary").and_then(Value::as_str),
        Some("Print this message.")
    );
    assert_eq!(
        command
            .get("outputs")
            .and_then(Value::as_array)
            .expect("help outputs must be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["text"]
    );
    assert_eq!(
        command
            .get("aliases")
            .and_then(Value::as_array)
            .expect("help aliases must be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["--help", "-h"]
    );
}

#[test]
fn agent_context_version_reports_text_only_output_and_real_aliases() {
    let context = build("0.1.0");
    let command = context
        .pointer("/commands/version")
        .and_then(Value::as_object)
        .expect("version command must be an object");

    assert_eq!(
        command.get("summary").and_then(Value::as_str),
        Some("Print version.")
    );
    assert_eq!(
        command
            .get("outputs")
            .and_then(Value::as_array)
            .expect("version outputs must be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["text"]
    );
    assert_eq!(
        command
            .get("aliases")
            .and_then(Value::as_array)
            .expect("version aliases must be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["--version", "-V"]
    );
}

#[test]
fn agent_context_status_matches_parser_and_help_contract() {
    let context = build("0.1.0");
    let flags = context
        .pointer("/commands/status/flags")
        .and_then(Value::as_object)
        .expect("status flags must be an object");
    let shard_config = vb_runtime::shard::ShardConfig::default();

    assert_eq!(
        context
            .pointer("/commands/status/summary")
            .and_then(Value::as_str),
        Some("Report runtime shard status (with live Fjall probe when --db is supplied).")
    );
    assert_eq!(
        flags
            .get("--active-runs")
            .and_then(Value::as_object)
            .and_then(|flag| flag.get("type"))
            .and_then(Value::as_str),
        Some("usize")
    );
    assert_eq!(
        flags
            .get("--active-runs")
            .and_then(Value::as_object)
            .and_then(|flag| flag.get("max"))
            .and_then(Value::as_u64),
        Some(u64::try_from(shard_config.max_active_runs).expect("usize fits into u64"))
    );
    assert_eq!(
        flags
            .get("--queue-depth")
            .and_then(Value::as_object)
            .and_then(|flag| flag.get("type"))
            .and_then(Value::as_str),
        Some("usize")
    );
    assert_eq!(
        flags
            .get("--queue-depth")
            .and_then(Value::as_object)
            .and_then(|flag| flag.get("max"))
            .and_then(Value::as_u64),
        Some(u64::try_from(shard_config.command_queue_capacity).expect("usize fits into u64"))
    );
    assert_eq!(
        flags
            .get("--trace-dropped")
            .and_then(Value::as_object)
            .and_then(|flag| flag.get("type"))
            .and_then(Value::as_str),
        Some("u64")
    );
    assert_eq!(
        flags
            .get("--db")
            .and_then(Value::as_object)
            .and_then(|flag| flag.get("required"))
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        flags
            .get("--emit")
            .and_then(Value::as_object)
            .expect("status emit flag must be an object")
            .get("values")
            .and_then(Value::as_array)
            .expect("status emit values must be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["text", "yaml"]
    );
}

#[test]
fn agent_context_system_status_matches_parser_and_help_contract() {
    let context = build("0.1.0");
    let flags = context
        .pointer("/commands/system status/flags")
        .and_then(Value::as_object)
        .expect("system status flags must be an object");

    assert_eq!(
        context
            .pointer("/commands/system status/summary")
            .and_then(Value::as_str),
        Some("Report bounded system health (probes Fjall when --db is supplied).")
    );
    assert_eq!(
        flags
            .get("--profile")
            .and_then(Value::as_object)
            .expect("system status profile must be an object")
            .get("values")
            .and_then(Value::as_array)
            .expect("system status profile values must be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["quick", "standard", "full"]
    );
    assert_eq!(
        flags
            .get("--server")
            .and_then(Value::as_object)
            .expect("system status server must be an object")
            .get("values")
            .and_then(Value::as_array)
            .expect("system status server values must be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["none"]
    );
    assert_eq!(
        flags
            .get("--db")
            .and_then(Value::as_object)
            .and_then(|flag| flag.get("required"))
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        flags
            .get("--emit")
            .and_then(Value::as_object)
            .expect("system status emit flag must be an object")
            .get("values")
            .and_then(Value::as_array)
            .expect("system status emit values must be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["text", "yaml"]
    );
}

#[test]
fn agent_context_action_list_matches_parser_and_help_contract() {
    let context = build("0.1.0");
    let flags = context
        .pointer("/commands/action list/flags")
        .and_then(Value::as_object)
        .expect("action list flags must be an object");

    assert_eq!(
        context
            .pointer("/commands/action list/summary")
            .and_then(Value::as_str),
        Some("List registered action contracts.")
    );
    assert_eq!(
        flags
            .get("--registry")
            .and_then(Value::as_object)
            .expect("action list registry flag must be an object")
            .get("values")
            .and_then(Value::as_array)
            .expect("action list registry values must be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["registered", "empty", "uninitialized"]
    );
}

#[test]
fn agent_context_action_inspect_matches_parser_and_help_contract() {
    let context = build("0.1.0");
    let command = context
        .pointer("/commands/action inspect")
        .and_then(Value::as_object)
        .expect("action inspect command must be an object");
    let flags = command
        .get("flags")
        .and_then(Value::as_object)
        .expect("action inspect flags must be an object");

    assert_eq!(
        command.get("summary").and_then(Value::as_str),
        Some("Show one registered action contract.")
    );
    assert_eq!(
        command
            .get("positionals")
            .and_then(Value::as_array)
            .expect("action inspect positionals must be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["action-name"]
    );
    assert_eq!(
        flags
            .get("--registry")
            .and_then(Value::as_object)
            .expect("action inspect registry flag must be an object")
            .get("values")
            .and_then(Value::as_array)
            .expect("action inspect registry values must be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["registered", "empty", "uninitialized"]
    );
}

#[test]
fn command_diff_declares_workflow_and_durable_run_modes() {
    let context = build("0.1.0");
    let modes = context
        .pointer("/commands/diff/modes")
        .and_then(Value::as_object)
        .expect("diff modes must be an object");

    assert!(modes.contains_key("workflow"));
    assert!(modes.contains_key("durable_run"));
}

#[test]
fn command_diff_workflow_mode_requires_against_without_db() {
    let context = build("0.1.0");
    let flags = context
        .pointer("/commands/diff/modes/workflow/flags")
        .and_then(Value::as_object)
        .expect("workflow diff flags must be an object");
    let against = flags
        .get("--against")
        .and_then(Value::as_object)
        .expect("--against must be an object");

    assert_eq!(against.get("required").and_then(Value::as_bool), Some(true));
    assert_eq!(against.get("type").and_then(Value::as_str), Some("path"));
    assert!(!flags.contains_key("--db"));
}

#[test]
fn command_diff_durable_run_mode_requires_db_and_run_positionals() {
    let context = build("0.1.0");
    let positionals = context
        .pointer("/commands/diff/modes/durable_run/positionals")
        .and_then(Value::as_array)
        .expect("durable run positionals must be an array");
    let flags = context
        .pointer("/commands/diff/modes/durable_run/flags")
        .and_then(Value::as_object)
        .expect("durable run diff flags must be an object");
    let db = flags
        .get("--db")
        .and_then(Value::as_object)
        .expect("--db must be an object");

    assert_eq!(
        positionals
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["run_a", "run_b"]
    );
    assert_eq!(db.get("required").and_then(Value::as_bool), Some(true));
    assert_eq!(db.get("type").and_then(Value::as_str), Some("path"));
}

#[test]
fn vocabulary_policy_canonical_output_flag_is_dash_dash_emit() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/vocabulary_policy/canonical_output_flag")
            .and_then(Value::as_str),
        Some("--emit")
    );
}

#[test]
fn vocabulary_policy_canonical_output_values_are_text_yaml_postcard() {
    let context = build("0.1.0");
    let values = context
        .pointer("/vocabulary_policy/canonical_output_values")
        .and_then(Value::as_array)
        .expect("canonical_output_values must be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();

    assert_eq!(values, vec!["text", "yaml", "postcard"]);
}

#[test]
fn vocabulary_policy_canonical_destructive_bypass_flag_is_dash_dash_force() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/vocabulary_policy/canonical_destructive_bypass_flag")
            .and_then(Value::as_str),
        Some("--force")
    );
}

#[test]
fn vocabulary_policy_canonical_resource_verbs_has_correct_values() {
    let context = build("0.1.0");
    let verbs = context
        .pointer("/vocabulary_policy/canonical_resource_verbs")
        .and_then(Value::as_array)
        .expect("canonical_resource_verbs must be an array");
    let values: Vec<&str> = verbs.iter().filter_map(Value::as_str).collect();
    assert_eq!(values, vec!["get", "list", "create", "update", "delete"]);
}

#[test]
fn vocabulary_policy_banned_verbs_has_correct_values() {
    let context = build("0.1.0");
    let verbs = context
        .pointer("/vocabulary_policy/banned_verbs")
        .and_then(Value::as_array)
        .expect("banned_verbs must be an array");
    let values: Vec<&str> = verbs.iter().filter_map(Value::as_str).collect();
    assert_eq!(values, vec!["info", "ls"]);
}

#[test]
fn vocabulary_policy_banned_flags_has_correct_values() {
    let context = build("0.1.0");
    let flags = context
        .pointer("/vocabulary_policy/banned_flags")
        .and_then(Value::as_array)
        .expect("banned_flags must be an array");
    let values: Vec<&str> = flags.iter().filter_map(Value::as_str).collect();
    assert_eq!(
        values,
        vec![
            "--json",
            "--jsonl",
            "--format=json",
            "--output=json",
            "--skip-confirmations"
        ]
    );
}

#[test]
fn agent_contract_non_interactive_by_default_is_true() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/agent_contract/non_interactive_by_default")
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn agent_contract_ansi_when_non_tty_is_false() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/agent_contract/ansi_when_non_tty")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn agent_contract_bounded_output_required_is_true() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/agent_contract/bounded_output_required")
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn agent_contract_destructive_operations_require_explicit_flag_is_true() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/agent_contract/destructive_operations_require_explicit_flag")
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn agent_contract_mutation_responses_return_identifiers_is_true() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/agent_contract/mutation_responses_return_identifiers")
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn agent_contract_stdout_is_data_only() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/agent_contract/stdout")
            .and_then(Value::as_str),
        Some("data only")
    );
}

#[test]
fn agent_contract_stderr_is_diagnostics_only() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/agent_contract/stderr")
            .and_then(Value::as_str),
        Some("diagnostics only")
    );
}

#[test]
fn agent_contract_prompt_bypass_flag_is_dash_dash_force() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/agent_contract/prompt_bypass_flag")
            .and_then(Value::as_str),
        Some("--force")
    );
}

#[test]
fn agent_contract_machine_output_flag_is_dash_dash_emit_postcard() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/agent_contract/machine_output_flag")
            .and_then(Value::as_str),
        Some("--emit postcard")
    );
}

#[test]
fn agent_contract_structured_output_flag_is_dash_dash_emit_yaml() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/agent_contract/structured_output_flag")
            .and_then(Value::as_str),
        Some("--emit yaml")
    );
}

#[test]
fn planned_agent_primitives_async_wait_flag_is_dash_dash_wait() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/planned_agent_primitives/async_wait_flag")
            .and_then(Value::as_str),
        Some("--wait")
    );
}

#[test]
fn planned_agent_primitives_jobs_commands_has_correct_values() {
    let context = build("0.1.0");
    let cmds = context
        .pointer("/planned_agent_primitives/jobs_commands")
        .and_then(Value::as_array)
        .expect("jobs_commands must be an array");
    let values: Vec<&str> = cmds.iter().filter_map(Value::as_str).collect();
    assert_eq!(values, vec!["jobs list", "jobs get", "jobs prune"]);
}

#[test]
fn planned_agent_primitives_delivery_flag_is_dash_dash_deliver() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/planned_agent_primitives/delivery_flag")
            .and_then(Value::as_str),
        Some("--deliver")
    );
}

#[test]
fn planned_agent_primitives_feedback_command_is_feedback() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/planned_agent_primitives/feedback_command")
            .and_then(Value::as_str),
        Some("feedback")
    );
}

#[test]
fn planned_agent_primitives_profile_commands_has_correct_values() {
    let context = build("0.1.0");
    let cmds = context
        .pointer("/planned_agent_primitives/profile_commands")
        .and_then(Value::as_array)
        .expect("profile_commands must be an array");
    let values: Vec<&str> = cmds.iter().filter_map(Value::as_str).collect();
    assert_eq!(
        values,
        vec![
            "profile save",
            "profile list",
            "profile show",
            "profile delete"
        ]
    );
}

#[test]
fn build_from_different_versions_produce_identical_structure() {
    let a = build("0.1.0");
    let b = build("9.9.9");

    let a_without_version = remove_key(&a, "version");
    let b_without_version = remove_key(&b, "version");
    assert_eq!(
        a_without_version, b_without_version,
        "only the 'version' field should differ between calls"
    );
}

#[test]
fn command_all_have_summary_field() {
    let context = build("0.1.0");
    let commands = context
        .get("commands")
        .and_then(Value::as_object)
        .expect("commands must be an object");

    for (name, cmd) in commands.iter() {
        assert!(
            cmd.get("summary").and_then(Value::as_str).is_some(),
            "command '{}' must have a summary",
            name
        );
    }
}

#[test]
fn command_answer_has_slot_and_value_and_db_flags() {
    let context = build("0.1.0");
    let flags = context
        .pointer("/commands/answer/flags")
        .and_then(Value::as_object)
        .expect("flags must be an object");

    for flag_name in &["--slot", "--value", "--db"] {
        let flag = flags
            .get(*flag_name)
            .and_then(Value::as_object)
            .unwrap_or_else(|| panic!("{} must be an object", flag_name));
        assert_eq!(flag.get("required").and_then(Value::as_bool), Some(true));
    }
}

#[test]
fn command_submit_has_durability_flag_required() {
    let context = build("0.1.0");
    let flags = context
        .pointer("/commands/submit/flags")
        .and_then(Value::as_object)
        .expect("flags must be an object");
    let durability = flags
        .get("--durability")
        .and_then(Value::as_object)
        .expect("--durability must be an object");
    assert_eq!(
        durability.get("required").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(durability.get("type").and_then(Value::as_str), Some("enum"));
}

fn remove_key(value: &Value, key: &str) -> Value {
    match value {
        Value::Object(map) => {
            let filtered: serde_json::Map<_, _> = map
                .iter()
                .filter(|(k, _)| k.as_str() != key)
                .map(|(k, v)| (k.clone(), remove_key(v, key)))
                .collect();
            Value::Object(filtered)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(|v| remove_key(v, key)).collect()),
        other => other.clone(),
    }
}
