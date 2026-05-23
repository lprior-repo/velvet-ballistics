use crate::agent_context::build;
use serde_json::Value;

#[test]
fn build_has_versioned_schema_and_json_flag() {
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
        Some("--json")
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
        Some("velvet-ballastics")
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
            .any(|v| v.as_str() == Some("velvet-ballastics"))
    );
    assert!(aliases.iter().any(|v| v.as_str() == Some("vb")));
}

#[test]
fn build_has_language_version() {
    let context = build("0.1.0");
    assert_eq!(
        context.get("language_version").and_then(Value::as_str),
        Some("velvet-ballastics/v1")
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
    assert!(contract.contains_key("streaming_output_flag"));
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
fn enums_has_emit_durability_verify_profile() {
    let context = build("0.1.0");
    let enums = context
        .get("enums")
        .and_then(Value::as_object)
        .expect("enums must be an object");

    assert!(enums.contains_key("emit"));
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
    assert_eq!(values, vec!["ir", "rust", "yaml", "postcard"]);
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
        "agent-context",
        "answer",
        "bench-run",
        "compile",
        "diff",
        "doctor",
        "events",
        "explain",
        "graph",
        "incident",
        "inspect",
        "ipc-serve",
        "replay",
        "resume",
        "retry",
        "run",
        "run-compiled",
        "simulate",
        "submit",
        "trace",
        "validate",
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
    assert_eq!(commands.len(), 22);
}

#[test]
fn command_agent_context_has_summary_and_outputs() {
    let context = build("0.1.0");
    let cmd = context
        .pointer("/commands/agent-context")
        .and_then(Value::as_object)
        .expect("agent-context command must be an object");

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
fn vocabulary_policy_canonical_output_flag_is_dash_dash_json() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/vocabulary_policy/canonical_output_flag")
            .and_then(Value::as_str),
        Some("--json")
    );
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
        vec!["--format=json", "--output=json", "--skip-confirmations"]
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
fn agent_contract_streaming_output_flag_is_dash_dash_jsonl() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/agent_contract/streaming_output_flag")
            .and_then(Value::as_str),
        Some("--jsonl")
    );
}

#[test]
fn agent_contract_structured_output_flag_is_dash_dash_json() {
    let context = build("0.1.0");
    assert_eq!(
        context
            .pointer("/agent_contract/structured_output_flag")
            .and_then(Value::as_str),
        Some("--json")
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
fn command_answer_has_step_and_value_file_and_db_flags() {
    let context = build("0.1.0");
    let flags = context
        .pointer("/commands/answer/flags")
        .and_then(Value::as_object)
        .expect("flags must be an object");

    for flag_name in &["--step", "--value-file", "--db"] {
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
