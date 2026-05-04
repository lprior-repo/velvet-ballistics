//! Node rendering data production for the flow editor canvas.
//!
//! This module is a pure-logic layer that takes a `FlowNodeRecord` and produces
//! structured render data (shapes, colors, badges, port positions, state
//! overlays) suitable for consumption by the Makepad GPU draw pipeline in
//! `flow_editor.rs`.
//!
//! Design goals:
//! - No Makepad dependencies -- fully testable with ordinary unit tests.
//! - Deterministic: same input always produces the same `NodeRenderData`.
//! - Kind-specific shapes/colors following the cyberpunk palette in `theme.rs`.

use flow_core::doc::{FlowNodeRecord, PortRole, PortSide};
use flow_core::ids::PortId;

use crate::draw;
use crate::theme;

// ---------------------------------------------------------------------------
// Shape enumeration
// ---------------------------------------------------------------------------

/// Visual shape for a workflow node on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeShape {
    /// Standard rounded rectangle (most node types).
    Rectangle,
    /// Diamond shape for branch decision nodes (Choose, ChooseSlot).
    Diamond,
    /// Hexagon for parallel nodes (Together*).
    Hexagon,
    /// Pill / stadium shape for suspend/wait nodes.
    Pill,
    /// Octagon for error handling nodes.
    Octagon,
    /// Circle for terminal nodes (Finish).
    Circle,
    /// Arrow / chevron for jump nodes.
    Arrow,
}

// ---------------------------------------------------------------------------
// Icon hint enumeration
// ---------------------------------------------------------------------------

/// Icon hint for a workflow node, used by the renderer to select an icon glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconHint {
    None,
    Data,
    Copy,
    Expression,
    Object,
    List,
    Action,
    Branch,
    Loop,
    Parallel,
    Retry,
    Wait,
    Ask,
    Error,
    Jump,
    Terminal,
    Nop,
}

// ---------------------------------------------------------------------------
// Badge
// ---------------------------------------------------------------------------

/// A small annotation badge rendered on a node (e.g. action ID, retry count).
#[derive(Debug, Clone, PartialEq)]
pub struct Badge {
    /// Badge display text (short, typically 1-3 chars).
    pub label: String,
    /// Badge background color.
    pub color: [f32; 4],
}

// ---------------------------------------------------------------------------
// Step state for overlay coloring
// ---------------------------------------------------------------------------

/// Execution state of a step, used to tint the node with state overlay colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Waiting,
    Asking,
    Cancelled,
    Secret,
}

impl StepState {
    /// Map a `StepState` to its cyberpunk neon color from the theme.
    #[must_use]
    pub fn color(self) -> [f32; 4] {
        match self {
            StepState::Pending => theme::colors::STATE_PENDING,
            StepState::Running => theme::colors::STATE_RUNNING,
            StepState::Succeeded => theme::colors::STATE_SUCCEEDED,
            StepState::Failed => theme::colors::STATE_FAILED,
            StepState::Waiting => theme::colors::STATE_WAITING,
            StepState::Asking => theme::colors::STATE_ASKING,
            StepState::Cancelled => theme::colors::STATE_CANCELLED,
            StepState::Secret => theme::colors::STATE_SECRET,
        }
    }
}

// ---------------------------------------------------------------------------
// Port position
// ---------------------------------------------------------------------------

/// A computed port position in world coordinates, ready for edge attachment.
#[derive(Debug, Clone, PartialEq)]
pub struct PortPosition {
    /// Port ID for matching with edge endpoints.
    pub id: PortId,
    /// World-space X coordinate.
    pub x: f64,
    /// World-space Y coordinate.
    pub y: f64,
    /// Whether this port is an input (left side).
    pub is_input: bool,
    /// Whether this port is an output (right side).
    pub is_output: bool,
}

// ---------------------------------------------------------------------------
// Node render data
// ---------------------------------------------------------------------------

/// Complete render data for a single node, produced by `NodeRenderer`.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeRenderData {
    /// Shape to draw.
    pub shape: NodeShape,
    /// Header fill color.
    pub header_color: [f32; 4],
    /// Body fill color.
    pub body_color: [f32; 4],
    /// Border color (stroke).
    pub border_color: [f32; 4],
    /// Text color for the title/label.
    pub text_color: [f32; 4],
    /// Width hint in world units.
    pub width_hint: f64,
    /// Height hint in world units.
    pub height_hint: f64,
    /// Badge annotations.
    pub badges: Vec<Badge>,
    /// Icon hint for the renderer.
    pub icon: IconHint,
    /// Computed port positions in world space.
    pub ports: Vec<PortPosition>,
    /// Optional state overlay tint (None means no overlay).
    pub state_overlay: Option<[f32; 4]>,
    /// Whether the node is hidden (should not be rendered).
    pub hidden: bool,
    /// Whether the node is locked (should render a lock indicator).
    pub locked: bool,
}

// ---------------------------------------------------------------------------
// Node category helpers
// ---------------------------------------------------------------------------

/// Category classification for a node kind string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeCategory {
    Data,
    External,
    Branch,
    Loop,
    Parallel,
    Suspend,
    Error,
    Terminal,
    Control,
    Unknown,
}

/// Classify a node kind string into a visual category.
#[must_use]
pub fn classify_kind(kind: &str) -> NodeCategory {
    match kind {
        "SetConst" | "set_const" | "Copy" | "copy" | "EvalExpr" | "eval_expr"
        | "BuildObject" | "build_object" | "BuildList" | "build_list" | "data" => {
            NodeCategory::Data
        }
        "Do" | "do" => NodeCategory::External,
        "Choose" | "choose" | "ChooseSlot" | "choose_slot" | "branch" => {
            NodeCategory::Branch
        }
        "ForEachStart" | "foreach_start" | "ForEachNext" | "foreach_next"
        | "ForEachJoin" | "foreach_join" | "CollectStart" | "collect_start"
        | "CollectPage" | "collect_page" | "CollectNext" | "collect_next"
        | "CollectFinish" | "collect_finish" | "ReduceStart" | "reduce_start"
        | "ReduceNext" | "reduce_next" | "ReduceFinish" | "reduce_finish"
        | "loop" => NodeCategory::Loop,
        "TogetherStart" | "together_start" | "TogetherBranch" | "together_branch"
        | "TogetherJoin" | "together_join" | "parallel" => NodeCategory::Parallel,
        "WaitUntil" | "wait_until" | "WaitEvent" | "wait_event" | "Ask" | "ask"
        | "AskResume" | "ask_resume" | "suspend" | "wait" => NodeCategory::Suspend,
        "ErrorHandler" | "error_handler" | "error" | "RetryCheck" | "retry_check"
        | "RepeatStart" | "repeat_start" | "RepeatAttempt" | "repeat_attempt"
        | "RepeatCheck" | "repeat_check" | "RepeatFinish" | "repeat_finish" => {
            NodeCategory::Error
        }
        "Finish" | "finish" | "terminal" => NodeCategory::Terminal,
        "Jump" | "jump" | "Nop" | "nop" | "control" => NodeCategory::Control,
        _ => NodeCategory::Unknown,
    }
}

// ---------------------------------------------------------------------------
// Default dimensions
// ---------------------------------------------------------------------------

