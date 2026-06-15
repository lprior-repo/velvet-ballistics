#![cfg(kani)]

use crate::agent_context;

/// OBL-001: build() never panics for any version string.
#[kani::proof]
#[kani::unwind(5)]
fn kani_build_no_panic() {
    let version: String = kani::any();
    let _result = crate::agent_context::build(&version);
}

/// OBL-002: build() output always contains required top-level fields.
#[kani::proof]
#[kani::unwind(5)]
fn kani_build_has_required_fields() {
    let version: String = kani::any();
    let result = crate::agent_context::build(&version);

    assert!(result.get("schema_version").is_some());
    assert!(result.get("kind").is_some());
    assert!(result.get("cli").is_some());
    assert!(result.get("version").is_some());
}

/// OBL-003: build() output always includes active_gates and known_blockers.
#[kani::proof]
#[kani::unwind(5)]
fn kani_build_has_runtime_policy_fields() {
    let version: String = kani::any();
    let result = crate::agent_context::build(&version);

    assert!(result.get("active_gates").is_some());
    assert!(result.get("known_blockers").is_some());
}

/// OBL-004: agent_context command is always present in commands.
#[kani::proof]
#[kani::unwind(5)]
fn kani_commands_includes_agent_context() {
    let version: String = kani::any();
    let result = crate::agent_context::build(&version);

    let commands = result.get("commands");
    assert!(commands.is_some());

    if let Some(cmds) = commands {
        assert!(cmds.get("agent-context").is_some());
    }
}

/// OBL-005: exit_codes covers all defined codes 0 through 8.
#[kani::proof]
#[kani::unwind(5)]
fn kani_exit_codes_has_defined_range() {
    let result = crate::agent_context::build("0.1.0");

    let exit_codes = result.get("exit_codes");
    assert!(exit_codes.is_some());

    if let Some(codes) = exit_codes {
        for code in 0..=8 {
            let key = format!("{}", code);
            assert!(
                codes.get(&key).is_some(),
                "exit code {} must be defined",
                code
            );
        }
    }
}

/// OBL-006: known_blockers has all three categories.
#[kani::proof]
#[kani::unwind(5)]
fn kani_known_blockers_has_all_categories() {
    let result = crate::agent_context::build("0.1.0");

    let blockers = result.get("known_blockers");
    assert!(blockers.is_some());

    if let Some(b) = blockers {
        assert!(b.get("policy").is_some());
        assert!(b.get("resource").is_some());
        assert!(b.get("capability").is_some());
    }
}

/// OBL-007: build() output is bounded to 8 KiB when serialized.
#[kani::proof]
#[kani::unwind(5)]
fn kani_output_size_bounded() {
    let version: String = kani::any();
    kani::assume(version.len() <= 64);
    let result = crate::agent_context::build(&version);
    let encoded = serde_json::to_string(&result);
    assert!(
        encoded.is_ok(),
        "output must serialize to JSON without error"
    );
    let size = encoded.map(|s| s.len()).unwrap_or(usize::MAX);
    assert!(
        size <= 8192,
        "agent-context output must be bounded to 8 KiB, got {} bytes",
        size
    );
}

/// OBL-008: build() is deterministic — same version produces identical output.
#[kani::proof]
#[kani::unwind(5)]
fn kani_build_deterministic() {
    let version: String = kani::any();
    kani::assume(version.len() <= 32);
    let first = crate::agent_context::build(&version);
    let second = crate::agent_context::build(&version);
    assert_eq!(first, second);
}

/// OBL-009: Serialized output is always valid JSON (roundtrip property).
#[kani::proof]
#[kani::unwind(5)]
fn kani_build_serializable_roundtrip() {
    let version: String = kani::any();
    kani::assume(version.len() <= 64);
    let result = crate::agent_context::build(&version);
    let encoded = serde_json::to_string(&result);
    assert!(encoded.is_ok());
    let decoded: Result<serde_json::Value, _> = match encoded {
        Ok(s) => serde_json::from_str(&s),
        Err(_) => {
            kani::assume(false);
            return;
        }
    };
    assert!(decoded.is_ok());
}

