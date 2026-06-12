#![forbid(unsafe_code)]
//! Pure workflow analysis logic for graph and simulate commands.

use vb_core::ids::StepIdx;
use vb_core::{CompiledNodeKind, CompiledWorkflow};

// DOT graph generation lives in `dot.rs` to keep this file under the
// 300-line source cap. The sub-module is included as a sibling so the
// `pub(crate)` items remain reachable through `crate::commands_workflow::*`.
mod dot;
pub(crate) use self::dot::{generate_dot, node_kind_label, DotGraph};

// ---------------------------------------------------------------------------
// Simulation dry-run
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn describe_node_for_simulate(
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
            *action_count = action_count.saturating_add(1);
            format!("Do action {} -- would execute action", action.get())
        }
        CompiledNodeKind::Choose { branches, .. } => {
            let count = branches.len();
            *branch_count = branch_count.saturating_add(count);
            format!("Choose ({count} branches)")
        }
        CompiledNodeKind::ChooseSlot { branches, .. } => {
            let count = branches.len();
            *branch_count = branch_count.saturating_add(count);
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
        CompiledNodeKind::Ask { .. } => "Ask -- would suspend".to_string(),
        CompiledNodeKind::AskResume { .. } => "Ask resume".to_string(),
        CompiledNodeKind::RetryCheck { .. } => "Retry check".to_string(),
        CompiledNodeKind::ErrorHandler { .. } => "Error handler".to_string(),
        CompiledNodeKind::Jump { .. } => "Jump".to_string(),
        CompiledNodeKind::Finish { .. } => "Finish".to_string(),
        _ => "Other".to_string(),
    }
}
