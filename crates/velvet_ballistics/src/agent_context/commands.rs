use serde_json::{Map, Value};

mod execution;
mod journal;
mod operators;
mod source;

pub(super) fn commands() -> Value {
    Value::Object(Map::from_iter(
        source::definitions()
            .chain(execution::definitions())
            .chain(journal::definitions())
            .chain(operators::definitions()),
    ))
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
    query_command(summary, db_output_flags())
}

fn query_command(summary: &str, flags: Value) -> Value {
    serde_json::json!({
        "summary": summary,
        "positionals": ["run_id"],
        "flags": flags
    })
}

fn run_flags<const N: usize>(additional: [(&str, Value); N]) -> Value {
    let flags = [
        (
            "--db",
            serde_json::json!({"type": "path", "required": true}),
        ),
        ("--emit", output_emit_flag()),
    ]
    .into_iter()
    .chain(additional);
    Value::Object(Map::from_iter(
        flags.map(|(name, value)| (name.to_owned(), value)),
    ))
}

fn event_status_flag() -> Value {
    serde_json::json!({"type": "enum", "values": ["pending", "active", "waiting_answer", "cancelled", "completed", "failed"]})
}
