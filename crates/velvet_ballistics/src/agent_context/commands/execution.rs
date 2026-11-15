use super::{Value, command, output_emit_flag};

pub(super) fn definitions() -> impl Iterator<Item = (String, Value)> {
    [run(), run_compiled(), ipc_serve(), submit()].into_iter()
}

fn run() -> (String, Value) {
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
    )
}

fn run_compiled() -> (String, Value) {
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
    )
}

fn ipc_serve() -> (String, Value) {
    command(
        "ipc-serve",
        serde_json::json!({
            "summary": "Start the bounded local binary IPC server.",
            "flags": {"--socket": {"type": "path", "required": true}, "--db": {"type": "path", "required": true}}
        }),
    )
}

fn submit() -> (String, Value) {
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
    )
}
