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
            run_query_command(
                "List durable events for a run.",
                serde_json::json!({
                    "--status": event_status_flag(),
                    "--limit": {"type": "i64"}
                }),
            ),
        ),
        command(
            "replay",
            run_id_db_command("Replay a run from the journal."),
        ),
        command(
            "trace",
            run_query_command(
                "Show step-by-step execution trace.",
                serde_json::json!({
                    "--step": {"type": "u16"},
                    "--action": {"type": "u16"},
                    "--status": event_status_flag(),
                    "--since-seq": {"type": "u64"},
                    "--until-seq": {"type": "u64"},
                    "--limit": {"type": "usize"}
                }),
            ),
        ),
        command(
            "retry",
            run_query_command(
                "Retry a failed run from its last successful step.",
                serde_json::json!({"--step": {"type": "u16", "required": true}}),
            ),
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
                "summary": "Answer a suspended ask by its pending slot using --slot with --value.",
                "positionals": ["run_id"],
                "flags": {"--slot": {"type": "u16", "required": true}, "--value": {"type": "path", "required": true}, "--db": {"type": "path", "required": true}, "--emit": output_emit_flag()}
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
                "flags": db_output_flags(),
                "variants": {"workflow": workflow_diff_variant()}
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

fn workflow_diff_variant() -> Value {
    serde_json::json!({
        "summary": "Compare a workflow definition against a baseline: textual, semantic and digest diff.",
        "positionals": ["workflow.yaml"],
        "flags": {
            "--against": {"type": "path", "baseline": true},
            "--db": {"type": "path", "baseline": true},
            "--emit": output_emit_flag()
        },
        "mutually_exclusive": [["--against", "--db"]],
        "requires_one_of": ["--against", "--db"],
        "exit_codes": {"0": "no semantic changes", "1": "semantic changes detected", "2": "error"}
    })
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

fn run_query_command(summary: &str, extra_flags: Value) -> Value {
    let mut command = run_id_db_command(summary);
    if let (Some(flags), Value::Object(extra)) = (command.get_mut("flags"), extra_flags)
        && let Some(flags) = flags.as_object_mut()
    {
        flags.extend(extra);
    }
    command
}

fn event_status_flag() -> Value {
    serde_json::json!({
        "type": "enum",
        "values": ["pending", "active", "waiting_answer", "cancelled", "completed", "failed"]
    })
}
