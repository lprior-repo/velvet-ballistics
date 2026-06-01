#![forbid(unsafe_code)]
//! Control flow graph generation command.

fn cmd_graph(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
    let bytes = match read_file(workflow, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match compile_bytes_json(&bytes, output) {
        Ok(c) => c,
        Err(code) => return code,
    };

    let graph = commands_workflow::generate_dot(&compiled);

    if output != OutputFormat::Text {
        emit_json_or_return!(
            &serde_json::json!({
                "format": "dot",
                "nodes": graph.node_count,
                "edges": graph.edge_count,
                "dot": graph.dot
            }),
            output,
        );
    } else {
        outln!("{}", graph.dot);
    }

    CliExitCode::Success.into()
}

