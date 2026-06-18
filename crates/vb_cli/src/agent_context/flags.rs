#![forbid(unsafe_code)]
#![cfg_attr(not(kani), allow(dead_code, unused_mut, unused_variables))]

use serde_json::{Value, json};

/// Emit flag accepting text, yaml, or postcard with text default.
pub(crate) fn output_emit_flag() -> Value {
    json!({"type": "enum", "values": ["text", "yaml", "postcard"], "default": "text"})
}

/// Emit flag accepting only text or yaml with text default.
pub(crate) fn text_yaml_emit_flag() -> Value {
    json!({"type": "enum", "values": ["text", "yaml"], "default": "text"})
}

/// Compile emit flag: enum with ir, yaml, postcard — required.
pub(crate) fn compile_emit_flag() -> Value {
    json!({"type": "enum", "values": ["ir", "yaml", "postcard"], "required": true})
}

/// Deliver output format flag with accepted/active/refused forms.
pub(crate) fn deliver_flag() -> Value {
    json!({
        "type": "string",
        "required": false,
        "accepted_forms": ["stdout", "file:<absolute-path>", "webhook:<url>"],
        "supported_forms": ["stdout", "file:<absolute-path>"],
        "currently_refused_forms": ["webhook:<url>"]
    })
}

/// Generic flags: `--emit`.
pub(crate) fn output_flags() -> Value {
    json!({"--emit": output_emit_flag()})
}

/// DB + emit flags for commands that require persistent storage access.
pub(crate) fn db_output_flags() -> Value {
    json!({"--db": {"type": "path", "required": true}, "--emit": output_emit_flag()})
}

/// Optional DB + emit flags for commands where DB is not always required.
pub(crate) fn optional_db_output_flags() -> Value {
    json!({"--db": {"type": "path", "required": false}, "--emit": output_emit_flag()})
}

/// Event listing flags: db, status filter, limit, emit.
pub(crate) fn events_flags() -> Value {
    json!({
        "--db": {"type": "path", "required": true},
        "--status": event_status_flag(),
        "--limit": {"type": "i64", "required": false},
        "--emit": output_emit_flag()
    })
}

/// Retry flags: db, optional step filter, emit.
pub(crate) fn retry_flags() -> Value {
    json!({
        "--db": {"type": "path", "required": true},
        "--step": {"type": "u16", "required": false},
        "--emit": output_emit_flag()
    })
}

/// Trace flags: db, step, action, status, since-seq, until-seq, limit, emit.
pub(crate) fn trace_flags() -> Value {
    json!({
        "--db": {"type": "path", "required": true},
        "--step": {"type": "u16", "required": false},
        "--action": {"type": "u16", "required": false},
        "--status": trace_status_flag(),
        "--since-seq": {"type": "u64", "required": false},
        "--until-seq": {"type": "u64", "required": false},
        "--limit": {"type": "usize", "required": false},
        "--emit": output_emit_flag()
    })
}

/// Event status enum: pending, active, waiting_answer, cancelled, completed, failed.
pub(crate) fn event_status_flag() -> Value {
    json!({
        "type": "enum",
        "required": false,
        "values": ["pending", "active", "waiting_answer", "cancelled", "completed", "failed"]
    })
}

/// Trace status — same as event status.
pub(crate) fn trace_status_flag() -> Value {
    event_status_flag()
}

/// Status command flags: active-runs, queue-depth, trace-dropped, optional db, text/yaml emit.
pub(crate) fn status_flags() -> Value {
    let config = vb_runtime::shard::ShardConfig::default();
    json!({
        "--active-runs": {"type": "usize", "max": config.max_active_runs},
        "--queue-depth": {"type": "usize", "max": config.command_queue_capacity},
        "--trace-dropped": {"type": "u64"},
        "--db": {"type": "path", "required": false},
        "--emit": text_yaml_emit_flag()
    })
}

/// System status flags: profile, server, optional db, text/yaml emit.
pub(crate) fn system_status_flags() -> Value {
    json!({
        "--profile": {"type": "enum", "values": ["quick", "standard", "full"], "default": "standard"},
        "--server": {"type": "enum", "values": ["none"], "default": "none"},
        "--db": {"type": "path", "required": false},
        "--emit": text_yaml_emit_flag()
    })
}

/// Action registry flags: emit + registry enum.
pub(crate) fn action_registry_flags() -> Value {
    json!({
        "--emit": output_emit_flag(),
        "--registry": {
            "type": "enum",
            "values": ["registered", "empty", "uninitialized"],
            "default": "registered"
        }
    })
}

/// Build a command that takes run_id positional + db+emit flags.
pub(crate) fn run_id_db_command(summary: &str) -> Value {
    json!({
        "summary": summary,
        "positionals": ["run_id"],
        "flags": db_output_flags()
    })
}
