use serde_json::{Map, Value};

pub(super) fn commands() -> Value {
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