/// Default node width for standard rectangle nodes.
pub const DEFAULT_WIDTH: f64 = 160.0;
/// Default node height for standard rectangle nodes.
pub const DEFAULT_HEIGHT: f64 = 60.0;
/// Diamond nodes are slightly wider.
pub const DIAMOND_WIDTH: f64 = 200.0;
/// Diamond height.
pub const DIAMOND_HEIGHT: f64 = 100.0;
/// Hexagon width for parallel constructs.
pub const HEXAGON_WIDTH: f64 = 180.0;
/// Hexagon height.
pub const HEXAGON_HEIGHT: f64 = 80.0;
/// Pill width for suspend nodes.
pub const PILL_WIDTH: f64 = 180.0;
/// Pill height.
pub const PILL_HEIGHT: f64 = 48.0;
/// Circle diameter for terminal nodes.
pub const CIRCLE_SIZE: f64 = 64.0;
/// Octagon width for error nodes.
pub const OCTAGON_WIDTH: f64 = 160.0;
/// Octagon height.
pub const OCTAGON_HEIGHT: f64 = 64.0;
/// Arrow width for jump nodes.
pub const ARROW_WIDTH: f64 = 140.0;
/// Arrow height.
pub const ARROW_HEIGHT: f64 = 48.0;

// ---------------------------------------------------------------------------
// NodeRenderer
// ---------------------------------------------------------------------------

/// Produces `NodeRenderData` from a `FlowNodeRecord`.
///
/// `NodeRenderer` is stateless and can be reused across multiple nodes.
/// It resolves the node kind to a shape, color palette, badge set, icon hint,
/// and computes port positions -- all without touching the GPU.
#[derive(Debug, Clone)]
pub struct NodeRenderer {
    /// Optional step state to overlay on nodes (e.g. from runtime execution).
    state: Option<StepState>,
}

impl NodeRenderer {
    /// Create a new renderer with no state overlay.
    #[must_use]
    pub fn new() -> Self {
        Self { state: None }
    }

    /// Create a renderer that applies the given step-state overlay.
    #[must_use]
    pub fn with_state(state: StepState) -> Self {
        Self {
            state: Some(state),
        }
    }

    /// Set or clear the state overlay for subsequent renders.
    pub fn set_state(&mut self, state: Option<StepState>) {
        self.state = state;
    }

    /// Produce complete render data for a single node.
    #[must_use]
    pub fn render(&self, node: &FlowNodeRecord) -> NodeRenderData {
        let cat = classify_kind(node.kind.as_str());

        let (shape, width_hint, height_hint) = Self::resolve_shape(&cat);
        let (header_color, body_color, border_color, text_color) = Self::resolve_colors(&cat);
        let badges = Self::resolve_badges(&cat, node);
        let icon = Self::resolve_icon(&cat);
        let ports = Self::compute_port_positions(node);
        let state_overlay = self.state.map(|s| s.color());

        NodeRenderData {
            shape,
            header_color: node.ui.color_override.unwrap_or(header_color),
            body_color,
            border_color,
            text_color,
            width_hint,
            height_hint,
            badges,
            icon,
            ports,
            state_overlay,
            hidden: node.flags.hidden,
            locked: node.flags.locked,
        }
    }

    // ---- Shape resolution ----

    fn resolve_shape(cat: &NodeCategory) -> (NodeShape, f64, f64) {
        match cat {
            NodeCategory::Branch => (NodeShape::Diamond, DIAMOND_WIDTH, DIAMOND_HEIGHT),
            NodeCategory::Parallel => (NodeShape::Hexagon, HEXAGON_WIDTH, HEXAGON_HEIGHT),
            NodeCategory::Suspend => (NodeShape::Pill, PILL_WIDTH, PILL_HEIGHT),
            NodeCategory::Terminal => (NodeShape::Circle, CIRCLE_SIZE, CIRCLE_SIZE),
            NodeCategory::Error => (NodeShape::Octagon, OCTAGON_WIDTH, OCTAGON_HEIGHT),
            NodeCategory::Control if true => {
                // Jump gets Arrow, Nop gets Rectangle -- distinguish by checking
                // the category is control. We refine below.
                (NodeShape::Rectangle, DEFAULT_WIDTH, DEFAULT_HEIGHT)
            }
            _ => (NodeShape::Rectangle, DEFAULT_WIDTH, DEFAULT_HEIGHT),
        }
    }

    /// Refine shape for specific control types (Jump = Arrow).
    #[must_use]
    pub fn refine_shape_for_kind(shape: NodeShape, kind: &str) -> NodeShape {
        match kind {
            "Jump" | "jump" => NodeShape::Arrow,
            _ => shape,
        }
    }

    // ---- Color resolution ----

    fn resolve_colors(
        cat: &NodeCategory,
    ) -> ([f32; 4], [f32; 4], [f32; 4], [f32; 4]) {
        match cat {
            NodeCategory::Data => (
                theme::colors::TEXT_SECONDARY,
                theme::colors::CARD_BG,
                theme::colors::BORDER,
                theme::colors::TEXT_PRIMARY,
            ),
            NodeCategory::External => (
                theme::colors::NEON_ORANGE,
                theme::colors::CARD_BG,
                theme::colors::NEON_ORANGE,
                theme::colors::TEXT_PRIMARY,
            ),
            NodeCategory::Branch => (
                theme::colors::NEON_PURPLE,
                theme::colors::CARD_BG,
                theme::colors::NEON_PURPLE,
                theme::colors::TEXT_PRIMARY,
            ),
            NodeCategory::Loop => (
                theme::colors::NEON_BLUE,
                theme::colors::CARD_BG,
                theme::colors::NEON_BLUE,
                theme::colors::TEXT_PRIMARY,
            ),
            NodeCategory::Parallel => (
                theme::colors::NEON_TEAL,
                theme::colors::CARD_BG,
                theme::colors::NEON_TEAL,
                theme::colors::TEXT_PRIMARY,
            ),
            NodeCategory::Suspend => (
                theme::colors::NEON_GREEN,
                theme::colors::CARD_BG,
                theme::colors::NEON_GREEN,
                theme::colors::TEXT_PRIMARY,
            ),
            NodeCategory::Error => (
                theme::colors::NEON_RED,
                theme::colors::CARD_BG,
                theme::colors::NEON_RED,
                theme::colors::TEXT_PRIMARY,
            ),
            NodeCategory::Terminal => (
                theme::colors::NEON_TEAL,
                theme::colors::CARD_BG,
                theme::colors::NEON_TEAL,
                theme::colors::TEXT_PRIMARY,
            ),
            NodeCategory::Control => (
                theme::colors::TEXT_SECONDARY,
                theme::colors::CARD_BG,
                theme::colors::BORDER,
                theme::colors::TEXT_DIM,
            ),
            NodeCategory::Unknown => (
                theme::colors::TEXT_SECONDARY,
                theme::colors::CARD_BG,
                theme::colors::BORDER,
                theme::colors::TEXT_PRIMARY,
            ),
        }
    }

    // ---- Badge resolution ----

    fn resolve_badges(cat: &NodeCategory, node: &FlowNodeRecord) -> Vec<Badge> {
        match cat {
            NodeCategory::External => {
                // Do nodes get an action badge derived from data
                let action_badge = Self::extract_action_badge(node);
                let mut badges = Vec::new();
                if let Some(b) = action_badge {
                    badges.push(b);
                }
                badges.push(Badge {
                    label: String::from("S"),
                    color: theme::colors::NEON_MAGENTA,
                });
                badges
            }
            NodeCategory::Error => {
                // RepeatStart gets retry count badge from data
                let retry_badge = Self::extract_retry_badge(node);
                match retry_badge {
                    Some(b) => vec![b],
                    None => Vec::new(),
                }
            }
            NodeCategory::Suspend => {
                // WaitEvent/Ask with timeout get a "T" badge
                if Self::has_timeout(node) {
                    vec![Badge {
                        label: String::from("T"),
                        color: theme::colors::NEON_RED,
                    }]
                } else {
                    Vec::new()
                }
            }
            NodeCategory::Terminal => {
                vec![Badge {
                    label: String::from("D"),
                    color: theme::colors::NEON_TEAL,
                }]
            }
            _ => Vec::new(),
        }
    }

