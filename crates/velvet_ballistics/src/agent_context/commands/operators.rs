use super::{Value, command, output_emit_flag, query_command, run_flags};

pub(super) fn definitions() -> impl Iterator<Item = (String, Value)> {
    [
        agent_context(),
        help(),
        version(),
        status(),
        system_status(),
        action_list(),
        action_inspect(),
        ai_context(),
        cancel(),
    ]
    .into_iter()
}

fn agent_context() -> (String, Value) {
    command(
        "agent-context",
        serde_json::json!({
            "summary": "Emit this versioned machine-readable CLI schema.",
            "outputs": ["json"]
        }),
    )
}

fn help() -> (String, Value) {
    command(
        "help",
        serde_json::json!({"summary": "Show command help.", "outputs": ["text"]}),
    )
}

fn version() -> (String, Value) {
    command(
        "version",
        serde_json::json!({"summary": "Show the CLI version.", "outputs": ["text"]}),
    )
}

fn status() -> (String, Value) {
    command(
        "status",
        serde_json::json!({
            "summary": "Report explicitly supplied local runtime counters.",
            "flags": {
                "--emit": status_emit_flag(),
                "--active-runs": {"type": "usize"},
                "--queue-depth": {"type": "usize"},
                "--trace-dropped": {"type": "u64"}
            }
        }),
    )
}

fn system_status() -> (String, Value) {
    command(
        "system status",
        serde_json::json!({
            "summary": "Run local system checks for the selected profile.",
            "flags": {
                "--emit": status_emit_flag(),
                "--profile": {"type": "enum", "values": ["quick", "standard", "full"], "default": "standard"},
                "--server": {"type": "enum", "values": ["none"], "default": "none"}
            }
        }),
    )
}

fn status_emit_flag() -> Value {
    serde_json::json!({"type": "enum", "values": ["text", "yaml"], "default": "text"})
}

fn action_list() -> (String, Value) {
    command(
        "action list",
        serde_json::json!({
            "summary": "List the selected action registry.", "flags": action_flags()
        }),
    )
}

fn action_inspect() -> (String, Value) {
    command(
        "action inspect",
        serde_json::json!({
            "summary": "Inspect a named action in the selected registry.",
            "positionals": ["action_name"], "flags": action_flags()
        }),
    )
}

fn action_flags() -> Value {
    serde_json::json!({
        "--emit": output_emit_flag(),
        "--registry": {"type": "enum", "values": ["registered", "empty", "uninitialized"], "default": "registered"}
    })
}

fn ai_context() -> (String, Value) {
    command(
        "ai-context",
        query_command(
            "Read the bounded diagnostic context for a durable run.",
            run_flags([]),
        ),
    )
}

fn cancel() -> (String, Value) {
    command(
        "cancel",
        query_command(
            "Cancel a durable run.",
            run_flags([(
                "--reason",
                serde_json::json!({"type": "string", "max_bytes": 256}),
            )]),
        ),
    )
}
