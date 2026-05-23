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
