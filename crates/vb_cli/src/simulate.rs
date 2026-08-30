//! Module: simulate — CLI entry point for static preflight.
//!
//! The `simulate` command is a **static preflight** analysis tool.
//! It compiles a workflow YAML to IR and performs a read-only structural
//! walk to produce a deterministic dry-run summary.
//!
//! ## Contract guarantees
//!
//! - **No execution**: Actions are never dispatched or run.
//! - **No storage**: The Fjall database is never opened or accessed.
//! - **No side effects**: No filesystem writes, no DB paths created.
//!   (Proven by test: `cli_simulate_does_not_create_db_side_effects`)
//! - **No I/O beyond IR input**: The only filesystem read is loading the
//!   compiled workflow bytes; all analysis happens in memory.
//!
//! ## Boundary vs. live runtime
//!
//! This command lives in `vb_cli` and depends only on `vb_compile` and
//! `vb_core::CompiledWorkflow`. It never imports or depends on `vb_runtime`
//! or `vb_storage`. The live execution path (`run`, `run-compiled`) goes
//! through `vb_runtime`'s multi-shard engine with full state management,
//! action dispatch, and journal persistence.
//!
//! The workflow analysis path is:
//! ```text
//! YAML file → vb_compile::compile_bytes_json() → CompiledWorkflow
//!           → commands_workflow::simulate_workflow() → SimulationResult
//!           → emit to stdout (text/yaml/postcard)
//! ```
//!
//! See `commands_workflow::simulate` module docs for the full boundary
//! comparison table between static preflight and live runtime.

use crate::app_impl::prelude::*;

pub(crate) fn cmd_simulate(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
    let bytes = match read_file(workflow, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match compile_bytes_json(&bytes, output) {
        Ok(c) => c,
        Err(code) => return code,
    };

    let result = commands_workflow::simulate_workflow(&compiled);

    if output != OutputFormat::Text {
        let trace: Vec<serde_json::Value> = result
            .steps
            .iter()
            .map(|s| {
                serde_json::json!({
                    "step": s.index,
                    "kind": s.kind_label,
                    "description": s.description,
                })
            })
            .collect();
        emit_json_or_return!(
            &serde_json::json!({
                "schema_version": "velvet-ballistics/v1",
                "kind": "simulate",
                "success": true,
                "total_steps": result.total_steps,
                "total_actions": result.action_count,
                "total_branches": result.branch_count,
                "trace": trace
            }),
            output,
        );
    } else {
        for step in &result.steps {
            outln!("Step {}: {}", step.index, step.description);
        }
        outln!("");
        outln!("simulation summary");
        outln!("  steps:    {}", result.total_steps);
        outln!("  actions:  {}", result.action_count);
        outln!("  branches: {}", result.branch_count);
        outln!("dry-run complete");
    }

    CliExitCode::Success.into()
}
