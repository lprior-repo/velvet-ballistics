#![forbid(unsafe_code)]
//! Pure workflow analysis logic for graph and simulate commands.

use serde::{Deserialize, Serialize};
use vb_core::ids::StepIdx;
use vb_core::{CompiledNodeKind, CompiledWorkflow};

// DOT graph generation lives in `dot.rs` to keep this file under the
// 300-line source cap. The sub-module is included as a sibling so the
// `pub(crate)` items remain reachable through `crate::commands_workflow::*`.
mod dot;
pub(crate) use self::dot::{DotGraph, generate_dot, node_kind_label};

// ---------------------------------------------------------------------------
// Simulation dry-run
// ---------------------------------------------------------------------------

/// Categorical kind of a simulated step, mirroring `CompiledNodeKind` so
/// downstream consumers can match on a stable, exhaustive set of variants
/// (with `Unknown` covering any future `non_exhaustive` additions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub(crate) enum StepKind {
    Nop,
    SetConst,
    Copy,
    EvalExpr,
    BuildObject,
    BuildList,
    Do,
    Choose,
    ChooseSlot,
    ForEachStart,
    ForEachNext,
    ForEachJoin,
    TogetherStart,
    TogetherBranch,
    TogetherJoin,
    CollectStart,
    CollectPage,
    CollectNext,
    CollectFinish,
    ReduceStart,
    ReduceNext,
    ReduceFinish,
    RepeatStart,
    RepeatAttempt,
    RepeatCheck,
    RepeatFinish,
    WaitUntil,
    WaitEvent,
    Ask,
    AskResume,
    RetryCheck,
    ErrorHandler,
    Jump,
    Finish,
    Unknown,
}

/// Map a `CompiledNodeKind` to its simulation `StepKind` counterpart.
///
/// `CompiledNodeKind` is `#[non_exhaustive]`, so any new upstream variant
/// falls through to `StepKind::Unknown` until the mapping is updated.
pub(crate) fn node_kind_to_step_kind(kind: &CompiledNodeKind) -> StepKind {
    match kind {
        CompiledNodeKind::Nop => StepKind::Nop,
        CompiledNodeKind::SetConst { .. } => StepKind::SetConst,
        CompiledNodeKind::Copy { .. } => StepKind::Copy,
        CompiledNodeKind::EvalExpr { .. } => StepKind::EvalExpr,
        CompiledNodeKind::BuildObject { .. } => StepKind::BuildObject,
        CompiledNodeKind::BuildList { .. } => StepKind::BuildList,
        CompiledNodeKind::Do { .. } => StepKind::Do,
        CompiledNodeKind::Choose { .. } => StepKind::Choose,
        CompiledNodeKind::ChooseSlot { .. } => StepKind::ChooseSlot,
        CompiledNodeKind::ForEachStart { .. } => StepKind::ForEachStart,
        CompiledNodeKind::ForEachNext { .. } => StepKind::ForEachNext,
        CompiledNodeKind::ForEachJoin { .. } => StepKind::ForEachJoin,
        CompiledNodeKind::TogetherStart { .. } => StepKind::TogetherStart,
        CompiledNodeKind::TogetherBranch { .. } => StepKind::TogetherBranch,
        CompiledNodeKind::TogetherJoin { .. } => StepKind::TogetherJoin,
        CompiledNodeKind::CollectStart { .. } => StepKind::CollectStart,
        CompiledNodeKind::CollectPage { .. } => StepKind::CollectPage,
        CompiledNodeKind::CollectNext { .. } => StepKind::CollectNext,
        CompiledNodeKind::CollectFinish { .. } => StepKind::CollectFinish,
        CompiledNodeKind::ReduceStart { .. } => StepKind::ReduceStart,
        CompiledNodeKind::ReduceNext { .. } => StepKind::ReduceNext,
        CompiledNodeKind::ReduceFinish { .. } => StepKind::ReduceFinish,
        CompiledNodeKind::RepeatStart { .. } => StepKind::RepeatStart,
        CompiledNodeKind::RepeatAttempt { .. } => StepKind::RepeatAttempt,
        CompiledNodeKind::RepeatCheck { .. } => StepKind::RepeatCheck,
        CompiledNodeKind::RepeatFinish { .. } => StepKind::RepeatFinish,
        CompiledNodeKind::WaitUntil { .. } => StepKind::WaitUntil,
        CompiledNodeKind::WaitEvent { .. } => StepKind::WaitEvent,
        CompiledNodeKind::Ask { .. } => StepKind::Ask,
        CompiledNodeKind::AskResume { .. } => StepKind::AskResume,
        CompiledNodeKind::RetryCheck { .. } => StepKind::RetryCheck,
        CompiledNodeKind::ErrorHandler { .. } => StepKind::ErrorHandler,
        CompiledNodeKind::Jump { .. } => StepKind::Jump,
        CompiledNodeKind::Finish { .. } => StepKind::Finish,
        // `CompiledNodeKind` is `#[non_exhaustive]`: any future variant
        // surfaces as `Unknown` rather than failing the simulation.
        _ => StepKind::Unknown,
    }
}

pub(crate) struct SimulationStep {
    pub index: usize,
    pub kind_label_text: String,
    pub kind: StepKind,
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

        let kind_label_text = node_kind_label(&node.kind).to_string();
        let kind = node_kind_to_step_kind(&node.kind);
        let description =
            describe_node_for_simulate(&node.kind, &mut action_count, &mut branch_count);

        steps.push(SimulationStep {
            index: i,
            kind_label_text,
            kind,
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

/// Saturating add that returns the new value, used instead of checked_add +
/// unwrap/or pattern.
fn saturating_add(a: usize, b: usize) -> usize {
    a.saturating_add(b)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
