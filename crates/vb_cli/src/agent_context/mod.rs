#![forbid(unsafe_code)]
#![cfg_attr(not(kani), allow(dead_code, unused_mut, unused_variables))]

use serde_json::{Map, Value};

/// Build the machine-readable CLI surface for AI agents.
pub(crate) fn build(version: &str) -> Value {
    serde_json::json!({
        "schema_version": "1",
        "kind": "AgentContext",
        "cli": "velvet-ballastics",
        "binary_aliases": ["velvet-ballastics"],
        "version": version,
        "language_version": "velvet-ballastics/v1",
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
        "commands": commands(),
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

fn commands() -> Value {
    Value::Object(Map::from_iter([
        command(
            "agent-context",
            serde_json::json!({
                "summary": "Emit this versioned machine-readable CLI schema.",
                "outputs": ["json"]
            }),
        ),
        command(
            "validate",
            serde_json::json!({
                "summary": "Validate a workflow definition.",
                "positionals": ["workflow.yaml"],
                "flags": output_flags()
            }),
        ),
        command(
            "verify",
            serde_json::json!({
                "summary": "Verify a workflow with a bounded profile.",
                "positionals": ["workflow.yaml"],
                "flags": {
                    "--profile": {"type": "enum", "values": ["quick", "standard", "full"], "default": "standard"},
                    "--emit": output_emit_flag()
                }
            }),
        ),
        command(
            "explain",
            serde_json::json!({
                "summary": "Explain validation errors in detail.",
                "positionals": ["workflow.yaml"],
                "flags": output_flags()
            }),
        ),
        command(
            "compile",
            serde_json::json!({
                "summary": "Compile a workflow to an artifact.",
                "positionals": ["workflow.yaml"],
                "flags": {
                    "--emit": {"type": "enum", "values": ["ir", "yaml", "postcard"], "required": true},
                    "--out": {"type": "path", "required": true}
                }
            }),
        ),
        command(
            "run",
            serde_json::json!({
                "summary": "Compile and execute a workflow.",
                "positionals": ["workflow.yaml"],
                "flags": {
                    "--input-bin": {"type": "path", "required": true},
                    "--durability": {"type": "enum", "values": ["strict", "journaled", "none"], "required": true},
                    "--db": "path",
                    "--step": "u16",
                    "--step-input": "path",
                    "--emit": output_emit_flag()
                }
            }),
        ),
        command(
            "run-compiled",
            serde_json::json!({
                "summary": "Execute a compiled workflow artifact.",
                "positionals": ["workflow.vbir"],
                "flags": {
                    "--input-bin": {"type": "path", "required": true},
                    "--durability": {"type": "enum", "values": ["strict", "journaled", "none"], "required": true},
                    "--db": "path",
                    "--emit": output_emit_flag()
                }
            }),
        ),
        command(
            "ipc-serve",
            serde_json::json!({
                "summary": "Start the bounded local binary IPC server.",
                "flags": {"--socket": {"type": "path", "required": true}, "--db": {"type": "path", "required": true}}
            }),
        ),
        command("inspect", run_id_db_command("Inspect a durable run.")),
        command(
            "events",
            run_id_db_command("List durable events for a run."),
        ),
        command(
            "replay",
            run_id_db_command("Replay a run from the journal."),
        ),
        command(
            "trace",
            run_id_db_command("Show step-by-step execution trace."),
        ),
        command(
            "retry",
            run_id_db_command("Retry a failed run from its last successful step."),
        ),
        command("resume", run_id_db_command("Resume a suspended run.")),
        command(
            "bench-run",
            serde_json::json!({
                "summary": "Benchmark a workflow fixture.",
                "positionals": ["workflow.yaml"],
                "flags": output_flags()
            }),
        ),
        command(
            "doctor",
            serde_json::json!({
                "summary": "Run diagnostic checks.",
                "flags": db_output_flags()
            }),
        ),
        command(
            "answer",
            serde_json::json!({
                "summary": "Answer a suspended ask step.",
                "positionals": ["run_id"],
                "flags": {"--step": {"type": "u16", "required": true}, "--value-file": {"type": "path", "required": true}, "--db": {"type": "path", "required": true}, "--emit": output_emit_flag()}
            }),
        ),
        command(
            "graph",
            serde_json::json!({
                "summary": "Output the control-flow graph.",
                "positionals": ["workflow.yaml"],
                "flags": output_flags()
            }),
        ),
        command(
            "diff",
            serde_json::json!({
                "summary": "Compare two durable runs.",
                "positionals": ["run_a", "run_b"],
                "flags": db_output_flags()
            }),
        ),
        command(
            "incident",
            run_id_db_command("Produce a black-box failure report."),
        ),
        command(
            "submit",
            serde_json::json!({
                "summary": "Submit a workflow run to durable storage.",
                "positionals": ["workflow.yaml"],
                "flags": {
                    "--input-bin": {"type": "path", "required": true},
                    "--db": {"type": "path", "required": true},
                    "--durability": {"type": "enum", "values": ["strict", "journaled", "none"], "required": true},
                    "--emit": output_emit_flag()
                }
            }),
        ),
        command(
            "simulate",
            serde_json::json!({
                "summary": "Dry-run a workflow without executing actions.",
                "positionals": ["workflow.yaml"],
                "flags": output_flags()
            }),
        ),
    ]))
}

fn command(name: &str, value: Value) -> (String, Value) {
    (name.to_owned(), value)
}

fn output_flags() -> Value {
    serde_json::json!({"--emit": output_emit_flag()})
}

fn db_output_flags() -> Value {
    serde_json::json!({"--db": {"type": "path", "required": true}, "--emit": output_emit_flag()})
}

fn output_emit_flag() -> Value {
    serde_json::json!({"type": "enum", "values": ["text", "yaml", "postcard"], "default": "text"})
}

fn run_id_db_command(summary: &str) -> Value {
    serde_json::json!({
        "summary": summary,
        "positionals": ["run_id"],
        "flags": db_output_flags()
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
