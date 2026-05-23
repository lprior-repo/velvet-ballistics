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
}
