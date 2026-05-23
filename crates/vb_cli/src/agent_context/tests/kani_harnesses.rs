#![cfg(kani)]

use crate::agent_context;

/// OBL-001: build() never panics for any version string.
#[kani::proof]
#[kani::unwind(5)]
fn kani_build_no_panic() {
    let version: String = kani::any();
    let _result = agent_context::build(&version);
}

/// OBL-002: build() output always contains required top-level fields.
#[kani::proof]
#[kani::unwind(5)]
fn kani_build_has_required_fields() {
    let version: String = kani::any();
    let result = agent_context::build(&version);

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
    let result = agent_context::build(&version);

    assert!(result.get("active_gates").is_some());
    assert!(result.get("known_blockers").is_some());
}

/// OBL-004: agent_context command is always present in commands.
#[kani::proof]
#[kani::unwind(5)]
fn kani_commands_includes_agent_context() {
    let version: String = kani::any();
    let result = agent_context::build(&version);

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
    let result = agent_context::build("0.1.0");

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
    let result = agent_context::build("0.1.0");

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
    let result = agent_context::build(&version);
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
