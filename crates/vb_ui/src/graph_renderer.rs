use vb_core::ids::StepIdx;
use vb_core::workflow::CompiledNodeKind;

#[derive(Debug, Clone)]
pub struct NodeCard {
    pub step_idx: u16,
    pub step_name: String,
    pub kind_label: String,
    pub category: NodeCategory,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub header_color: [f32; 4],
    pub body_color: [f32; 4],
    pub border_color: [f32; 4],
    pub text_color: [f32; 4],
    pub badges: Vec<NodeBadge>,
    pub state_overlay: Option<StateOverlay>,
}

#[derive(Debug, Clone)]
pub struct NodeBadge {
    pub label: String,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeCategory {
    Data,
    External,
    Branch,
    Loop,
    Parallel,
    Suspend,
    Terminal,
    Error,
    Control,
}

#[derive(Debug, Clone)]
pub struct StateOverlay {
    pub state: OverlayState,
    pub glow_color: [f32; 4],
    pub glow_radius: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Waiting,
    Asking,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct EdgeLine {
    pub source_step: u16,
    pub target_step: u16,
    pub source_port: String,
    pub target_port: String,
    pub edge_type: EdgeType,
    pub color: [f32; 4],
    pub width: f32,
    pub dashed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    Normal,
    Branch,
    ErrorRoute,
    RetryRoute,
    Join,
    LoopBack,
}

pub fn classify_node(kind: &CompiledNodeKind) -> NodeCategory {
    match kind {
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. } => NodeCategory::Data,

        CompiledNodeKind::Do { .. } => NodeCategory::External,

        CompiledNodeKind::Choose { .. }
        | CompiledNodeKind::ChooseSlot { .. }
        | CompiledNodeKind::RetryCheck { .. } => NodeCategory::Branch,

        CompiledNodeKind::ForEachStart { .. }
        | CompiledNodeKind::ForEachNext { .. }
        | CompiledNodeKind::ForEachJoin { .. }
        | CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::CollectPage { .. }
        | CompiledNodeKind::CollectNext { .. }
        | CompiledNodeKind::CollectFinish { .. }
        | CompiledNodeKind::ReduceStart { .. }
        | CompiledNodeKind::ReduceNext { .. }
        | CompiledNodeKind::ReduceFinish { .. }
        | CompiledNodeKind::RepeatStart { .. }
        | CompiledNodeKind::RepeatAttempt { .. }
        | CompiledNodeKind::RepeatCheck { .. }
        | CompiledNodeKind::RepeatFinish { .. } => NodeCategory::Loop,

        CompiledNodeKind::TogetherStart { .. }
        | CompiledNodeKind::TogetherBranch { .. }
        | CompiledNodeKind::TogetherJoin { .. } => NodeCategory::Parallel,

        CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. } => NodeCategory::Suspend,

        CompiledNodeKind::Finish { .. } => NodeCategory::Terminal,

        CompiledNodeKind::ErrorHandler { .. } => NodeCategory::Error,

        CompiledNodeKind::Jump { .. } => NodeCategory::Control,
    }
}

pub fn node_header_color(category: NodeCategory) -> [f32; 4] {
    match category {
        NodeCategory::Data => [0.15, 0.15, 0.25, 1.0],
        NodeCategory::External => [0.6, 0.27, 0.0, 1.0],
        NodeCategory::Branch => [0.45, 0.18, 0.7, 1.0],
        NodeCategory::Loop => [0.1, 0.2, 0.5, 1.0],
        NodeCategory::Parallel => [0.1, 0.25, 0.55, 1.0],
        NodeCategory::Suspend => [0.12, 0.3, 0.15, 1.0],
        NodeCategory::Terminal => [0.0, 0.55, 0.45, 1.0],
        NodeCategory::Error => [0.5, 0.05, 0.12, 1.0],
        NodeCategory::Control => [0.2, 0.2, 0.2, 1.0],
    }
}

pub fn node_body_color(category: NodeCategory) -> [f32; 4] {
    let [r, g, b, a] = node_header_color(category);
    [r * 0.6, g * 0.6, b * 0.6, a]
}