    /// Try to extract an action badge ("A{n}") from node data.
    fn extract_action_badge(node: &FlowNodeRecord) -> Option<Badge> {
        let action_id = node.data.get("action_id").and_then(|v| v.as_u64())?;
        Some(Badge {
            label: format!("A{action_id}"),
            color: theme::colors::NEON_ORANGE,
        })
    }

    /// Try to extract a retry badge ("R{n}") from node data.
    fn extract_retry_badge(node: &FlowNodeRecord) -> Option<Badge> {
        let attempts = node.data.get("max_attempts").and_then(|v| v.as_u64())?;
        Some(Badge {
            label: format!("R{attempts}"),
            color: theme::colors::NEON_YELLOW,
        })
    }

    /// Check whether the node data indicates a timeout is configured.
    fn has_timeout(node: &FlowNodeRecord) -> bool {
        node.data
            .get("timeout_slot")
            .is_some_and(|v| !v.is_null())
            || node
                .data
                .get("has_timeout")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
    }

    // ---- Icon resolution ----

    fn resolve_icon(cat: &NodeCategory) -> IconHint {
        match cat {
            NodeCategory::Data => IconHint::Data,
            NodeCategory::External => IconHint::Action,
            NodeCategory::Branch => IconHint::Branch,
            NodeCategory::Loop => IconHint::Loop,
            NodeCategory::Parallel => IconHint::Parallel,
            NodeCategory::Suspend => IconHint::Wait,
            NodeCategory::Error => IconHint::Error,
            NodeCategory::Terminal => IconHint::Terminal,
            NodeCategory::Control => IconHint::Nop,
            NodeCategory::Unknown => IconHint::None,
        }
    }

    // ---- Port positions ----

    /// Compute world-space positions for all ports on a node.
    ///
    /// Input ports appear on the left edge, output ports on the right edge.
    /// Port vertical positions are computed from `port.order` and the
    /// standard spacing constants.
    #[allow(clippy::arithmetic_side_effects)]
    fn compute_port_positions(node: &FlowNodeRecord) -> Vec<PortPosition> {
        let header_h = draw::node::HEADER_HEIGHT;
        let padding = draw::node::PADDING;
        let port_height = draw::port::HEIGHT;

        let mut result = Vec::with_capacity(node.ports.len());

        for port in &node.ports {
            let port_y = node.position[1]
                + header_h
                + padding
                + f64::from(port.order) * port_height
                + port_height / 2.0;

            let is_input = port.role == PortRole::Target
                || port.role == PortRole::Bidirectional;
            let is_output = port.role == PortRole::Source
                || port.role == PortRole::Bidirectional;

            // Use port.side to determine horizontal placement, defaulting to
            // role-based placement if side is ambiguous.
            let port_x = if is_output && !is_input {
                // Output on right side
                node.position[0] + node.size[0]
            } else {
                // Input on left side (or bidirectional defaults to left)
                node.position[0]
            };

            // Override with explicit side if it's a right-side output
            let final_x = match port.side {
                PortSide::Right => node.position[0] + node.size[0],
                PortSide::Top | PortSide::Bottom => {
                    // For top/bottom ports, use horizontal center
                    node.position[0] + node.size[0] / 2.0
                }
                _ => port_x,
            };

            result.push(PortPosition {
                id: port.id.clone(),
                x: final_x,
                y: port_y,
                is_input,
                is_output,
            });
        }

        result
    }

    /// Convenience: compute port positions from a node record using the
    /// standard constants. Public wrapper around the internal method.
    #[must_use]
    pub fn port_positions(node: &FlowNodeRecord) -> Vec<PortPosition> {
        Self::compute_port_positions(node)
    }
}

impl Default for NodeRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Convenience: render a node with default state (no overlay)
// ---------------------------------------------------------------------------

