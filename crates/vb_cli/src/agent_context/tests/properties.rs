#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::panic
)]
use crate::agent_context::build;
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_build_always_has_schema_version(version in "\\PC*") {
        let result = build(&version);
        prop_assert!(result.get("schema_version").is_some());
    }

    #[test]
    fn prop_build_always_has_kind(version in "\\PC*") {
        let result = build(&version);
        prop_assert!(result.get("kind").is_some());
    }

    #[test]
    fn prop_build_version_matches_input(version in "\\PC*") {
        let result = build(&version);
        prop_assert_eq!(
            result.get("version").and_then(serde_json::Value::as_str),
            Some(version.as_str())
        );
    }

    #[test]
    fn prop_build_exit_codes_stable(version in "\\PC*") {
        let result = build(&version);
        let exit_codes = result
            .get("exit_codes")
            .and_then(serde_json::Value::as_object)
            .expect("exit_codes must be an object");
        for code in 0..=8 {
            let key = format!("{}", code);
            prop_assert!(
                exit_codes.contains_key(&key),
                "exit code {} must be defined",
                code
            );
        }
    }

    #[test]
    fn prop_build_commands_stable(version in "\\PC*") {
        let result = build(&version);
        let commands = result
            .get("commands")
            .and_then(serde_json::Value::as_object)
            .expect("commands must be an object");
        prop_assert!(commands.contains_key("agent-context"));
        prop_assert!(commands.contains_key("cancel"));
        prop_assert!(commands.contains_key("validate"));
        prop_assert!(commands.contains_key("compile"));
        prop_assert!(commands.contains_key("run"));
    }

    #[test]
    fn prop_build_active_gates_stable(version in "\\PC*") {
        let result = build(&version);
        let gates = result
            .get("active_gates")
            .and_then(serde_json::Value::as_object)
            .expect("active_gates must be an object");
        prop_assert!(gates.contains_key("validation"));
        prop_assert!(gates.contains_key("verification"));
        prop_assert!(gates.contains_key("compilation"));
    }

    #[test]
    fn prop_build_known_blockers_stable(version in "\\PC*") {
        let result = build(&version);
        let blockers = result
            .get("known_blockers")
            .and_then(serde_json::Value::as_object)
            .expect("known_blockers must be an object");
        prop_assert!(blockers.contains_key("policy"));
        prop_assert!(blockers.contains_key("resource"));
        prop_assert!(blockers.contains_key("capability"));
    }

    // ── Extended property tests ──────────────────────────────────

    #[test]
    fn prop_build_always_has_all_top_level_keys(version in "\\PC*") {
        let result = build(&version);
        let expected = [
            "active_gates", "agent_contract", "binary_aliases",
            "cli", "commands", "enums", "exit_codes", "kind",
            "known_blockers", "language_version",
            "planned_agent_primitives", "schema_version",
            "version", "vocabulary_policy",
        ];
        for key in &expected {
            prop_assert!(
                result.get(key).is_some(),
                "top-level key '{}' must be present",
                key
            );
        }
    }

    #[test]
    fn prop_build_deterministic(version in "\\PC*") {
        let first = build(&version);
        let second = build(&version);
        prop_assert_eq!(first, second);
    }

    #[test]
    fn prop_build_agent_contract_has_all_keys(version in "\\PC*") {
        let result = build(&version);
        let contract = result
            .get("agent_contract")
            .and_then(serde_json::Value::as_object)
            .expect("agent_contract must be an object");
        let keys = [
            "ansi_when_non_tty", "bounded_output_required",
            "destructive_operations_require_explicit_flag",
            "mutation_responses_return_identifiers",
            "non_interactive_by_default", "prompt_bypass_flag",
            "machine_output_flag", "stderr", "stdout",
            "structured_output_flag",
        ];
        for key in &keys {
            prop_assert!(
                contract.contains_key(*key),
                "agent_contract must contain '{}'",
                key
            );
        }
    }

    #[test]
    fn prop_build_vocabulary_policy_has_all_keys(version in "\\PC*") {
        let result = build(&version);
        let policy = result
            .get("vocabulary_policy")
            .and_then(serde_json::Value::as_object)
            .expect("vocabulary_policy must be an object");
        let keys = [
            "banned_flags", "banned_verbs", "canonical_destructive_bypass_flag",
            "canonical_output_flag", "canonical_output_values", "canonical_resource_verbs",
        ];
        for key in &keys {
            prop_assert!(
                policy.contains_key(*key),
                "vocabulary_policy must contain '{}'",
                key
            );
        }
    }

    #[test]
    fn prop_build_enums_has_all_variants(version in "\\PC*") {
        let result = build(&version);
        let enums = result
            .get("enums")
            .and_then(serde_json::Value::as_object)
            .expect("enums must be an object");
        prop_assert!(enums.contains_key("emit"));
        prop_assert!(enums.contains_key("compile_emit"));
        prop_assert!(enums.contains_key("durability"));
        prop_assert!(enums.contains_key("verify_profile"));
    }

    #[test]
    fn prop_build_serializable_roundtrip(version in "\\PC*") {
        let result = build(&version);
        let encoded = serde_json::to_string(&result).expect("must serialize to JSON string");
        let decoded: serde_json::Value = serde_json::from_str(&encoded).expect("must deserialize from JSON string");
        prop_assert_eq!(decoded, result);
    }

    #[test]
    fn prop_build_known_blockers_policy_length(version in "\\PC*") {
        let result = build(&version);
        let policy = result
            .pointer("/known_blockers/policy")
            .and_then(serde_json::Value::as_array)
            .expect("policy must be an array");
        prop_assert_eq!(policy.len(), 8);
    }

    #[test]
    fn prop_build_known_blockers_resource_length(version in "\\PC*") {
        let result = build(&version);
        let resource = result
            .pointer("/known_blockers/resource")
            .and_then(serde_json::Value::as_array)
            .expect("resource must be an array");
        prop_assert_eq!(resource.len(), 3);
    }

    #[test]
    fn prop_build_known_blockers_capability_length(version in "\\PC*") {
        let result = build(&version);
        let capability = result
            .pointer("/known_blockers/capability")
            .and_then(serde_json::Value::as_array)
            .expect("capability must be an array");
        prop_assert_eq!(capability.len(), 3);
    }

    #[test]
    fn prop_build_output_is_object(version in "\\PC*") {
        let result = build(&version);
        prop_assert!(result.is_object());
    }

    #[test]
    fn prop_build_commands_count_is_30(version in "\\PC*") {
        let result = build(&version);
        let commands = result
            .get("commands")
            .and_then(serde_json::Value::as_object)
            .expect("commands must be an object");
        prop_assert_eq!(commands.len(), 30);
    }

    #[test]
    fn prop_build_non_version_fields_independent_of_version(version in "\\PC*") {
        let a = build(&version);
        let b = build("0.1.0");

        let expected = [
            "active_gates", "agent_contract", "binary_aliases",
            "cli", "commands", "enums", "exit_codes", "kind",
            "known_blockers", "language_version",
            "planned_agent_primitives", "schema_version",
            "vocabulary_policy",
        ];
        for key in &expected {
            prop_assert_eq!(
                a.get(key), b.get(key),
                "field '{}' must be identical regardless of version",
                key
            );
        }
    }
}