pub fn kind_label(kind: &CompiledNodeKind) -> String {
    match kind {
        CompiledNodeKind::Nop => "Nop".into(),
        CompiledNodeKind::SetConst { .. } => "SetConst".into(),
        CompiledNodeKind::Copy { .. } => "Copy".into(),
        CompiledNodeKind::EvalExpr { .. } => "EvalExpr".into(),
        CompiledNodeKind::BuildObject { .. } => "BuildObject".into(),
        CompiledNodeKind::BuildList { .. } => "BuildList".into(),
        CompiledNodeKind::Do { action, .. } => format!("Do (action {})", action.get()),
        CompiledNodeKind::Choose { .. } => "Choose".into(),
        CompiledNodeKind::ChooseSlot { .. } => "ChooseSlot".into(),
        CompiledNodeKind::ForEachStart { .. } => "ForEach".into(),
        CompiledNodeKind::ForEachNext { .. } => "ForEachNext".into(),
        CompiledNodeKind::ForEachJoin { .. } => "ForEachJoin".into(),
        CompiledNodeKind::CollectStart { .. } => "Collect".into(),
        CompiledNodeKind::CollectPage { .. } => "CollectPage".into(),
        CompiledNodeKind::CollectNext { .. } => "CollectNext".into(),
        CompiledNodeKind::CollectFinish { .. } => "CollectFinish".into(),
        CompiledNodeKind::ReduceStart { .. } => "Reduce".into(),
        CompiledNodeKind::ReduceNext { .. } => "ReduceNext".into(),
        CompiledNodeKind::ReduceFinish { .. } => "ReduceFinish".into(),
        CompiledNodeKind::RepeatStart { .. } => "Repeat".into(),
        CompiledNodeKind::RepeatAttempt { .. } => "RepeatAttempt".into(),
        CompiledNodeKind::RepeatCheck { .. } => "RepeatCheck".into(),
        CompiledNodeKind::RepeatFinish { .. } => "RepeatFinish".into(),
        CompiledNodeKind::TogetherStart { .. } => "Together".into(),
        CompiledNodeKind::TogetherBranch { .. } => "TogetherBranch".into(),
        CompiledNodeKind::TogetherJoin { .. } => "TogetherJoin".into(),
        CompiledNodeKind::WaitUntil { .. } => "WaitUntil".into(),
        CompiledNodeKind::WaitEvent { .. } => "WaitEvent".into(),
        CompiledNodeKind::Ask { .. } => "Ask".into(),
        CompiledNodeKind::AskResume { .. } => "AskResume".into(),
        CompiledNodeKind::RetryCheck { .. } => "RetryCheck".into(),
        CompiledNodeKind::ErrorHandler { .. } => "ErrorHandler".into(),
        CompiledNodeKind::Jump { .. } => "Jump".into(),
        CompiledNodeKind::Finish { .. } => "Finish".into(),
    }
}

pub fn state_glow(state: OverlayState) -> ([f32; 4], f32) {
    match state {
        OverlayState::Running => ([0.0, 0.96, 1.0, 1.0], 4.0),
        OverlayState::Succeeded => ([0.22, 1.0, 0.08, 1.0], 3.0),
        OverlayState::Failed => ([1.0, 0.03, 0.23, 1.0], 6.0),
        OverlayState::Waiting => ([0.18, 0.42, 1.0, 1.0], 2.0),
        OverlayState::Asking => ([1.0, 0.9, 0.0, 1.0], 3.0),
        OverlayState::Skipped => ([0.33, 0.33, 0.47, 1.0], 0.0),
        OverlayState::Cancelled => ([0.33, 0.33, 0.47, 1.0], 0.0),
        OverlayState::Pending => ([0.16, 0.16, 0.29, 1.0], 0.0),
    }
}

pub fn extract_badges(kind: &CompiledNodeKind) -> Vec<NodeBadge> {
    let mut badges = Vec::new();
    if let CompiledNodeKind::Do { action, .. } = kind {
        badges.push(NodeBadge {
            label: format!("A{}", action.get()),
            color: [1.0, 0.42, 0.0, 1.0],
        });
    }
    badges
}

pub fn edge_color(edge_type: EdgeType) -> [f32; 4] {
    match edge_type {
        EdgeType::Normal => [0.42, 0.42, 0.58, 1.0],
        EdgeType::Branch => [0.69, 0.3, 1.0, 1.0],
        EdgeType::ErrorRoute => [1.0, 0.03, 0.23, 1.0],
        EdgeType::RetryRoute => [1.0, 0.42, 0.0, 1.0],
        EdgeType::Join => [0.18, 0.42, 1.0, 1.0],
        EdgeType::LoopBack => [0.0, 0.96, 1.0, 1.0],
    }
}
