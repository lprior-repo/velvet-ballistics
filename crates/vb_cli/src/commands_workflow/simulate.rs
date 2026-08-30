#![forbid(unsafe_code)]
//! Workflow simulation — static preflight analysis.
//!
//! This module provides **static preflight** analysis for compiled workflows.
//! It inspects the compiled IR to produce a deterministic dry-run summary
//! without executing any actions, mutating any state, or touching storage.
//!
//! ## Boundary: simulate vs. live runtime
//!
//! | Aspect | `simulate` (static preflight) | `run` / `run-compiled` (live runtime) |
//! |---|---|---|
//! | **Execution** | None — read-only IR walk | Full deterministic engine with action dispatch |
//! | **State mutation** | None | Frame writes, slot updates, taint propagation |
//! | **Storage access** | None — no DB open, no journal I/O | Fjall journal, WAL, persistence |
//! | **Side effects** | None (verified by test: `cli_simulate_does_not_create_db_side_effects`) | Durable events, run acceptance, action registry writes |
//! | **I/O** | None (file read only for the compiled IR input) | IPC, timers, external action dispatch |
//! | **Purpose** | Validate workflow structure, enumerate steps, count actions/branches before committing to a run | Execute the workflow against real input data |
//!
//! The `simulate` command lives in `vb_cli` as a **pure analysis** layer.
//! It depends only on `vb_core::CompiledWorkflow` — the immutable IR produced
//! by the compiler (`vb_compile`). It never touches `vb_runtime`, `vb_storage`,
//! or any system resources.
//!
//! ## Relationship to runtime preflight
//!
//! The runtime engine also performs **preflight checks** (see
//! `vb_runtime::shard::lifecycle::preflight_action_completion` and
//! `preflight_action_failure`), but these are *live execution preflight* —
//! they validate action tickets, output sizes, and taint constraints at the
//! moment an action completes during actual execution. This is fundamentally
//! different from the static preflight provided by this module:
//!
//! - **Static preflight** (this module): Analyzes workflow *structure* before any
//!   run starts. Answers "what would this workflow do?" without doing anything.
//! - **Live preflight** (`vb_runtime` shard lifecycle): Validates action
//!   completion inputs *during* execution. Ensures invariants hold at each
//!   action boundary. This is a safety gate, not an analysis tool.
//!
//! ## Invocation path
//!
//! ```text
//! CLI entry:  vb_cli::simulate::cmd_simulate()
//!             → vb_compile: compile_bytes_json()  (YAML → CompiledWorkflow)
//!             → vb_cli::commands_workflow::simulate_workflow()  (static IR walk)
//!             → output summary (text, yaml, or postcard)
//! ```
//!
//! No `vb_runtime`, no `vb_storage`, no network, no filesystem writes.

//! Workflow simulation data types for dry-run analysis.
//!
//! These types are consumed by `cmd_simulate` in the CLI and by
//! integration tests. They carry only structural information about
//! the compiled workflow — no runtime state, no action results,
//! no storage references.

use vb_core::ids::StepIdx;
use vb_core::{CompiledNodeKind, CompiledWorkflow};

use super::helpers::{node_kind_label, saturating_add};

pub(crate) struct SimulationStep {
    pub index: usize,
    pub kind_label: String,
    pub description: String,
}

pub(crate) struct SimulationResult {
    pub steps: Vec<SimulationStep>,
    pub total_steps: usize,
    pub action_count: usize,
    pub branch_count: usize,
}

pub(crate) fn simulate_workflow(workflow: &CompiledWorkflow) -> SimulationResult {
    let node_count = usize::from(workflow.node_count());
    let mut action_count: usize = 0;
    let mut branch_count: usize = 0;
    let mut steps: Vec<SimulationStep> = Vec::new();

    for i in 0..node_count {
        let step = StepIdx::new(u16::try_from(i).unwrap_or(u16::MAX));
        let node = match workflow.node(step) {
            Some(n) => n,
            None => continue,
        };

        let kind_label = node_kind_label(&node.kind).to_string();
        let description =
            describe_node_for_simulate(&node.kind, &mut action_count, &mut branch_count);

        steps.push(SimulationStep {
            index: i,
            kind_label,
            description,
        });
    }

    SimulationResult {
        total_steps: node_count,
        action_count,
        branch_count,
        steps,
    }
}

