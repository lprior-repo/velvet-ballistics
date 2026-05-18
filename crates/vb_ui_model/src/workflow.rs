//! Workflow graph view types.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use vb_core::ids::{StepIdx, WorkflowDigest, WorkflowId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowGraphView {
    pub workflow_id: WorkflowId,
    pub workflow_digest: WorkflowDigest,
    pub nodes: Vec<WorkflowNodeView>,
    pub edges: Vec<WorkflowEdgeView>,
    #[serde(skip)]
    pub node_x: Vec<f32>,
    #[serde(skip)]
    pub node_y: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowNodeView {
    pub step_idx: StepIdx,
    pub label: String,
    pub kind: WorkflowNodeKind,
    pub input_slot_count: u16,
    pub output_slot_count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum WorkflowNodeKind {
    Sequence = 0,
    Parallel = 1,
    ForEach = 2,
    If = 3,
    Switch = 4,
    Do = 5,
    OnError = 6,
    Finish = 7,
    Start = 8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowEdgeView {
    pub from_step: StepIdx,
    pub to_step: StepIdx,
    pub label: Option<String>,
}