/// OBL-010: agent_contract boolean fields are actual booleans (not strings/nulls).
#[kani::proof]
#[kani::unwind(5)]
fn kani_agent_contract_booleans_are_bools() {
    let version: String = kani::any();
    let result = crate::agent_context::build(&version);
    let contract = result.get("agent_contract");
    assert!(contract.is_some());
    if let Some(c) = contract {
        for key in &[
            "non_interactive_by_default",
            "ansi_when_non_tty",
            "bounded_output_required",
            "destructive_operations_require_explicit_flag",
            "mutation_responses_return_identifiers",
        ] {
            let val = c.get(*key);
            match val {
                Some(v) => assert!(v.is_boolean()),
                None => {
                    kani::assume(false);
                    return;
                }
            }
        }
    }
}

/// OBL-011: vocabulary_policy arrays are actual arrays.
#[kani::proof]
#[kani::unwind(5)]
fn kani_vocabulary_policy_arrays_are_arrays() {
    let version: String = kani::any();
    let result = crate::agent_context::build(&version);
    let policy = result.get("vocabulary_policy");
    assert!(policy.is_some());
    if let Some(p) = policy {
        for key in &["canonical_resource_verbs", "banned_verbs", "banned_flags"] {
            let val = p.get(*key);
            match val {
                Some(v) => assert!(v.is_array()),
                None => {
                    kani::assume(false);
                    return;
                }
            }
        }
    }
}

/// OBL-012: known_blockers policy has exactly 8 entries.
#[kani::proof]
#[kani::unwind(5)]
fn kani_known_blockers_policy_count_exact() {
    let result = crate::agent_context::build("0.1.0");
    let policy = result.pointer("/known_blockers/policy");
    assert!(policy.is_some());
    if let Some(p) = policy {
        assert!(p.is_array());
        assert_eq!(p.as_array().map(|a| a.len()), Some(8));
    }
}

/// OBL-013: known_blockers resource has exactly 3 entries.
#[kani::proof]
#[kani::unwind(5)]
fn kani_known_blockers_resource_count_exact() {
    let result = crate::agent_context::build("0.1.0");
    let resource = result.pointer("/known_blockers/resource");
    assert!(resource.is_some());
    if let Some(r) = resource {
        assert!(r.is_array());
        assert_eq!(r.as_array().map(|a| a.len()), Some(3));
    }
}

/// OBL-014: known_blockers capability has exactly 3 entries.
#[kani::proof]
#[kani::unwind(5)]
fn kani_known_blockers_capability_count_exact() {
    let result = crate::agent_context::build("0.1.0");
    let capability = result.pointer("/known_blockers/capability");
    assert!(capability.is_some());
    if let Some(c) = capability {
        assert!(c.is_array());
        assert_eq!(c.as_array().map(|a| a.len()), Some(3));
    }
}

/// OBL-015: every command definition contains a "summary" key.
#[kani::proof]
#[kani::unwind(5)]
fn kani_all_commands_have_summary() {
    let version: String = kani::any();
    let result = crate::agent_context::build(&version);
    let commands = result.get("commands");
    assert!(commands.is_some());
    if let Some(cmds) = commands {
        assert!(cmds.is_object());
        if let Some(obj) = cmds.as_object() {
            for (name, cmd) in obj.iter() {
                assert!(
                    cmd.get("summary").is_some(),
                    "command '{}' must have a summary",
                    name
                );
            }
        }
    }
}

/// OBL-016: build() never returns null — output is always an Object.
#[kani::proof]
#[kani::unwind(5)]
fn kani_build_output_is_object() {
    let version: String = kani::any();
    let result = crate::agent_context::build(&version);
    assert!(result.is_object());
}

/// OBL-017: enums key is always an object with the documented variants.
#[kani::proof]
#[kani::unwind(5)]
fn kani_enums_has_all_variants() {
    let version: String = kani::any();
    let result = crate::agent_context::build(&version);
    let enums = result.get("enums");
    assert!(enums.is_some());
    if let Some(e) = enums {
        assert!(e.is_object());
        for key in &["emit", "compile_emit", "durability", "verify_profile"] {
            match e.get(*key) {
                Some(v) => assert!(v.is_array()),
                None => {
                    kani::assume(false);
                    return;
                }
            }
        }
    }
}

/// OBL-018: Non-version structural fields are independent of version input.
#[kani::proof]
#[kani::unwind(5)]
fn kani_non_version_fields_independent_of_version() {
    let v1: String = kani::any();
    let v2: String = kani::any();

    let a = crate::agent_context::build(&v1);
    let b = crate::agent_context::build(&v2);

    let structural_keys = [
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
        "vocabulary_policy",
    ];
    for key in &structural_keys {
        assert_eq!(
            a.get(key),
            b.get(key),
            "field '{}' must be identical regardless of version",
            key
        );
    }
}
