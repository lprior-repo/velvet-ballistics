#![forbid(unsafe_code)]
#![cfg_attr(not(kani), allow(dead_code, unused_mut, unused_variables))]

use serde_json::{Map, Value, json};

use super::constants::CLI_NAME;

/// Active validation/verification gates for the pipeline.
pub(crate) fn active_gates() -> Value {
    json!({
        "validation": {"required": true, "gate": "vb_validate"},
        "verification": {"required": true, "gate": "vb_verify"},
        "compilation": {"required": true, "gate": "vb_compile"},
        "admission": {"required": true, "gate": "vb_storage::admission"},
        "durability": {"required": false, "gate": "vb_storage::FjallJournal"}
    })
}

/// Known error categories that block pipeline execution.
pub(crate) fn known_blockers() -> Value {
    json!({
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

/// Numeric exit codes and their human-readable descriptions.
pub(crate) fn exit_codes() -> Value {
    json!({
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

/// Enum value groups used across commands and flags.
pub(crate) fn enums() -> Value {
    json!({
        "emit": ["text", "yaml", "postcard"],
        "compile_emit": ["ir", "yaml", "postcard"],
        "durability": ["strict", "journaled", "none"],
        "verify_profile": ["quick", "standard", "full"]
    })
}

/// Agent contract: behavioral rules for AI agents consuming the CLI.
pub(crate) fn agent_contract() -> Value {
    json!({
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
    })
}

/// Vocabulary policy: canonical naming rules for flags and verbs.
pub(crate) fn vocabulary_policy() -> Value {
    json!({
        "canonical_output_flag": "--emit",
        "canonical_output_values": ["text", "yaml", "postcard"],
        "canonical_destructive_bypass_flag": "--force",
        "canonical_resource_verbs": ["get", "list", "create", "update", "delete"],
        "banned_verbs": ["info", "ls"],
        "banned_flags": ["--json", "--jsonl", "--format=json", "--output=json", "--skip-confirmations"]
    })
}

/// Binary aliases for the CLI.
pub(crate) fn binary_aliases() -> Value {
    json!([CLI_NAME])
}
