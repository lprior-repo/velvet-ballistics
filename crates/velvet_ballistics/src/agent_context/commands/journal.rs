use super::{
    Value, command, db_output_flags, event_status_flag, output_emit_flag, query_command, run_flags,
    run_id_db_command,
};

pub(super) fn definitions() -> impl Iterator<Item = (String, Value)> {
    [
        inspect(),
        events(),
        replay(),
        trace(),
        retry(),
        resume(),
        doctor(),
        answer(),
        diff(),
        incident(),
    ]
    .into_iter()
}

fn inspect() -> (String, Value) {
    command("inspect", run_id_db_command("Inspect a durable run."))
}

fn events() -> (String, Value) {
    command(
        "events",
        query_command(
            "List durable events for a run.",
            run_flags([
                ("--status", event_status_flag()),
                ("--limit", serde_json::json!({"type": "i64"})),
            ]),
        ),
    )
}

fn replay() -> (String, Value) {
    command(
        "replay",
        run_id_db_command("Replay a run from the journal."),
    )
}

fn trace() -> (String, Value) {
    command(
        "trace",
        query_command(
            "Show step-by-step execution trace.",
            run_flags([
                ("--step", serde_json::json!({"type": "u16"})),
                ("--action", serde_json::json!({"type": "u16"})),
                ("--status", event_status_flag()),
                ("--since-seq", serde_json::json!({"type": "u64"})),
                ("--until-seq", serde_json::json!({"type": "u64"})),
                ("--limit", serde_json::json!({"type": "usize"})),
            ]),
        ),
    )
}

fn retry() -> (String, Value) {
    command(
        "retry",
        query_command(
            "Retry a failed run from its last successful step.",
            run_flags([(
                "--step",
                serde_json::json!({"type": "u16", "required": true}),
            )]),
        ),
    )
}

fn resume() -> (String, Value) {
    command("resume", run_id_db_command("Resume a suspended run."))
}

fn doctor() -> (String, Value) {
    command(
        "doctor",
        serde_json::json!({
            "summary": "Run diagnostic checks.",
            "flags": db_output_flags()
        }),
    )
}

fn answer() -> (String, Value) {
    command(
        "answer",
        serde_json::json!({
            "summary": "Answer a suspended ask by its pending slot using --slot with --value.",
            "positionals": ["run_id"],
            "flags": {"--slot": {"type": "u16", "required": true}, "--value": {"type": "path", "required": true}, "--db": {"type": "path", "required": true}, "--emit": output_emit_flag()}
        }),
    )
}

fn diff() -> (String, Value) {
    command(
        "diff",
        serde_json::json!({
            "summary": "Compare two durable runs.",
            "positionals": ["run_a", "run_b"],
            "flags": db_output_flags(),
            "variants": [workflow_diff()]
        }),
    )
}

fn workflow_diff() -> Value {
    serde_json::json!({
        "summary": "Compare workflow definitions using exactly one baseline: --against old.yaml or --db path; these flags are mutually exclusive.",
        "positionals": ["workflow.yaml"],
        "flags": {
            "--against": {"type": "path"},
            "--db": {"type": "path"},
            "--emit": output_emit_flag()
        }
    })
}

fn incident() -> (String, Value) {
    command(
        "incident",
        run_id_db_command("Produce a black-box failure report."),
    )
}
