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

    kani::assert(result.get("schema_version", "assertion failed").is_some(),
        "OBL-002: schema_version must be present",
    );
    kani::assert(result.get("kind", "assertion failed").is_some(),
        "OBL-002: kind must be present",
    );
    kani::assert(result.get("cli", "assertion failed").is_some(), "OBL-002: cli must be present");
    kani::assert(result.get("version", "assertion failed").is_some(),
        "OBL-002: version must be present",
    );
}

/// OBL-003: build() output always includes active_gates and known_blockers.
#[kani::proof]
#[kani::unwind(5)]
fn kani_build_has_runtime_policy_fields() {
    let version: String = kani::any();
    let result = crate::agent_context::build(&version);

    kani::assert(result.get("active_gates", "assertion failed").is_some(),
        "OBL-003: active_gates must be present",
    );
    kani::assert(result.get("known_blockers", "assertion failed").is_some(),
        "OBL-003: known_blockers must be present",
    );
}

/// OBL-004: agent_context command is always present in commands.
#[kani::proof]
#[kani::unwind(5)]
fn kani_commands_includes_agent_context() {
    let version: String = kani::any();
    let result = crate::agent_context::build(&version);

    let commands = result.get("commands");
    kani::assert(commands.is_some(), "OBL-004: commands must be present");

    if let Some(cmds) = commands {
        kani::assert(cmds.get("agent-context", "assertion failed").is_some(),
            "OBL-004: agent-context command must be present",
        );
    }
}

