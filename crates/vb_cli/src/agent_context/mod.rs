mod commands;

#![forbid(unsafe_code)]
#![cfg_attr(not(kani), allow(dead_code, unused_mut, unused_variables))]

use serde_json::Value;

/// Build the machine-readable CLI surface for AI agents.
pub(crate) fn build(version: &str) -> Value {
    serde_json::json!({
        "schema_version": "1",
        "kind": "AgentContext",
        "cli": "velvet-ballistics",
        "binary_aliases": ["velvet-ballistics"],
        "version": version,
        "language_version": "velvet-ballistics/v1",
        "agent_contract": {
            "non_interactive_by_default": true,
            "prompt_bypass_flag": "--force",
            "structured_output_flag": "--emit yaml",
            "machine_output_flag": "--emit postcard",
            "stdout": "data only",
            "stderr": "diagnostics only",
            "ansi_when_non_tty": false,
            "bounded_output_required": true,
            "destructive_operations_require_explicit_flag": true,
            "mutation_responses_return_identifiers": true
        },
        "vocabulary_policy": {
            "canonical_output_flag": "--emit",
            "canonical_output_values": ["text", "yaml", "postcard"],
            "canonical_destructive_bypass_flag": "--force",
            "canonical_resource_verbs": ["get", "list", "create", "update", "delete"],
            "banned_verbs": ["info", "ls"],
            "banned_flags": ["--json", "--jsonl", "--format=json", "--output=json", "--skip-confirmations"]
        },
        "active_gates": active_gates(),
        "known_blockers": known_blockers(),
        "exit_codes": exit_codes(),
        "enums": enums(),
        "commands": commands::commands(),
        "planned_agent_primitives": planned_agent_primitives()
    })
}

fn active_gates() -> Value {
    serde_json::json!({
        "validation": {"required": true, "gate": "vb_validate"},
        "verification": {"required": true, "gate": "vb_verify"},
        "compilation": {"required": true, "gate": "vb_compile"},
        "admission": {"required": true, "gate": "vb_storage::admission"},
        "durability": {"required": false, "gate": "vb_storage::FjallJournal"}
    })
}

fn known_blockers() -> Value {
    serde_json::json!({
        "policy": [
            {"category": "validation_failed", "exit_code": 1},
            {"category": "verification_failed", "exit_code": 2},
            {"category": "compile_failed", "exit_code": 3},
            {"category": "runtime_failed", "exit_code": 4},
            {"category": "storage_error", "exit_code": 5},
            {"category": "ipc_error", "exit_code": 6},
            {"category": "action_policy_error", "exit_code": 7},
            {"category": "replay_divergence", "exit_code": 8}
        ],
        "resource": [
            {"category": "slot_count_exceeded"},
            {"category": "input_index_out_of_range"},
            {"category": "journal_capacity"}
        ],
        "capability": [
            {"category": "unregistered_action"},
            {"category": "missing_capability"},
            {"category": "capability_mismatch"}
        ]
    })
}

fn exit_codes() -> Value {
    serde_json::json!({
        "0": "success",
        "1": "validation failed",
        "2": "verification failed",
        "3": "compile failed",
        "4": "runtime failed",
        "5": "storage error",
        "6": "ipc error",
        "7": "action policy error",
        "8": "replay divergence"
    })
}

fn enums() -> Value {
    serde_json::json!({
        "emit": ["ir", "yaml", "postcard"],
        "durability": ["strict", "journaled", "none"],
        "verify_profile": ["quick", "standard", "full"]
    })
}

fn planned_agent_primitives() -> Value {
    serde_json::json!({
        "async_wait_flag": "--wait",
        "jobs_commands": ["jobs list", "jobs get", "jobs prune"],
        "profile_commands": ["profile save", "profile list", "profile show", "profile delete"],
        "delivery_flag": "--deliver",
        "feedback_command": "feedback"
    })
}

#[cfg(test)]
mod tests;