/// One-shot render of a node with no state overlay.
/// Equivalent to `NodeRenderer::new().render(node)`.
#[must_use]
pub fn render_node(node: &FlowNodeRecord) -> NodeRenderData {
    NodeRenderer::new().render(node)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use flow_core::doc::{Cardinality, FlowPortRecord, NodeFlags, NodeUiState};
    use flow_core::ids::NodeId;
    use smol_str::SmolStr;

    // ---- helpers ----

    fn nid(s: &str) -> NodeId {
        SmolStr::from(s)
    }

    fn pid(s: &str) -> PortId {
        SmolStr::from(s)
    }

    fn make_node(id: &str, kind: &str) -> FlowNodeRecord {
        FlowNodeRecord {
            id: nid(id),
            kind: SmolStr::from(kind),
            title: SmolStr::from(id),
            position: [100.0, 200.0],
            size: [160.0, 60.0],
            z_index: 0,
            parent: None,
            ports: Vec::new(),
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        }
    }

    fn make_node_with_data(
        id: &str,
        kind: &str,
        data: serde_json::Value,
    ) -> FlowNodeRecord {
        FlowNodeRecord {
            data,
            ..make_node(id, kind)
        }
    }

    fn make_port(id: &str, side: PortSide, role: PortRole, order: u16) -> FlowPortRecord {
        FlowPortRecord {
            id: pid(id),
            side,
            role,
            label: SmolStr::from(id),
            order,
            cardinality: Cardinality::One,
            data_type: None,
        }
    }

    fn make_node_with_ports(
        id: &str,
        kind: &str,
        ports: Vec<FlowPortRecord>,
    ) -> FlowNodeRecord {
        FlowNodeRecord {
            ports,
            ..make_node(id, kind)
        }
    }

    // ---- classify_kind tests ----

    #[test]
    fn classify_do_is_external() {
        assert_eq!(classify_kind("Do"), NodeCategory::External);
    }

    #[test]
    fn classify_do_lowercase() {
        assert_eq!(classify_kind("do"), NodeCategory::External);
    }

    #[test]
    fn classify_choose_is_branch() {
        assert_eq!(classify_kind("Choose"), NodeCategory::Branch);
    }

    #[test]
    fn classify_choose_slot_is_branch() {
        assert_eq!(classify_kind("ChooseSlot"), NodeCategory::Branch);
    }

    #[test]
    fn classify_foreach_start_is_loop() {
        assert_eq!(classify_kind("ForEachStart"), NodeCategory::Loop);
    }

    #[test]
    fn classify_together_start_is_parallel() {
        assert_eq!(classify_kind("TogetherStart"), NodeCategory::Parallel);
    }

    #[test]
    fn classify_wait_until_is_suspend() {
        assert_eq!(classify_kind("WaitUntil"), NodeCategory::Suspend);
    }

    #[test]
    fn classify_ask_is_suspend() {
        assert_eq!(classify_kind("Ask"), NodeCategory::Suspend);
    }

    #[test]
    fn classify_error_handler_is_error() {
        assert_eq!(classify_kind("ErrorHandler"), NodeCategory::Error);
    }

    #[test]
    fn classify_repeat_start_is_error() {
        assert_eq!(classify_kind("RepeatStart"), NodeCategory::Error);
    }

    #[test]
    fn classify_finish_is_terminal() {
        assert_eq!(classify_kind("Finish"), NodeCategory::Terminal);
    }

    #[test]
    fn classify_jump_is_control() {
        assert_eq!(classify_kind("Jump"), NodeCategory::Control);
    }

    #[test]
    fn classify_nop_is_control() {
        assert_eq!(classify_kind("Nop"), NodeCategory::Control);
    }

    #[test]
    fn classify_unknown() {
        assert_eq!(classify_kind("WidgetFrobnicator"), NodeCategory::Unknown);
    }

    #[test]
    fn classify_collect_start_is_loop() {
        assert_eq!(classify_kind("CollectStart"), NodeCategory::Loop);
    }

    #[test]
    fn classify_reduce_start_is_loop() {
        assert_eq!(classify_kind("ReduceStart"), NodeCategory::Loop);
    }

    #[test]
    fn classify_set_const_is_data() {
        assert_eq!(classify_kind("SetConst"), NodeCategory::Data);
    }

    #[test]
    fn classify_build_object_is_data() {
        assert_eq!(classify_kind("BuildObject"), NodeCategory::Data);
    }

    // ---- NodeRenderer basic rendering ----

    #[test]
    fn renderer_new_has_no_state_overlay() {
        let renderer = NodeRenderer::new();
        assert!(renderer.state.is_none());
    }

    #[test]
    fn renderer_default_is_new() {
        let renderer = NodeRenderer::default();
        assert!(renderer.state.is_none());
    }

    #[test]
    fn render_do_node_is_rectangle() {
        let node = make_node("n1", "Do");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.shape, NodeShape::Rectangle);
    }

    #[test]
    fn render_choose_node_is_diamond() {
        let node = make_node("n1", "Choose");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.shape, NodeShape::Diamond);
    }

    #[test]
    fn render_together_start_is_hexagon() {
        let node = make_node("n1", "TogetherStart");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.shape, NodeShape::Hexagon);
    }

    #[test]
    fn render_wait_until_is_pill() {
        let node = make_node("n1", "WaitUntil");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.shape, NodeShape::Pill);
    }

    #[test]
    fn render_ask_is_pill() {
        let node = make_node("n1", "Ask");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.shape, NodeShape::Pill);
    }

    #[test]
    fn render_error_handler_is_octagon() {
        let node = make_node("n1", "ErrorHandler");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.shape, NodeShape::Octagon);
    }

    #[test]
    fn render_finish_is_circle() {
        let node = make_node("n1", "Finish");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.shape, NodeShape::Circle);
    }

    #[test]
    fn render_jump_is_arrow_after_refinement() {
        let node = make_node("n1", "Jump");
        let data = NodeRenderer::new().render(&node);
        // The base shape is Rectangle for Control; refine for Jump
        let refined = NodeRenderer::refine_shape_for_kind(data.shape, "Jump");
        assert_eq!(refined, NodeShape::Arrow);
    }

    #[test]
    fn render_nop_is_rectangle() {
        let node = make_node("n1", "Nop");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.shape, NodeShape::Rectangle);
    }

    // ---- Color tests ----

    #[test]
    fn render_do_border_is_orange() {
        let node = make_node("n1", "Do");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.border_color, theme::colors::NEON_ORANGE);
    }

    #[test]
    fn render_choose_border_is_purple() {
        let node = make_node("n1", "Choose");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.border_color, theme::colors::NEON_PURPLE);
    }

    #[test]
    fn render_foreach_border_is_blue() {
        let node = make_node("n1", "ForEachStart");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.border_color, theme::colors::NEON_BLUE);
    }

    #[test]
    fn render_together_border_is_teal() {
        let node = make_node("n1", "TogetherStart");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.border_color, theme::colors::NEON_TEAL);
    }

    #[test]
    fn render_error_handler_border_is_red() {
        let node = make_node("n1", "ErrorHandler");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.border_color, theme::colors::NEON_RED);
    }

    #[test]
    fn render_finish_border_is_teal() {
        let node = make_node("n1", "Finish");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.border_color, theme::colors::NEON_TEAL);
    }

    #[test]
    fn render_nop_text_is_dim() {
        let node = make_node("n1", "Nop");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.text_color, theme::colors::TEXT_DIM);
    }

    // ---- Badge tests ----

    #[test]
    fn render_do_with_action_id_has_action_badge() {
        let data_val = serde_json::json!({"action_id": 42});
        let node = make_node_with_data("n1", "Do", data_val);
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.badges.len(), 2);
        assert_eq!(data.badges[0].label, "A42");
        assert_eq!(data.badges[1].label, "S");
    }

    #[test]
    fn render_do_without_action_id_still_has_secret_badge() {
        let node = make_node("n1", "Do");
        let data = NodeRenderer::new().render(&node);
        // No action_id in data, so only the "S" badge
        assert_eq!(data.badges.len(), 1);
        assert_eq!(data.badges[0].label, "S");
    }

    #[test]
    fn render_repeat_start_with_max_attempts_has_retry_badge() {
        let data_val = serde_json::json!({"max_attempts": 5});
        let node = make_node_with_data("n1", "RepeatStart", data_val);
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.badges.len(), 1);
        assert_eq!(data.badges[0].label, "R5");
    }

    #[test]
    fn render_repeat_start_without_max_attempts_has_no_badge() {
        let node = make_node("n1", "RepeatStart");
        let data = NodeRenderer::new().render(&node);
        assert!(data.badges.is_empty());
    }

    #[test]
    fn render_wait_with_timeout_has_timeout_badge() {
        let data_val = serde_json::json!({"timeout_slot": "slot_1"});
        let node = make_node_with_data("n1", "WaitEvent", data_val);
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.badges.len(), 1);
        assert_eq!(data.badges[0].label, "T");
    }

    #[test]
    fn render_ask_with_timeout_has_timeout_badge() {
        let data_val = serde_json::json!({"has_timeout": true});
        let node = make_node_with_data("n1", "Ask", data_val);
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.badges.len(), 1);
        assert_eq!(data.badges[0].label, "T");
    }

    #[test]
    fn render_wait_without_timeout_no_badge() {
        let node = make_node("n1", "WaitEvent");
        let data = NodeRenderer::new().render(&node);
        assert!(data.badges.is_empty());
    }

    #[test]
    fn render_finish_has_durable_badge() {
        let node = make_node("n1", "Finish");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.badges.len(), 1);
        assert_eq!(data.badges[0].label, "D");
    }

    #[test]
    fn render_nop_no_badges() {
        let node = make_node("n1", "Nop");
        let data = NodeRenderer::new().render(&node);
        assert!(data.badges.is_empty());
    }

    // ---- Icon tests ----

    #[test]
    fn render_do_icon_is_action() {
        let node = make_node("n1", "Do");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.icon, IconHint::Action);
    }

    #[test]
    fn render_choose_icon_is_branch() {
        let node = make_node("n1", "Choose");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.icon, IconHint::Branch);
    }

    #[test]
    fn render_foreach_icon_is_loop() {
        let node = make_node("n1", "ForEachStart");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.icon, IconHint::Loop);
    }

    #[test]
    fn render_together_icon_is_parallel() {
        let node = make_node("n1", "TogetherStart");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.icon, IconHint::Parallel);
    }

    #[test]
    fn render_error_handler_icon_is_error() {
        let node = make_node("n1", "ErrorHandler");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.icon, IconHint::Error);
    }

    #[test]
    fn render_finish_icon_is_terminal() {
        let node = make_node("n1", "Finish");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.icon, IconHint::Terminal);
    }

    #[test]
    fn render_jump_icon_is_nop() {
        // Jump is in Control category, which maps to Nop icon
        let node = make_node("n1", "Jump");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.icon, IconHint::Nop);
    }

    #[test]
    fn render_unknown_icon_is_none() {
        let node = make_node("n1", "MysteriousNode");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.icon, IconHint::None);
    }

    // ---- State overlay tests ----

    #[test]
    fn render_with_running_state_overlay() {
        let node = make_node("n1", "Do");
        let renderer = NodeRenderer::with_state(StepState::Running);
        let data = renderer.render(&node);
        assert_eq!(data.state_overlay, Some(theme::colors::STATE_RUNNING));
    }

    #[test]
    fn render_with_succeeded_state_overlay() {
        let node = make_node("n1", "Do");
        let renderer = NodeRenderer::with_state(StepState::Succeeded);
        let data = renderer.render(&node);
        assert_eq!(data.state_overlay, Some(theme::colors::STATE_SUCCEEDED));
    }

    #[test]
    fn render_with_failed_state_overlay() {
        let node = make_node("n1", "Do");
        let renderer = NodeRenderer::with_state(StepState::Failed);
        let data = renderer.render(&node);
        assert_eq!(data.state_overlay, Some(theme::colors::STATE_FAILED));
    }

    #[test]
    fn render_no_state_overlay_by_default() {
        let node = make_node("n1", "Do");
        let data = NodeRenderer::new().render(&node);
        assert!(data.state_overlay.is_none());
    }

    #[test]
    fn render_set_state_updates_overlay() {
        let mut renderer = NodeRenderer::new();
        assert!(renderer.state.is_none());
        renderer.set_state(Some(StepState::Failed));
        assert_eq!(renderer.state, Some(StepState::Failed));
        renderer.set_state(None);
        assert!(renderer.state.is_none());
    }

    // ---- StepState color tests ----

    #[test]
    fn step_state_pending_color() {
        assert_eq!(StepState::Pending.color(), theme::colors::STATE_PENDING);
    }

    #[test]
    fn step_state_running_color() {
        assert_eq!(StepState::Running.color(), theme::colors::STATE_RUNNING);
    }

    #[test]
    fn step_state_succeeded_color() {
        assert_eq!(StepState::Succeeded.color(), theme::colors::STATE_SUCCEEDED);
    }

    #[test]
    fn step_state_failed_color() {
        assert_eq!(StepState::Failed.color(), theme::colors::STATE_FAILED);
    }

    #[test]
    fn step_state_waiting_color() {
        assert_eq!(StepState::Waiting.color(), theme::colors::STATE_WAITING);
    }

    #[test]
    fn step_state_asking_color() {
        assert_eq!(StepState::Asking.color(), theme::colors::STATE_ASKING);
    }

    #[test]
    fn step_state_cancelled_color() {
        assert_eq!(StepState::Cancelled.color(), theme::colors::STATE_CANCELLED);
    }

    #[test]
    fn step_state_secret_color() {
        assert_eq!(StepState::Secret.color(), theme::colors::STATE_SECRET);
    }

    // ---- Port position tests ----

    #[test]
    fn port_positions_empty_ports() {
        let node = make_node("n1", "Do");
        let ports = NodeRenderer::port_positions(&node);
        assert!(ports.is_empty());
    }

    #[test]
    fn port_positions_input_on_left() {
        let port = make_port("in0", PortSide::Left, PortRole::Target, 0);
        let node = make_node_with_ports("n1", "Do", vec![port]);
        let ports = NodeRenderer::port_positions(&node);
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].id, pid("in0"));
        assert!(ports[0].is_input);
        assert!(!ports[0].is_output);
        // Input on left edge = node.x
        assert!((ports[0].x - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn port_positions_output_on_right() {
        let port = make_port("out0", PortSide::Right, PortRole::Source, 0);
        let node = make_node_with_ports("n1", "Do", vec![port]);
        let ports = NodeRenderer::port_positions(&node);
        assert_eq!(ports.len(), 1);
        assert!(!ports[0].is_input);
        assert!(ports[0].is_output);
        // Output on right edge = node.x + node.width
        assert!((ports[0].x - 260.0).abs() < f64::EPSILON);
    }

    #[test]
    fn port_positions_bidirectional() {
        let port = make_port("bio", PortSide::Left, PortRole::Bidirectional, 0);
        let node = make_node_with_ports("n1", "Do", vec![port]);
        let ports = NodeRenderer::port_positions(&node);
        assert_eq!(ports.len(), 1);
        assert!(ports[0].is_input);
        assert!(ports[0].is_output);
    }

    #[test]
    fn port_positions_vertical_spacing() {
        let p0 = make_port("in0", PortSide::Left, PortRole::Target, 0);
        let p1 = make_port("in1", PortSide::Left, PortRole::Target, 1);
        let node = make_node_with_ports("n1", "Do", vec![p0, p1]);
        let ports = NodeRenderer::port_positions(&node);
        assert_eq!(ports.len(), 2);
        // Vertical gap between order 0 and order 1 is port::HEIGHT
        let expected_gap = draw::port::HEIGHT;
        assert!((ports[1].y - ports[0].y - expected_gap).abs() < f64::EPSILON);
    }

    #[test]
    fn port_positions_top_side_uses_center_x() {
        let port = make_port("top0", PortSide::Top, PortRole::Target, 0);
        let node = make_node_with_ports("n1", "Do", vec![port]);
        let ports = NodeRenderer::port_positions(&node);
        // Top port: x = node.x + node.width / 2 = 100 + 80 = 180
        assert!((ports[0].x - 180.0).abs() < f64::EPSILON);
    }

    // ---- Node flags tests ----

    #[test]
    fn render_hidden_node() {
        let node = FlowNodeRecord {
            flags: NodeFlags {
                hidden: true,
                ..NodeFlags::default()
            },
            ..make_node("n1", "Do")
        };
        let data = NodeRenderer::new().render(&node);
        assert!(data.hidden);
    }

    #[test]
    fn render_locked_node() {
        let node = FlowNodeRecord {
            flags: NodeFlags {
                locked: true,
                ..NodeFlags::default()
            },
            ..make_node("n1", "Do")
        };
        let data = NodeRenderer::new().render(&node);
        assert!(data.locked);
    }

    #[test]
    fn render_visible_unlocked_node() {
        let node = make_node("n1", "Do");
        let data = NodeRenderer::new().render(&node);
        assert!(!data.hidden);
        assert!(!data.locked);
    }

    // ---- Color override test ----

    #[test]
    fn render_color_override_applies_to_header() {
        let custom_color = [0.5, 0.5, 0.5, 1.0];
        let node = FlowNodeRecord {
            ui: NodeUiState {
                color_override: Some(custom_color),
                ..NodeUiState::default()
            },
            ..make_node("n1", "Do")
        };
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.header_color, custom_color);
        // Border should still be category color
        assert_eq!(data.border_color, theme::colors::NEON_ORANGE);
    }

    // ---- Shape dimension tests ----

    #[test]
    fn diamond_dimensions() {
        let node = make_node("n1", "Choose");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.width_hint, DIAMOND_WIDTH);
        assert_eq!(data.height_hint, DIAMOND_HEIGHT);
    }

    #[test]
    fn hexagon_dimensions() {
        let node = make_node("n1", "TogetherStart");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.width_hint, HEXAGON_WIDTH);
        assert_eq!(data.height_hint, HEXAGON_HEIGHT);
    }

    #[test]
    fn pill_dimensions() {
        let node = make_node("n1", "WaitUntil");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.width_hint, PILL_WIDTH);
        assert_eq!(data.height_hint, PILL_HEIGHT);
    }

    #[test]
    fn circle_dimensions() {
        let node = make_node("n1", "Finish");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.width_hint, CIRCLE_SIZE);
        assert_eq!(data.height_hint, CIRCLE_SIZE);
    }

    #[test]
    fn octagon_dimensions() {
        let node = make_node("n1", "ErrorHandler");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.width_hint, OCTAGON_WIDTH);
        assert_eq!(data.height_hint, OCTAGON_HEIGHT);
    }

    #[test]
    fn rectangle_default_dimensions() {
        let node = make_node("n1", "Do");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.width_hint, DEFAULT_WIDTH);
        assert_eq!(data.height_hint, DEFAULT_HEIGHT);
    }

    // ---- render_node convenience function ----

    #[test]
    fn convenience_render_node() {
        let node = make_node("n1", "Do");
        let data = render_node(&node);
        assert_eq!(data.shape, NodeShape::Rectangle);
        assert_eq!(data.border_color, theme::colors::NEON_ORANGE);
    }

    // ---- All colors have positive alpha ----

    #[test]
    fn all_category_colors_have_positive_alpha() {
        let categories = [
            NodeCategory::Data,
            NodeCategory::External,
            NodeCategory::Branch,
            NodeCategory::Loop,
            NodeCategory::Parallel,
            NodeCategory::Suspend,
            NodeCategory::Error,
            NodeCategory::Terminal,
            NodeCategory::Control,
            NodeCategory::Unknown,
        ];
        for cat in &categories {
            let (h, b, brd, txt) = NodeRenderer::resolve_colors(cat);
            assert!(h[3] > 0.0, "header alpha for {cat:?}");
            assert!(b[3] > 0.0, "body alpha for {cat:?}");
            assert!(brd[3] > 0.0, "border alpha for {cat:?}");
            assert!(txt[3] > 0.0, "text alpha for {cat:?}");
        }
    }

    // ---- Multiple ports ordering ----

    #[test]
    fn port_positions_multiple_ports_correct_order() {
        let p0 = make_port("in0", PortSide::Left, PortRole::Target, 0);
        let p2 = make_port("in2", PortSide::Left, PortRole::Target, 2);
        let p1 = make_port("in1", PortSide::Left, PortRole::Target, 1);
        // Ports are in vec order, but their `order` field determines Y position
        let node = make_node_with_ports("n1", "Do", vec![p0, p2, p1]);
        let ports = NodeRenderer::port_positions(&node);
        assert_eq!(ports.len(), 3);
        // port with order=2 should be below port with order=0
        assert!(ports[1].y > ports[0].y); // in2 (order 2) > in0 (order 0)
        assert!(ports[2].y > ports[0].y); // in1 (order 1) > in0 (order 0)
    }

    // ---- All state colors are distinct from each other ----

    #[test]
    fn all_step_state_colors_are_distinct() {
        let states = [
            StepState::Pending,
            StepState::Running,
            StepState::Succeeded,
            StepState::Failed,
            StepState::Waiting,
            StepState::Asking,
            StepState::Cancelled,
            StepState::Secret,
        ];
        for i in 0..states.len() {
            for j in (i.saturating_add(1))..states.len() {
                assert_ne!(
                    states[i].color(),
                    states[j].color(),
                    "states {:?} and {:?} should have distinct colors",
                    states[i],
                    states[j]
                );
            }
        }
    }

    // ---- Badge color consistency ----

    #[test]
    fn do_action_badge_color_is_orange() {
        let data_val = serde_json::json!({"action_id": 7});
        let node = make_node_with_data("n1", "Do", data_val);
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.badges[0].color, theme::colors::NEON_ORANGE);
    }

    #[test]
    fn do_secret_badge_color_is_magenta() {
        let node = make_node("n1", "Do");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.badges[0].color, theme::colors::NEON_MAGENTA);
    }

    #[test]
    fn retry_badge_color_is_yellow() {
        let data_val = serde_json::json!({"max_attempts": 3});
        let node = make_node_with_data("n1", "RepeatStart", data_val);
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.badges[0].color, theme::colors::NEON_YELLOW);
    }

    #[test]
    fn timeout_badge_color_is_red() {
        let data_val = serde_json::json!({"timeout_slot": "x"});
        let node = make_node_with_data("n1", "WaitEvent", data_val);
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.badges[0].color, theme::colors::NEON_RED);
    }

    #[test]
    fn finish_badge_color_is_teal() {
        let node = make_node("n1", "Finish");
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.badges[0].color, theme::colors::NEON_TEAL);
    }

    // ---- Port position Y calculation ----

    #[test]
    fn port_y_includes_header_and_padding() {
        let port = make_port("in0", PortSide::Left, PortRole::Target, 0);
        let node = make_node_with_ports("n1", "Do", vec![port]);
        let ports = NodeRenderer::port_positions(&node);
        let expected_y = node.position[1]
            + draw::node::HEADER_HEIGHT
            + draw::node::PADDING
            + draw::port::HEIGHT / 2.0;
        assert!((ports[0].y - expected_y).abs() < f64::EPSILON);
    }

    // ---- Edge case: node at origin ----

    #[test]
    fn port_positions_node_at_origin() {
        let port = make_port("out0", PortSide::Right, PortRole::Source, 0);
        let mut node = make_node_with_ports("n1", "Do", vec![port]);
        node.position = [0.0, 0.0];
        node.size = [100.0, 60.0];
        let ports = NodeRenderer::port_positions(&node);
        // Output port at right edge: x = 0 + 100 = 100
        assert!((ports[0].x - 100.0).abs() < f64::EPSILON);
    }

    // =====================================================================
    // Additional comprehensive coverage tests
    // =====================================================================

    // ---- NodeShape derive tests ----

    #[test]
    fn node_shape_equality() {
        assert_eq!(NodeShape::Rectangle, NodeShape::Rectangle);
        assert_ne!(NodeShape::Rectangle, NodeShape::Diamond);
        assert_ne!(NodeShape::Hexagon, NodeShape::Pill);
        assert_ne!(NodeShape::Octagon, NodeShape::Circle);
        assert_ne!(NodeShape::Arrow, NodeShape::Rectangle);
    }

    #[test]
    fn node_shape_debug_format() {
        let shapes = [
            NodeShape::Rectangle,
            NodeShape::Diamond,
            NodeShape::Hexagon,
            NodeShape::Pill,
            NodeShape::Octagon,
            NodeShape::Circle,
            NodeShape::Arrow,
        ];
        for shape in &shapes {
            let debug = format!("{shape:?}");
            assert!(!debug.is_empty(), "NodeShape {:?} debug should not be empty", shape);
        }
    }

    #[test]
    fn node_shape_clone_copy() {
        let s1 = NodeShape::Diamond;
        let s2 = s1; // Copy
        let s3 = s1; // Copy again
        assert_eq!(s1, s2);
        assert_eq!(s1, s3);
    }

    // ---- IconHint derive tests ----

    #[test]
    fn icon_hint_equality() {
        assert_eq!(IconHint::None, IconHint::None);
        assert_ne!(IconHint::Data, IconHint::Action);
        assert_ne!(IconHint::Branch, IconHint::Loop);
    }

    #[test]
    fn icon_hint_all_variants_are_distinct() {
        let hints = [
            IconHint::None,
            IconHint::Data,
            IconHint::Copy,
            IconHint::Expression,
            IconHint::Object,
            IconHint::List,
            IconHint::Action,
            IconHint::Branch,
            IconHint::Loop,
            IconHint::Parallel,
            IconHint::Retry,
            IconHint::Wait,
            IconHint::Ask,
            IconHint::Error,
            IconHint::Jump,
            IconHint::Terminal,
            IconHint::Nop,
        ];
        for i in 0..hints.len() {
            for j in (i.saturating_add(1))..hints.len() {
                assert_ne!(
                    hints[i], hints[j],
                    "IconHint variants at index {i} and {j} should be distinct"
                );
            }
        }
    }

    #[test]
    fn icon_hint_debug_format() {
        let debug = format!("{:?}", IconHint::Retry);
        assert!(debug.contains("Retry"));
    }

    // ---- NodeCategory derive tests ----

    #[test]
    fn node_category_all_variants_are_distinct() {
        let cats = [
            NodeCategory::Data,
            NodeCategory::External,
            NodeCategory::Branch,
            NodeCategory::Loop,
            NodeCategory::Parallel,
            NodeCategory::Suspend,
            NodeCategory::Error,
            NodeCategory::Terminal,
            NodeCategory::Control,
            NodeCategory::Unknown,
        ];
        for i in 0..cats.len() {
            for j in (i.saturating_add(1))..cats.len() {
                assert_ne!(
                    cats[i], cats[j],
                    "NodeCategory variants at index {i} and {j} should be distinct"
                );
            }
        }
    }

    #[test]
    fn node_category_debug_format() {
        let debug = format!("{:?}", NodeCategory::Parallel);
        assert!(debug.contains("Parallel"));
    }

    // ---- StepState derive tests ----

    #[test]
    fn step_state_equality() {
        assert_eq!(StepState::Running, StepState::Running);
        assert_ne!(StepState::Running, StepState::Succeeded);
    }

    #[test]
    fn step_state_debug_format() {
        let debug = format!("{:?}", StepState::Asking);
        assert!(debug.contains("Asking"));
    }

    #[test]
    fn step_state_all_variants_color_is_valid() {
        let states = [
            StepState::Pending,
            StepState::Running,
            StepState::Succeeded,
            StepState::Failed,
            StepState::Waiting,
            StepState::Asking,
            StepState::Cancelled,
            StepState::Secret,
        ];
        for state in &states {
            let c = state.color();
            assert!(
                c[0] >= 0.0 && c[0] <= 1.0,
                "step state {:?} red out of range: {}",
                state, c[0]
            );
            assert!(
                c[1] >= 0.0 && c[1] <= 1.0,
                "step state {:?} green out of range: {}",
                state, c[1]
            );
            assert!(
                c[2] >= 0.0 && c[2] <= 1.0,
                "step state {:?} blue out of range: {}",
                state, c[2]
            );
            assert!(
                c[3] > 0.0 && c[3] <= 1.0,
                "step state {:?} alpha out of range: {}",
                state, c[3]
            );
        }
    }

    // ---- Badge derive tests ----

    #[test]
    fn badge_equality() {
        let b1 = Badge {
            label: String::from("A1"),
            color: theme::colors::NEON_ORANGE,
        };
        let b2 = Badge {
            label: String::from("A1"),
            color: theme::colors::NEON_ORANGE,
        };
        assert_eq!(b1, b2);
    }

    #[test]
    fn badge_inequality_different_label() {
        let b1 = Badge {
            label: String::from("A1"),
            color: theme::colors::NEON_ORANGE,
        };
        let b2 = Badge {
            label: String::from("A2"),
            color: theme::colors::NEON_ORANGE,
        };
        assert_ne!(b1, b2);
    }

    #[test]
    fn badge_inequality_different_color() {
        let b1 = Badge {
            label: String::from("A1"),
            color: theme::colors::NEON_ORANGE,
        };
        let b2 = Badge {
            label: String::from("A1"),
            color: theme::colors::NEON_CYAN,
        };
        assert_ne!(b1, b2);
    }

    #[test]
    fn badge_debug_format() {
        let badge = Badge {
            label: String::from("T"),
            color: theme::colors::NEON_RED,
        };
        let debug = format!("{badge:?}");
        assert!(debug.contains("Badge"));
    }

    #[test]
    fn badge_clone() {
        let badge = Badge {
            label: String::from("R3"),
            color: theme::colors::NEON_YELLOW,
        };
        let cloned = badge.clone();
        assert_eq!(badge, cloned);
    }

    // ---- PortPosition derive tests ----

    #[test]
    fn port_position_equality() {
        let p1 = PortPosition {
            id: pid("in0"),
            x: 10.0,
            y: 20.0,
            is_input: true,
            is_output: false,
        };
        let p2 = PortPosition {
            id: pid("in0"),
            x: 10.0,
            y: 20.0,
            is_input: true,
            is_output: false,
        };
        assert_eq!(p1, p2);
    }

    #[test]
    fn port_position_inequality() {
        let p1 = PortPosition {
            id: pid("in0"),
            x: 10.0,
            y: 20.0,
            is_input: true,
            is_output: false,
        };
        let p2 = PortPosition {
            id: pid("out0"),
            x: 10.0,
            y: 20.0,
            is_input: false,
            is_output: true,
        };
        assert_ne!(p1, p2);
    }

    #[test]
    fn port_position_debug_format() {
        let pp = PortPosition {
            id: pid("test"),
            x: 1.0,
            y: 2.0,
            is_input: true,
            is_output: false,
        };
        let debug = format!("{pp:?}");
        assert!(debug.contains("PortPosition"));
    }

    // ---- NodeRenderData derive tests ----

    #[test]
    fn node_render_data_equality_same() {
        let node = make_node("n1", "Do");
        let d1 = NodeRenderer::new().render(&node);
        let d2 = NodeRenderer::new().render(&node);
        assert_eq!(d1, d2);
    }

    #[test]
    fn node_render_data_debug_format() {
        let node = make_node("n1", "Do");
        let data = NodeRenderer::new().render(&node);
        let debug = format!("{data:?}");
        assert!(debug.contains("NodeRenderData"));
    }

    #[test]
    fn node_render_data_clone() {
        let node = make_node("n1", "Do");
        let data = NodeRenderer::new().render(&node);
        let cloned = data.clone();
        assert_eq!(data, cloned);
    }

    // ---- refine_shape_for_kind tests ----

    #[test]
    fn refine_shape_jump_is_arrow() {
        assert_eq!(
            NodeRenderer::refine_shape_for_kind(NodeShape::Rectangle, "Jump"),
            NodeShape::Arrow
        );
    }

    #[test]
    fn refine_shape_jump_lowercase_is_arrow() {
        assert_eq!(
            NodeRenderer::refine_shape_for_kind(NodeShape::Rectangle, "jump"),
            NodeShape::Arrow
        );
    }

    #[test]
    fn refine_shape_non_jump_preserves_shape() {
        assert_eq!(
            NodeRenderer::refine_shape_for_kind(NodeShape::Diamond, "Choose"),
            NodeShape::Diamond
        );
    }

    #[test]
    fn refine_shape_nop_preserves_rectangle() {
        assert_eq!(
            NodeRenderer::refine_shape_for_kind(NodeShape::Rectangle, "Nop"),
            NodeShape::Rectangle
        );
    }

    #[test]
    fn refine_shape_unknown_preserves_shape() {
        assert_eq!(
            NodeRenderer::refine_shape_for_kind(NodeShape::Hexagon, "Something"),
            NodeShape::Hexagon
        );
    }

    // ---- classify_kind additional lowercase and variant tests ----

    #[test]
    fn classify_all_lowercase_variants() {
        assert_eq!(classify_kind("set_const"), NodeCategory::Data);
        assert_eq!(classify_kind("copy"), NodeCategory::Data);
        assert_eq!(classify_kind("eval_expr"), NodeCategory::Data);
        assert_eq!(classify_kind("build_object"), NodeCategory::Data);
        assert_eq!(classify_kind("build_list"), NodeCategory::Data);
        assert_eq!(classify_kind("data"), NodeCategory::Data);
        assert_eq!(classify_kind("choose_slot"), NodeCategory::Branch);
        assert_eq!(classify_kind("branch"), NodeCategory::Branch);
        assert_eq!(classify_kind("foreach_start"), NodeCategory::Loop);
        assert_eq!(classify_kind("foreach_next"), NodeCategory::Loop);
        assert_eq!(classify_kind("foreach_join"), NodeCategory::Loop);
        assert_eq!(classify_kind("collect_start"), NodeCategory::Loop);
        assert_eq!(classify_kind("collect_page"), NodeCategory::Loop);
        assert_eq!(classify_kind("collect_next"), NodeCategory::Loop);
        assert_eq!(classify_kind("collect_finish"), NodeCategory::Loop);
        assert_eq!(classify_kind("reduce_start"), NodeCategory::Loop);
        assert_eq!(classify_kind("reduce_next"), NodeCategory::Loop);
        assert_eq!(classify_kind("reduce_finish"), NodeCategory::Loop);
        assert_eq!(classify_kind("loop"), NodeCategory::Loop);
        assert_eq!(classify_kind("together_start"), NodeCategory::Parallel);
        assert_eq!(classify_kind("together_branch"), NodeCategory::Parallel);
        assert_eq!(classify_kind("together_join"), NodeCategory::Parallel);
        assert_eq!(classify_kind("parallel"), NodeCategory::Parallel);
        assert_eq!(classify_kind("wait_until"), NodeCategory::Suspend);
        assert_eq!(classify_kind("wait_event"), NodeCategory::Suspend);
        assert_eq!(classify_kind("ask_resume"), NodeCategory::Suspend);
        assert_eq!(classify_kind("suspend"), NodeCategory::Suspend);
        assert_eq!(classify_kind("wait"), NodeCategory::Suspend);
        assert_eq!(classify_kind("error_handler"), NodeCategory::Error);
        assert_eq!(classify_kind("error"), NodeCategory::Error);
        assert_eq!(classify_kind("retry_check"), NodeCategory::Error);
        assert_eq!(classify_kind("repeat_start"), NodeCategory::Error);
        assert_eq!(classify_kind("repeat_attempt"), NodeCategory::Error);
        assert_eq!(classify_kind("repeat_check"), NodeCategory::Error);
        assert_eq!(classify_kind("repeat_finish"), NodeCategory::Error);
        assert_eq!(classify_kind("finish"), NodeCategory::Terminal);
        assert_eq!(classify_kind("terminal"), NodeCategory::Terminal);
        assert_eq!(classify_kind("jump"), NodeCategory::Control);
        assert_eq!(classify_kind("nop"), NodeCategory::Control);
        assert_eq!(classify_kind("control"), NodeCategory::Control);
    }

    // ---- extract_action_badge edge cases ----

    #[test]
    fn action_badge_with_string_action_id_returns_none() {
        let data_val = serde_json::json!({"action_id": "not_a_number"});
        let node = make_node_with_data("n1", "Do", data_val);
        let data = NodeRenderer::new().render(&node);
        // Only the "S" badge since action_id is not a u64
        assert_eq!(data.badges.len(), 1);
        assert_eq!(data.badges[0].label, "S");
    }

    #[test]
    fn action_badge_with_zero_action_id() {
        let data_val = serde_json::json!({"action_id": 0});
        let node = make_node_with_data("n1", "Do", data_val);
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.badges.len(), 2);
        assert_eq!(data.badges[0].label, "A0");
    }

    // ---- extract_retry_badge edge cases ----

    #[test]
    fn retry_badge_with_string_max_attempts_returns_none() {
        let data_val = serde_json::json!({"max_attempts": "three"});
        let node = make_node_with_data("n1", "RepeatStart", data_val);
        let data = NodeRenderer::new().render(&node);
        assert!(data.badges.is_empty());
    }

    #[test]
    fn retry_badge_with_large_attempts() {
        let data_val = serde_json::json!({"max_attempts": 999});
        let node = make_node_with_data("n1", "RepeatStart", data_val);
        let data = NodeRenderer::new().render(&node);
        assert_eq!(data.badges[0].label, "R999");
    }

    // ---- has_timeout edge cases ----

    #[test]
    fn has_timeout_with_null_timeout_slot() {
        let data_val = serde_json::json!({"timeout_slot": null});
        let node = make_node_with_data("n1", "WaitEvent", data_val);
        let data = NodeRenderer::new().render(&node);
        assert!(data.badges.is_empty());
    }

    #[test]
    fn has_timeout_with_false_has_timeout() {
        let data_val = serde_json::json!({"has_timeout": false});
        let node = make_node_with_data("n1", "Ask", data_val);
        let data = NodeRenderer::new().render(&node);
        assert!(data.badges.is_empty());
    }

    #[test]
    fn has_timeout_with_both_fields_set() {
        let data_val = serde_json::json!({"timeout_slot": "slot_1", "has_timeout": true});
        let node = make_node_with_data("n1", "WaitUntil", data_val);
        let data = NodeRenderer::new().render(&node);
        // Should have exactly one timeout badge (not two)
        assert_eq!(data.badges.len(), 1);
        assert_eq!(data.badges[0].label, "T");
    }

    // ---- Port position: bottom side ----

    #[test]
    fn port_positions_bottom_side_uses_center_x() {
        let port = make_port("bot0", PortSide::Bottom, PortRole::Target, 0);
        let node = make_node_with_ports("n1", "Do", vec![port]);
        let ports = NodeRenderer::port_positions(&node);
        // Bottom port: x = node.x + node.width / 2 = 100 + 80 = 180
        assert!((ports[0].x - 180.0).abs() < f64::EPSILON);
    }

    // ---- Port position: input on right side ----

    #[test]
    fn port_positions_right_side_input() {
        let port = make_port("in_r", PortSide::Right, PortRole::Target, 0);
        let node = make_node_with_ports("n1", "Do", vec![port]);
        let ports = NodeRenderer::port_positions(&node);
        // PortSide::Right overrides: x = node.x + node.width = 260
        assert!((ports[0].x - 260.0).abs() < f64::EPSILON);
        assert!(ports[0].is_input);
        assert!(!ports[0].is_output);
    }

    // ---- Dimension constants are positive ----

    #[test]
    fn all_dimension_constants_are_positive() {
        assert!(DEFAULT_WIDTH > 0.0);
        assert!(DEFAULT_HEIGHT > 0.0);
        assert!(DIAMOND_WIDTH > 0.0);
        assert!(DIAMOND_HEIGHT > 0.0);
        assert!(HEXAGON_WIDTH > 0.0);
        assert!(HEXAGON_HEIGHT > 0.0);
        assert!(PILL_WIDTH > 0.0);
        assert!(PILL_HEIGHT > 0.0);
        assert!(CIRCLE_SIZE > 0.0);
        assert!(OCTAGON_WIDTH > 0.0);
        assert!(OCTAGON_HEIGHT > 0.0);
        assert!(ARROW_WIDTH > 0.0);
        assert!(ARROW_HEIGHT > 0.0);
    }

    // ---- NodeRenderer with_state ----

    #[test]
    fn renderer_with_state_debug() {
        let renderer = NodeRenderer::with_state(StepState::Running);
        let debug = format!("{renderer:?}");
        assert!(debug.contains("NodeRenderer"));
    }

    #[test]
    fn renderer_clone() {
        let renderer = NodeRenderer::with_state(StepState::Waiting);
        let cloned = renderer.clone();
        assert_eq!(cloned.state, Some(StepState::Waiting));
    }

    // ---- All category colors: body is always CARD_BG ----

    #[test]
    fn all_category_body_colors_are_card_bg() {
        let categories = [
            NodeCategory::Data,
            NodeCategory::External,
            NodeCategory::Branch,
            NodeCategory::Loop,
            NodeCategory::Parallel,
            NodeCategory::Suspend,
            NodeCategory::Error,
            NodeCategory::Terminal,
            NodeCategory::Control,
            NodeCategory::Unknown,
        ];
        for cat in &categories {
            let (_, body, _, _) = NodeRenderer::resolve_colors(cat);
            assert_eq!(
                body,
                theme::colors::CARD_BG,
                "body color for {cat:?} should be CARD_BG"
            );
        }
    }

    // ---- All category colors: text is primary except control and data ----

    #[test]
    fn control_category_text_is_dim() {
        let (_, _, _, txt) = NodeRenderer::resolve_colors(&NodeCategory::Control);
        assert_eq!(txt, theme::colors::TEXT_DIM);
    }

    // ---- NodeRenderer render respects node size for port positions ----

    #[test]
    fn port_positions_respects_custom_node_size() {
        let port = make_port("out0", PortSide::Right, PortRole::Source, 0);
        let mut node = make_node_with_ports("n1", "Do", vec![port]);
        node.size = [300.0, 100.0];
        let ports = NodeRenderer::port_positions(&node);
        // Right edge: 100 + 300 = 400
        assert!((ports[0].x - 400.0).abs() < f64::EPSILON);
    }

    // ---- Arrow dimensions test ----

    #[test]
    fn arrow_width_and_height_positive() {
        assert!(ARROW_WIDTH > 0.0);
        assert!(ARROW_HEIGHT > 0.0);
    }
}