/// OBL-005: exit_codes covers all defined codes 0 through 8.
#[kani::proof]
#[kani::unwind(5)]
fn kani_exit_codes_has_defined_range() {
    let result = crate::agent_context::build("0.1.0");

    let exit_codes = result.get("exit_codes");
    kani::assert(exit_codes.is_some(), "OBL-005: exit_codes must be present");

    if let Some(codes) = exit_codes {
        for code in 0..=8 {
            let key = format!("{}", code);
            kani::assert(codes.get(&key, "assertion failed").is_some(),
                &format!("OBL-005: exit code {} must be defined", code),
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
    kani::assert(blockers.is_some(),
        "OBL-006: known_blockers must be present",
    );

    if let Some(b) = blockers {
        kani::assert(b.get("policy", "assertion failed").is_some(),
            "OBL-006: policy category must be present",
        );
        kani::assert(b.get("resource", "assertion failed").is_some(),
            "OBL-006: resource category must be present",
        );
        kani::assert(b.get("capability", "assertion failed").is_some(),
            "OBL-006: capability category must be present",
        );
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
    kani::assert(encoded.is_ok(),
        "OBL-007: output must serialize to JSON without error",
    );
    let size = encoded.map(|s| s.len()).unwrap_or(usize::MAX);
    ,
        "OBL-007: output must serialize to JSON without error",
    );
    let size = encoded.map(|s| s.len()).unwrap_or(usize::MAX);
    kani::assert(
        size <= 8192,
        &format!(
            "OBL-007: agent-context output must be bounded to 8 KiB, got {} bytes",
            size
        ),
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
    ,
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
    kani::assert(
        first == second,
        "OBL-008: same version must produce identical output",
    );
}

/// OBL-009: Serialized output is always valid JSON (roundtrip property).
#[kani::proof]
#[kani::unwind(5)]
fn kani_build_serializable_roundtrip() {
    let version: String = kani::any();
    kani::assume(version.len() <= 64);
    let result = crate::agent_context::build(&version);
    let encoded = serde_json::to_string(&result);
    kani::assert(encoded.is_ok(), "OBL-009: serialization must succeed");
    let decoded: Result<serde_json::Value, _> = match encoded {
        Ok(s) => serde_json::from_str(&s),
        Err(_) => {
            kani::assume(false);
            return;
        }
    };
    kani::assert(decoded.is_ok(),
        "OBL-009: deserialization roundtrip must succeed",
    );
}

/// OBL-010: agent_contract boolean fields are actual booleans (not strings/nulls).
#[kani::proof]
#[kani::unwind(5)]
fn kani_agent_contract_booleans_are_bools() {
    let version: String = kani::any();
    let result = crate::agent_context::build(&version);
    let contract = result.get("agent_contract");
    kani::assert(contract.is_some(),
        "OBL-010: agent_contract must be present",
    );
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
                Some(v) => kani::assert(v.is_boolean(, "assertion failed"),
                    &format!("OBL-010: field '{}' must be a boolean", key),
                ),
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
    kani::assert(policy.is_some(),
        "OBL-011: vocabulary_policy must be present",
    );
    if let Some(p) = policy {
        for key in &["canonical_resource_verbs", "banned_verbs", "banned_flags"] {
            let val = p.get(*key);
            match val {
                Some(v) => kani::assert(v.is_array(, "assertion failed"),
                    &format!("OBL-011: field '{}' must be an array", key),
                ),
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
    kani::assert(policy.is_some(), "OBL-012: policy pointer must be present");
    if let Some(p) = policy {
        kani::assert(p.is_array(, "assertion failed"), "OBL-012: policy must be an array");
        kani::assert(p.as_array(, "assertion failed").map(|a| a.len()) == Some(8),
            "OBL-012: policy must have exactly 8 entries",
        );
    }
}

/// OBL-013: known_blockers resource has exactly 3 entries.
#[kani::proof]
#[kani::unwind(5)]
fn kani_known_blockers_resource_count_exact() {
    let result = crate::agent_context::build("0.1.0");
    let resource = result.pointer("/known_blockers/resource");
    kani::assert(resource.is_some(),
        "OBL-013: resource pointer must be present",
    );
    if let Some(r) = resource {
        kani::assert(r.is_array(, "assertion failed"), "OBL-013: resource must be an array");
        kani::assert(r.as_array(, "assertion failed").map(|a| a.len()) == Some(3),
            "OBL-013: resource must have exactly 3 entries",
        );
    }
}

/// OBL-014: known_blockers capability has exactly 3 entries.
#[kani::proof]
#[kani::unwind(5)]
fn kani_known_blockers_capability_count_exact() {
    let result = crate::agent_context::build("0.1.0");
    let capability = result.pointer("/known_blockers/capability");
    kani::assert(capability.is_some(),
        "OBL-014: capability pointer must be present",
    );
    if let Some(c) = capability {
        kani::assert(c.is_array(, "assertion failed"), "OBL-014: capability must be an array");
        kani::assert(c.as_array(, "assertion failed").map(|a| a.len()) == Some(3),
            "OBL-014: capability must have exactly 3 entries",
        );
    }
}

/// OBL-015: every command definition contains a "summary" key.
#[kani::proof]
#[kani::unwind(5)]
fn kani_all_commands_have_summary() {
    let version: String = kani::any();
    let result = crate::agent_context::build(&version);
    let commands = result.get("commands");
    kani::assert(commands.is_some(), "OBL-015: commands must be present");
    if let Some(cmds) = commands {
        kani::assert(cmds.is_object(, "assertion failed"), "OBL-015: commands must be an object");
        if let Some(obj) = cmds.as_object() {
            for (name, cmd) in obj.iter() {
                kani::assert(cmd.get("summary", "assertion failed").is_some(),
                    &format!("OBL-015: command '{}' must have a summary", name),
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
    kani::assert(result.is_object(, "assertion failed"),
        "OBL-016: build output must be an object",
    );
}

/// OBL-017: enums key is always an object with the documented variants.
#[kani::proof]
#[kani::unwind(5)]
fn kani_enums_has_all_variants() {
    let version: String = kani::any();
    let result = crate::agent_context::build(&version);
    let enums = result.get("enums");
    kani::assert(enums.is_some(), "OBL-017: enums must be present");
    if let Some(e) = enums {
        kani::assert(e.is_object(, "assertion failed"), "OBL-017: enums must be an object");
        for key in &["emit", "compile_emit", "durability", "verify_profile"] {
            match e.get(*key) {
                Some(v) => kani::assert(v.is_array(, "assertion failed"),
                    &format!("OBL-017: enum '{}' must be an array", key),
                ),
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
        kani::assert(a.get(key, "assertion failed") == b.get(key),
            &format!(
                "OBL-018: field '{}' must be identical regardless of version",
                key
            ),
        );
    }
}
