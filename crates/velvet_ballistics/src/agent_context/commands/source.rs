use super::{Value, command, output_emit_flag, output_flags};

pub(super) fn definitions() -> impl Iterator<Item = (String, Value)> {
    [
        validate(),
        verify(),
        explain(),
        compile(),
        graph(),
        simulate(),
        bench_run(),
    ]
    .into_iter()
}

fn validate() -> (String, Value) {
    command(
        "validate",
        serde_json::json!({
            "summary": "Validate a workflow definition.",
            "positionals": ["workflow.yaml"],
            "flags": output_flags()
        }),
    )
}

fn verify() -> (String, Value) {
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
    )
}

fn explain() -> (String, Value) {
    command(
        "explain",
        serde_json::json!({
            "summary": "Compile without execution and report the execution plan.",
            "positionals": ["workflow.yaml"],
            "flags": output_flags()
        }),
    )
}

fn compile() -> (String, Value) {
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
    )
}

fn graph() -> (String, Value) {
    command(
        "graph",
        serde_json::json!({
            "summary": "Output the control-flow graph.",
            "positionals": ["workflow.yaml"],
            "flags": output_flags()
        }),
    )
}

fn simulate() -> (String, Value) {
    command(
        "simulate",
        serde_json::json!({
            "summary": "Dry-run a workflow without executing actions.",
            "positionals": ["workflow.yaml"],
            "flags": output_flags()
        }),
    )
}

fn bench_run() -> (String, Value) {
    command(
        "bench-run",
        serde_json::json!({
            "summary": "Benchmark a workflow fixture.",
            "positionals": ["workflow.yaml"],
            "flags": output_flags()
        }),
    )
}