pub(super) fn describe_node_for_simulate(
    kind: &CompiledNodeKind,
    action_count: &mut usize,
    branch_count: &mut usize,
) -> String {
    match kind {
        CompiledNodeKind::Nop => "Entry".to_string(),
        CompiledNodeKind::SetConst { .. } => "Set constant value".to_string(),
        CompiledNodeKind::Copy { .. } => "Copy slot".to_string(),
        CompiledNodeKind::EvalExpr { .. } => "Evaluate expression".to_string(),
        CompiledNodeKind::BuildObject { fields } => {
            format!("Build object ({} fields)", fields.len())
        }
        CompiledNodeKind::BuildList { items } => {
            format!("Build list ({} items)", items.len())
        }
        CompiledNodeKind::Do { action, .. } => {
            *action_count = saturating_add(*action_count, 1);
            format!("Do action {} -- would execute action", action.get())
        }
        CompiledNodeKind::Choose { branches, .. } => {
            let count = branches.len();
            *branch_count = saturating_add(*branch_count, count);
            format!("Choose ({count} branches)")
        }
        CompiledNodeKind::ChooseSlot { branches, .. } => {
            let count = branches.len();
            *branch_count = saturating_add(*branch_count, count);
            format!("ChooseSlot ({count} branches)")
        }
        CompiledNodeKind::ForEachStart { limit, .. } => {
            format!("ForEach (limit {limit})")
        }
        CompiledNodeKind::ForEachNext { .. } => "ForEach advance".to_string(),
        CompiledNodeKind::ForEachJoin { .. } => "ForEach join".to_string(),
        CompiledNodeKind::TogetherStart { branches, .. } => {
            format!("Together ({} branches)", branches.len())
        }
        CompiledNodeKind::TogetherBranch { branch, .. } => {
            format!("Together branch {branch}")
        }
        CompiledNodeKind::TogetherJoin { .. } => "Together join".to_string(),
        CompiledNodeKind::CollectStart { limit, .. } => {
            format!("Collect (limit {limit})")
        }
        CompiledNodeKind::CollectPage { .. } => "Collect page".to_string(),
        CompiledNodeKind::CollectNext { .. } => "Collect next".to_string(),
        CompiledNodeKind::CollectFinish { .. } => "Collect finish".to_string(),
        CompiledNodeKind::ReduceStart { .. } => "Reduce start".to_string(),
        CompiledNodeKind::ReduceNext { .. } => "Reduce advance".to_string(),
        CompiledNodeKind::ReduceFinish { .. } => "Reduce finish".to_string(),
        CompiledNodeKind::RepeatStart { max_attempts, .. } => {
            format!("Repeat (max {max_attempts} attempts)")
        }
        CompiledNodeKind::RepeatAttempt { .. } => "Repeat attempt".to_string(),
        CompiledNodeKind::RepeatCheck { .. } => "Repeat check".to_string(),
        CompiledNodeKind::RepeatFinish { .. } => "Repeat finish".to_string(),
        CompiledNodeKind::WaitUntil { .. } => "WaitUntil -- would suspend".to_string(),
        CompiledNodeKind::WaitEvent { .. } => "WaitEvent -- would suspend".to_string(),
        CompiledNodeKind::Ask { .. } => "Ask -- would suspend for input".to_string(),
        CompiledNodeKind::AskResume { .. } => "AskResume".to_string(),
        CompiledNodeKind::RetryCheck { .. } => "RetryCheck".to_string(),
        CompiledNodeKind::ErrorHandler { .. } => "ErrorHandler".to_string(),
        CompiledNodeKind::Jump { .. } => "Jump".to_string(),
        CompiledNodeKind::Finish { .. } => "Finish -- would complete run".to_string(),
        _ => "Unknown".to_string(),
    }
}
