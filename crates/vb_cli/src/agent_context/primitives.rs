#![forbid(unsafe_code)]

use serde_json::Value;

pub(crate) fn planned_agent_primitives() -> Value {
    serde_json::json!({
        "async_wait_flag": "--wait",
        "jobs_commands": ["jobs list", "jobs get", "jobs prune"],
        "profile_commands": ["profile save", "profile list", "profile show", "profile delete"],
        "delivery_flag": "--deliver",
        "feedback_command": "feedback"
    })
}

pub(crate) fn diff_command() -> Value {
    serde_json::json!({
        "summary": "Compare workflow definitions or durable runs.",
        "modes": {
            "workflow": {
                "summary": "Compare workflow semantics against another workflow definition.",
                "positionals": ["workflow.yaml"],
                "flags": {"--against": {"type": "path", "required": true}, "--emit": output_emit_flag()}
            },
            "durable_run": {
                "summary": "Compare two durable runs.",
                "positionals": ["run_a", "run_b"],
                "flags": {"--db": {"type": "path", "required": true}, "--emit": output_emit_flag()}
            }
        }
    })
}

fn output_emit_flag() -> Value {
    serde_json::json!({"type": "enum", "values": ["text", "yaml", "postcard"], "default": "text"})
}
