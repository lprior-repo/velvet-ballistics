#![forbid(unsafe_code)]
#![cfg_attr(not(kani), allow(dead_code, unused_mut, unused_variables))]

use serde_json::{json, Map, Value};

use super::flags::*;
use super::primitives;

/// All 30 CLI command definitions.
pub(crate) fn commands() -> Value {
    Value::Object(Map::from_iter([
        command(
            "agent-context",
            json!({
                "summary": "Emit this versioned machine-readable CLI schema.",
                "flags": {
                    "--deliver": deliver_flag()
                },
                "outputs": ["json"]
            }),
        ),
        command(
            "validate",
            json!({
                "summary": "Validate a workflow definition.",
                "positionals": ["workflow.yaml"],
                "flags": output_flags()
            }),
        ),
        command(
            "verify",
            json!({
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
            json!({
                "summary": "Dry-run a workflow and report the execution plan with semantic parity to run.",
                "positionals": ["workflow.yaml"],
                "flags": output_flags()
            }),
        ),
        command(
            "compile",
            json!({
                "summary": "Compile a workflow to an artifact.",
                "positionals": ["workflow.yaml"],
                "flags": {
                    "--emit": compile_emit_flag(),
                    "--out": {"type": "path", "required": true}
                }
            }),
        ),
        command(
            "run",
            json!({
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
            json!({
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
            json!({
                "summary": "Start the bounded local binary IPC server.",
                "flags": {"--socket": {"type": "path", "required": true}, "--db": {"type": "path", "required": true}}
            }),
        ),
        command("inspect", run_id_db_command("Inspect a durable run.")),
        command(
            "events",
            json!({
                "summary": "List durable events for a run.",
                "positionals": ["run_id"],
                "flags": events_flags()
            }),
        ),
        command(
            "replay",
            run_id_db_command("Replay a run from the journal."),
        ),
        command(
            "trace",
            json!({
                "summary": "Show step-by-step execution trace.",
                "positionals": ["run_id"],
                "flags": trace_flags()
            }),
        ),
        command(
            "retry",
            json!({
                "summary": "Retry a failed run from its last successful step.",
                "positionals": ["run_id"],
                "flags": retry_flags()
            }),
        ),
        command("resume", run_id_db_command("Resume a suspended run.")),
        command(
            "cancel",
            json!({
                "summary": "Cancel a durable run.",
                "positionals": ["run_id"],
                "flags": {
                    "--db": {"type": "path", "required": true},
                    "--reason": {
                        "type": "string",
                        "max_length": crate::args::run_ops::CANCEL_REASON_MAX_CHARS,
                        "length_unit": "characters"
                    },
                    "--emit": output_emit_flag()
                }
            }),
        ),
        command(
            "bench-run",
            json!({
                "summary": "Benchmark a workflow fixture.",
                "positionals": ["workflow.yaml"],
                "flags": output_flags()
            }),
        ),
        command(
            "doctor",
            json!({
                "summary": "Run diagnostic checks.",
                "flags": optional_db_output_flags()
            }),
        ),
        command(
            "answer",
            json!({
                "summary": "Answer a suspended ask step.",
                "positionals": ["run_id"],
                "flags": {"--slot": {"type": "u16", "required": true}, "--value": {"type": "path", "required": true, "format": "postcard-encoded SlotValue bytes"}, "--db": {"type": "path", "required": true}, "--emit": output_emit_flag()}
            }),
        ),
        command(
            "graph",
            json!({
                "summary": "Output the control-flow graph.",
                "positionals": ["workflow.yaml"],
                "flags": output_flags()
            }),
        ),
        command("diff", primitives::diff_command()),
        command(
            "incident",
            run_id_db_command("Produce a black-box failure report."),
        ),
        command(
            "submit",
            json!({
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
            json!({
                "summary": "Dry-run a workflow without executing actions.",
                "positionals": ["workflow.yaml"],
                "flags": output_flags()
            }),
        ),
        command(
            "help",
            json!({
                "summary": "Print this message.",
                "outputs": ["text"],
                "aliases": ["--help", "-h"]
            }),
        ),
        command(
            "version",
            json!({
                "summary": "Print version.",
                "outputs": ["text"],
                "aliases": ["--version", "-V"]
            }),
        ),
        command(
            "status",
            json!({
                "summary": "Report runtime shard status (with live Fjall probe when --db is supplied).",
                "flags": status_flags()
            }),
        ),
        command(
            "system status",
            json!({
                "summary": "Report bounded system health (probes Fjall when --db is supplied).",
                "flags": system_status_flags()
            }),
        ),
        command(
            "action list",
            json!({
                "summary": "List registered action contracts.",
                "flags": action_registry_flags()
            }),
        ),
        command(
            "action inspect",
            json!({
                "summary": "Show one registered action contract.",
                "positionals": ["action-name"],
                "flags": action_registry_flags()
            }),
        ),
        command(
            "ai-context",
            json!({
                "summary": "Emit machine-readable per-run context.",
                "positionals": ["run_id"],
                "flags": db_output_flags()
            }),
        ),
    ]))
}

fn command(name: &str, value: Value) -> (String, Value) {
    (name.to_owned(), value)
}
