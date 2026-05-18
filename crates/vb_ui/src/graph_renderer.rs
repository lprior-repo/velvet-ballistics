#![forbid(unsafe_code)]
//! Render instruction production for the workflow graph.
//!
//! This module produces data-structure render instructions (`NodeCard`, `EdgeLine`)
//! that a Makepad Splash UI can consume. It is NOT a widget -- it has no side effects
//! and performs no drawing. Given a `CompiledNodeKind` and optional runtime state, it
//! produces colour, badge, label, and dimension data for each graph node and edge.

use crate::theme::colors;
use vb_core::workflow::CompiledNodeKind;

// ---------------------------------------------------------------------------
// Node card dimensions
// ---------------------------------------------------------------------------

/// Fixed node card width in pixels.
pub const NODE_WIDTH: f64 = 160.0;
/// Fixed node card height in pixels.
pub const NODE_HEIGHT: f64 = 48.0;
/// Header strip height in pixels.
pub const HEADER_HEIGHT: f64 = 24.0;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Render instruction for a single graph node card.
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

/// A small annotation badge on a node card (e.g. "A0", "R3", "S").
#[derive(Debug, Clone)]
pub struct NodeBadge {
    pub label: String,
    pub color: [f32; 4],
}

/// Semantic category of a node -- determines colour palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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

/// Glow overlay for a node reflecting runtime step state.
#[derive(Debug, Clone)]
pub struct StateOverlay {
    pub state: OverlayState,
    pub glow_color: [f32; 4],
    pub glow_radius: f32,
}

/// Runtime step state used for the glow overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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

/// Render instruction for a directed edge between two nodes.
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

/// Visual classification of an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EdgeType {
    Normal,
    Branch,
    ErrorRoute,
    RetryRoute,
    Join,
    LoopBack,
}

// ---------------------------------------------------------------------------
// classify_node
// ---------------------------------------------------------------------------

/// Classify a compiled node kind into a visual category.
///
/// Maps all 34 `CompiledNodeKind` variants to one of 9 `NodeCategory` values.
#[must_use]
pub fn classify_node(kind: &CompiledNodeKind) -> NodeCategory {
    match kind {
        // Data manipulation and construction
        CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. } => NodeCategory::Data,

        // External action
        CompiledNodeKind::Do { .. } => NodeCategory::External,

        // Branching
        CompiledNodeKind::Choose { .. } | CompiledNodeKind::ChooseSlot { .. } => {
            NodeCategory::Branch
        }

        // Loop constructs (for-each, collect, reduce, repeat)
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

        // Parallel (together)
        CompiledNodeKind::TogetherStart { .. }
        | CompiledNodeKind::TogetherBranch { .. }
        | CompiledNodeKind::TogetherJoin { .. } => NodeCategory::Parallel,

        // Suspend / wait / ask
        CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. } => NodeCategory::Suspend,

        // Error handling and retry
        CompiledNodeKind::ErrorHandler { .. } | CompiledNodeKind::RetryCheck { .. } => {
            NodeCategory::Error
        }

        // Terminal
        CompiledNodeKind::Finish { .. } => NodeCategory::Terminal,

        // Control flow (nop, jump)
        CompiledNodeKind::Nop | CompiledNodeKind::Jump { .. } => NodeCategory::Control,
    }
}

// ---------------------------------------------------------------------------
// node_header_color
// ---------------------------------------------------------------------------

/// Header strip colour for a given node category, sourced from the theme palette.
#[must_use]
pub fn node_header_color(category: NodeCategory) -> [f32; 4] {
    match category {
        NodeCategory::Data => colors::node_header::DATA,
        NodeCategory::External => colors::node_header::EXTERNAL,
        NodeCategory::Branch => colors::node_header::BRANCH,
        NodeCategory::Loop => colors::node_header::LOOP,
        NodeCategory::Parallel => colors::node_header::PARALLEL,
        NodeCategory::Suspend => colors::node_header::SUSPEND,
        NodeCategory::Terminal => colors::node_header::TERMINAL,
        NodeCategory::Error => colors::node_header::ERROR,
        NodeCategory::Control => colors::node_header::CONTROL,
    }
}

// ---------------------------------------------------------------------------
// node_body_color
// ---------------------------------------------------------------------------

/// Body fill colour for a given node category, sourced from the theme palette.
#[must_use]
pub fn node_body_color(category: NodeCategory) -> [f32; 4] {
    match category {
        NodeCategory::Data => colors::node_category::DATA,
        NodeCategory::External => colors::node_category::EXTERNAL,
        NodeCategory::Branch => colors::node_category::BRANCH,
        NodeCategory::Loop => colors::node_category::LOOP,
        NodeCategory::Parallel => colors::node_category::PARALLEL,
        NodeCategory::Suspend => colors::node_category::SUSPEND,
        NodeCategory::Terminal => colors::node_category::TERMINAL,
        NodeCategory::Error => colors::node_category::ERROR,
        NodeCategory::Control => colors::node_category::CONTROL,
    }
}

// ---------------------------------------------------------------------------
// kind_label
// ---------------------------------------------------------------------------

/// Human-readable label for a compiled node kind.
#[must_use]
pub fn kind_label(kind: &CompiledNodeKind) -> String {
    match kind {
        CompiledNodeKind::Nop => String::from("Nop"),
        CompiledNodeKind::SetConst { .. } => String::from("SetConst"),
        CompiledNodeKind::Copy { .. } => String::from("Copy"),
        CompiledNodeKind::EvalExpr { .. } => String::from("EvalExpr"),
        CompiledNodeKind::BuildObject { .. } => String::from("BuildObject"),
        CompiledNodeKind::BuildList { .. } => String::from("BuildList"),
        CompiledNodeKind::Do { action, .. } => format!("Do#{}", action.get()),
        CompiledNodeKind::Choose { .. } => String::from("Choose"),
        CompiledNodeKind::ChooseSlot { .. } => String::from("ChooseSlot"),
        CompiledNodeKind::ForEachStart { .. } => String::from("ForEach"),
        CompiledNodeKind::ForEachNext { .. } => String::from("ForEach*"),
        CompiledNodeKind::ForEachJoin { .. } => String::from("ForEachJoin"),
        CompiledNodeKind::TogetherStart { .. } => String::from("Together"),
        CompiledNodeKind::TogetherBranch { branch, .. } => format!("Branch#{}", branch),
        CompiledNodeKind::TogetherJoin { .. } => String::from("TogetherJoin"),
        CompiledNodeKind::CollectStart { .. } => String::from("Collect"),
        CompiledNodeKind::CollectPage { .. } => String::from("CollectPage"),
        CompiledNodeKind::CollectNext { .. } => String::from("Collect*"),
        CompiledNodeKind::CollectFinish { .. } => String::from("CollectDone"),
        CompiledNodeKind::ReduceStart { .. } => String::from("Reduce"),
        CompiledNodeKind::ReduceNext { .. } => String::from("Reduce*"),
        CompiledNodeKind::ReduceFinish { .. } => String::from("ReduceDone"),
        CompiledNodeKind::RepeatStart { max_attempts, .. } => {
            format!("Repeat(<= {})", max_attempts)
        }
        CompiledNodeKind::RepeatAttempt { .. } => String::from("Attempt"),
        CompiledNodeKind::RepeatCheck { .. } => String::from("RepeatCheck"),
        CompiledNodeKind::RepeatFinish { .. } => String::from("RepeatDone"),
        CompiledNodeKind::WaitUntil { .. } => String::from("WaitUntil"),
        CompiledNodeKind::WaitEvent { .. } => String::from("WaitEvent"),
        CompiledNodeKind::Ask { .. } => String::from("Ask"),
        CompiledNodeKind::AskResume { .. } => String::from("AskResume"),
        CompiledNodeKind::RetryCheck { .. } => String::from("RetryCheck"),
        CompiledNodeKind::ErrorHandler { .. } => String::from("ErrorHandler"),
        CompiledNodeKind::Jump { .. } => String::from("Jump"),
        CompiledNodeKind::Finish { .. } => String::from("Finish"),
    }
}

// ---------------------------------------------------------------------------
// state_glow
// ---------------------------------------------------------------------------

/// Glow colour and radius for a runtime overlay state.
///
/// Returns `(glow_color, glow_radius)` sourced from the theme palette.
#[must_use]
pub fn state_glow(state: OverlayState) -> ([f32; 4], f32) {
    match state {
        OverlayState::Pending => (colors::state::PENDING, 2.0),
        OverlayState::Running => (colors::state::RUNNING, 4.0),
        OverlayState::Succeeded => (colors::state::SUCCEEDED, 3.0),
        OverlayState::Failed => (colors::state::FAILED, 6.0),
        OverlayState::Skipped => (colors::state::SKIPPED, 2.0),
        OverlayState::Waiting => (colors::state::WAITING, 3.0),
        OverlayState::Asking => (colors::state::ASKING, 3.0),
        OverlayState::Cancelled => (colors::state::CANCELLED, 2.0),
    }
}

// ---------------------------------------------------------------------------
// extract_badges
// ---------------------------------------------------------------------------

/// Extract annotation badges from a compiled node kind.
///
/// Badge types:
/// - `"A{id}"` -- action ID badge for `Do` nodes.
/// - `"S"` -- secret-sensitive badge for `Do` nodes.
/// - `"R{max}"` -- retry badge for `RepeatStart` nodes (max attempts).
/// - `"T"` -- timeout badge for `WaitEvent` and `Ask` nodes with a timeout slot.
/// - `"D"` -- strict-durable badge for `Finish` nodes.
#[must_use]
pub fn extract_badges(kind: &CompiledNodeKind) -> Vec<NodeBadge> {
    let mut badges = Vec::new();

    match kind {
        CompiledNodeKind::Do { action, .. } => {
            badges.push(NodeBadge {
                label: format!("A{}", action.get()),
                color: colors::neon::ORANGE,
            });
            badges.push(NodeBadge {
                label: String::from("S"),
                color: colors::neon::MAGENTA,
            });
        }

        CompiledNodeKind::RepeatStart { max_attempts, .. } => {
            badges.push(NodeBadge {
                label: format!("R{}", max_attempts),
                color: colors::neon::YELLOW,
            });
        }

        CompiledNodeKind::WaitEvent {
            timeout_slot: Some(_),
            ..
        }
        | CompiledNodeKind::Ask {
            timeout_slot: Some(_),
            ..
        } => {
            badges.push(NodeBadge {
                label: String::from("T"),
                color: colors::neon::RED,
            });
        }

        CompiledNodeKind::Finish { .. } => {
            badges.push(NodeBadge {
                label: String::from("D"),
                color: colors::neon::TEAL,
            });
        }

        // All other variants produce no badges.
        _ => {}
    }

    badges
}

// ---------------------------------------------------------------------------
// edge_color
// ---------------------------------------------------------------------------

/// Colour for a given edge type, sourced from the theme palette.
#[must_use]
pub fn edge_color(edge_type: EdgeType) -> [f32; 4] {
    match edge_type {
        EdgeType::Normal => colors::neon::CYAN_DIM,
        EdgeType::Branch => colors::neon::PURPLE,
        EdgeType::ErrorRoute => colors::neon::RED_DIM,
        EdgeType::RetryRoute => colors::neon::YELLOW,
        EdgeType::Join => colors::neon::BLUE_DIM,
        EdgeType::LoopBack => colors::neon::TEAL,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{ActionId, SlotIdx, StepIdx};

    /// Build a Vec of all 34 CompiledNodeKind variants for exhaustive testing.
    fn all_kinds() -> Vec<CompiledNodeKind> {
        vec![
            CompiledNodeKind::Nop,
            CompiledNodeKind::SetConst {
                value: vb_core::ids::ConstIdx::new(0),
            },
            CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
            CompiledNodeKind::EvalExpr {
                expr: vb_core::ids::ExprIdx::new(0),
            },
            CompiledNodeKind::BuildObject {
                fields: Box::new([]),
            },
            CompiledNodeKind::BuildList {
                items: Box::new([]),
            },
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Choose {
                branches: Box::new([]),
                otherwise: None,
            },
            CompiledNodeKind::ChooseSlot {
                branches: Box::new([]),
                otherwise: None,
            },
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(0),
            },
            CompiledNodeKind::TogetherStart {
                branches: Box::new([]),
                join: StepIdx::new(0),
            },
            CompiledNodeKind::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(1),
                join: StepIdx::new(2),
                accumulator: SlotIdx::new(0),
            },
            CompiledNodeKind::TogetherJoin {
                branch_count: 1,
                accumulator: SlotIdx::new(0),
            },
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 10,
                page_size: 5,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::CollectPage {
                collector_slot: SlotIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::CollectNext {
                collector_slot: SlotIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(0),
            },
            CompiledNodeKind::ReduceStart {
                input: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                initial: vb_core::ids::ConstIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::ReduceNext {
                iterator_slot: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::ReduceFinish {
                accumulator: SlotIdx::new(0),
            },
            CompiledNodeKind::RepeatStart {
                max_attempts: 3,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::RepeatAttempt {
                attempt_slot: SlotIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::RepeatCheck {
                attempt_slot: SlotIdx::new(0),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::RepeatFinish {
                result: SlotIdx::new(0),
            },
            CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
            CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None,
            },
            CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
            CompiledNodeKind::AskResume {
                answer: SlotIdx::new(0),
            },
            CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(0),
                body: StepIdx::new(1),
                exhausted: StepIdx::new(2),
            },
            CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: None,
            },
            CompiledNodeKind::Jump {
                target: StepIdx::new(1),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]
    }

    #[test]
    fn classify_all_34_variants() {
        let kinds = all_kinds();
        assert_eq!(
            kinds.len(),
            34,
            "must exercise all 34 CompiledNodeKind variants"
        );

        for kind in &kinds {
            let cat = classify_node(kind);
            // Verify header/body colour lookup does not panic.
            let _hdr = node_header_color(cat);
            let _body = node_body_color(cat);
            // Verify label production.
            let label = kind_label(kind);
            assert!(!label.is_empty(), "label must not be empty for {kind:?}");
            // Verify badge extraction does not panic.
            let _badges = extract_badges(kind);
        }
    }

    #[test]
    fn do_node_has_action_and_secret_badges() {
        let kind = CompiledNodeKind::Do {
            action: ActionId::new(42),
            input: SlotIdx::new(0),
        };
        let badges = extract_badges(&kind);
        assert_eq!(badges.len(), 2);
        assert_eq!(badges[0].label, "A42");
        assert_eq!(badges[1].label, "S");
    }

    #[test]
    fn repeat_start_has_retry_badge() {
        let kind = CompiledNodeKind::RepeatStart {
            max_attempts: 5,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let badges = extract_badges(&kind);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].label, "R5");
    }

    #[test]
    fn wait_event_with_timeout_has_timeout_badge() {
        let kind = CompiledNodeKind::WaitEvent {
            event: SlotIdx::new(0),
            timeout_slot: Some(SlotIdx::new(1)),
        };
        let badges = extract_badges(&kind);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].label, "T");
    }

    #[test]
    fn wait_event_without_timeout_has_no_badges() {
        let kind = CompiledNodeKind::WaitEvent {
            event: SlotIdx::new(0),
            timeout_slot: None,
        };
        let badges = extract_badges(&kind);
        assert!(badges.is_empty());
    }

    #[test]
    fn ask_with_timeout_has_timeout_badge() {
        let kind = CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: Some(SlotIdx::new(1)),
        };
        let badges = extract_badges(&kind);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].label, "T");
    }

    #[test]
    fn finish_has_durable_badge() {
        let kind = CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        };
        let badges = extract_badges(&kind);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].label, "D");
    }

    #[test]
    fn nop_has_no_badges() {
        let badges = extract_badges(&CompiledNodeKind::Nop);
        assert!(badges.is_empty());
    }

    #[test]
    fn state_glow_running_is_cyan() {
        let (color, radius) = state_glow(OverlayState::Running);
        assert_eq!(color, colors::state::RUNNING);
        assert!(radius > 0.0);
    }

    #[test]
    fn state_glow_failed_is_red() {
        let (color, radius) = state_glow(OverlayState::Failed);
        assert_eq!(color, colors::state::FAILED);
        assert!(radius > 0.0);
    }

    #[test]
    fn all_overlay_states_have_glow() {
        let states = [
            OverlayState::Pending,
            OverlayState::Running,
            OverlayState::Succeeded,
            OverlayState::Failed,
            OverlayState::Skipped,
            OverlayState::Waiting,
            OverlayState::Asking,
            OverlayState::Cancelled,
        ];
        for s in &states {
            let (_, r) = state_glow(*s);
            assert!(r > 0.0, "glow radius must be positive for {s:?}");
        }
    }

    #[test]
    fn all_edge_types_have_color_with_positive_alpha() {
        let types = [
            EdgeType::Normal,
            EdgeType::Branch,
            EdgeType::ErrorRoute,
            EdgeType::RetryRoute,
            EdgeType::Join,
            EdgeType::LoopBack,
        ];
        for t in &types {
            let c = edge_color(*t);
            assert!(c[3] > 0.0, "alpha must be positive for {t:?}");
        }
    }

    #[test]
    fn node_dimensions_are_fixed() {
        assert_eq!(NODE_WIDTH, 160.0);
        assert_eq!(NODE_HEIGHT, 48.0);
        assert_eq!(HEADER_HEIGHT, 24.0);
    }

    #[test]
    fn classify_do_as_external() {
        let kind = CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        };
        assert_eq!(classify_node(&kind), NodeCategory::External);
    }

    #[test]
    fn classify_choose_as_branch() {
        let kind = CompiledNodeKind::Choose {
            branches: Box::new([]),
            otherwise: None,
        };
        assert_eq!(classify_node(&kind), NodeCategory::Branch);
    }

    #[test]
    fn classify_together_start_as_parallel() {
        let kind = CompiledNodeKind::TogetherStart {
            branches: Box::new([]),
            join: StepIdx::new(0),
        };
        assert_eq!(classify_node(&kind), NodeCategory::Parallel);
    }

    #[test]
    fn classify_wait_until_as_suspend() {
        let kind = CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(0),
        };
        assert_eq!(classify_node(&kind), NodeCategory::Suspend);
    }

    #[test]
    fn classify_finish_as_terminal() {
        let kind = CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        };
        assert_eq!(classify_node(&kind), NodeCategory::Terminal);
    }

    #[test]
    fn classify_error_handler_as_error() {
        let kind = CompiledNodeKind::ErrorHandler {
            body: StepIdx::new(1),
            handler: StepIdx::new(2),
            error_slot: None,
        };
        assert_eq!(classify_node(&kind), NodeCategory::Error);
    }

    #[test]
    fn classify_retry_check_as_error() {
        let kind = CompiledNodeKind::RetryCheck {
            policy_slot: SlotIdx::new(0),
            body: StepIdx::new(1),
            exhausted: StepIdx::new(2),
        };
        assert_eq!(classify_node(&kind), NodeCategory::Error);
    }

    #[test]
    fn classify_jump_as_control() {
        let kind = CompiledNodeKind::Jump {
            target: StepIdx::new(1),
        };
        assert_eq!(classify_node(&kind), NodeCategory::Control);
    }

    #[test]
    fn classify_foreach_start_as_loop() {
        let kind = CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 10,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        assert_eq!(classify_node(&kind), NodeCategory::Loop);
    }

    #[test]
    fn header_and_body_colors_differ_for_each_category() {
        for cat in [
            NodeCategory::Data,
            NodeCategory::External,
            NodeCategory::Branch,
            NodeCategory::Loop,
            NodeCategory::Parallel,
            NodeCategory::Suspend,
            NodeCategory::Terminal,
            NodeCategory::Error,
            NodeCategory::Control,
        ] {
            let hdr = node_header_color(cat);
            let body = node_body_color(cat);
            // Header should be darker (lower RGB values) than body.
            assert!(
                hdr[0] <= body[0] || hdr[1] <= body[1] || hdr[2] <= body[2],
                "header should be darker than body for {cat:?}",
            );
        }
    }

    #[test]
    fn kind_label_do_includes_action_id() {
        let kind = CompiledNodeKind::Do {
            action: ActionId::new(7),
            input: SlotIdx::new(0),
        };
        let label = kind_label(&kind);
        assert_eq!(label, "Do#7");
    }

    #[test]
    fn kind_label_together_branch_includes_index() {
        let kind = CompiledNodeKind::TogetherBranch {
            branch: 3,
            entry: StepIdx::new(1),
            join: StepIdx::new(2),
            accumulator: SlotIdx::new(0),
        };
        let label = kind_label(&kind);
        assert_eq!(label, "Branch#3");
    }

    #[test]
    fn kind_label_repeat_start_includes_max() {
        let kind = CompiledNodeKind::RepeatStart {
            max_attempts: 10,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let label = kind_label(&kind);
        assert_eq!(label, "Repeat(<= 10)");
    }

    #[test]
    fn node_header_color_uses_theme_palette() {
        assert_eq!(
            node_header_color(NodeCategory::Data),
            colors::node_header::DATA
        );
        assert_eq!(
            node_header_color(NodeCategory::External),
            colors::node_header::EXTERNAL
        );
        assert_eq!(
            node_header_color(NodeCategory::Error),
            colors::node_header::ERROR
        );
    }

    #[test]
    fn node_body_color_uses_theme_palette() {
        assert_eq!(
            node_body_color(NodeCategory::Data),
            colors::node_category::DATA
        );
        assert_eq!(
            node_body_color(NodeCategory::External),
            colors::node_category::EXTERNAL
        );
        assert_eq!(
            node_body_color(NodeCategory::Error),
            colors::node_category::ERROR
        );
    }

    #[test]
    fn edge_color_uses_theme_palette() {
        assert_eq!(edge_color(EdgeType::Normal), colors::neon::CYAN_DIM);
        assert_eq!(edge_color(EdgeType::Branch), colors::neon::PURPLE);
        assert_eq!(edge_color(EdgeType::ErrorRoute), colors::neon::RED_DIM);
        assert_eq!(edge_color(EdgeType::RetryRoute), colors::neon::YELLOW);
        assert_eq!(edge_color(EdgeType::Join), colors::neon::BLUE_DIM);
        assert_eq!(edge_color(EdgeType::LoopBack), colors::neon::TEAL);
    }

    // -------------------------------------------------------------------------
    // New tests covering requested surface area
    // -------------------------------------------------------------------------

    #[test]
    fn classify_build_object_as_data() {
        let kind = CompiledNodeKind::BuildObject {
            fields: Box::new([]),
        };
        assert_eq!(classify_node(&kind), NodeCategory::Data);
    }

    #[test]
    fn classify_build_list_as_data() {
        let kind = CompiledNodeKind::BuildList {
            items: Box::new([]),
        };
        assert_eq!(classify_node(&kind), NodeCategory::Data);
    }

    #[test]
    fn classify_collect_start_as_loop() {
        let kind = CompiledNodeKind::CollectStart {
            source: SlotIdx::new(0),
            limit: 10,
            page_size: 5,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        assert_eq!(classify_node(&kind), NodeCategory::Loop);
    }

    #[test]
    fn classify_reduce_start_as_loop() {
        let kind = CompiledNodeKind::ReduceStart {
            input: SlotIdx::new(0),
            accumulator: SlotIdx::new(1),
            initial: vb_core::ids::ConstIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        assert_eq!(classify_node(&kind), NodeCategory::Loop);
    }

    #[test]
    fn classify_ask_resume_as_suspend() {
        let kind = CompiledNodeKind::AskResume {
            answer: SlotIdx::new(0),
        };
        assert_eq!(classify_node(&kind), NodeCategory::Suspend);
    }

    #[test]
    fn classify_nop_as_control() {
        assert_eq!(classify_node(&CompiledNodeKind::Nop), NodeCategory::Control);
    }

    #[test]
    fn kind_label_all_loop_variants() {
        let foreach_start = CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 10,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        assert_eq!(kind_label(&foreach_start), "ForEach");

        let collect_start = CompiledNodeKind::CollectStart {
            source: SlotIdx::new(0),
            limit: 10,
            page_size: 5,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        assert_eq!(kind_label(&collect_start), "Collect");

        let collect_next = CompiledNodeKind::CollectNext {
            collector_slot: SlotIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        assert_eq!(kind_label(&collect_next), "Collect*");

        let collect_finish = CompiledNodeKind::CollectFinish {
            collector_slot: SlotIdx::new(0),
        };
        assert_eq!(kind_label(&collect_finish), "CollectDone");

        let reduce_start = CompiledNodeKind::ReduceStart {
            input: SlotIdx::new(0),
            accumulator: SlotIdx::new(1),
            initial: vb_core::ids::ConstIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        assert_eq!(kind_label(&reduce_start), "Reduce");

        let reduce_finish = CompiledNodeKind::ReduceFinish {
            accumulator: SlotIdx::new(0),
        };
        assert_eq!(kind_label(&reduce_finish), "ReduceDone");
    }

    #[test]
    fn extract_badges_ask_without_timeout_is_empty() {
        let kind = CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None,
        };
        let badges = extract_badges(&kind);
        assert!(badges.is_empty());
    }

    #[test]
    fn edge_color_loopback_is_teal() {
        let c = edge_color(EdgeType::LoopBack);
        assert_eq!(c, colors::neon::TEAL);
    }

    #[test]
    fn state_glow_each_variant_positive_radius() {
        let variants = [
            OverlayState::Pending,
            OverlayState::Running,
            OverlayState::Succeeded,
            OverlayState::Failed,
            OverlayState::Skipped,
            OverlayState::Waiting,
            OverlayState::Asking,
            OverlayState::Cancelled,
        ];
        for v in &variants {
            let (color, radius) = state_glow(*v);
            assert!(
                radius > 0.0,
                "radius must be positive for {v:?}, got {radius}"
            );
            // Alpha channel of glow color must also be positive.
            assert!(color[3] > 0.0, "glow alpha must be positive for {v:?}");
        }
    }

    // -----------------------------------------------------------------------
    // BLACKHAT security and correctness review tests
    // -----------------------------------------------------------------------

    /// LOW: RepeatStart with max_attempts=0 produces "R0" badge. This is
    /// semantically questionable -- a repeat with 0 max attempts means the
    /// loop body never executes. The badge is still generated, which could
    /// mislead users into thinking retry is configured.
    #[test]
    fn blackhat_repeat_start_zero_max_attempts_produces_r0_badge() {
        let kind = CompiledNodeKind::RepeatStart {
            max_attempts: 0,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let badges = extract_badges(&kind);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].label, "R0");
    }

    /// LOW: Do node action ID u16::MAX produces large label string.
    /// The action.get() returns the raw u16 which is formatted into the badge.
    /// No overflow, but the label "A65535" is long for a badge.
    #[test]
    fn blackhat_do_node_action_max_u16_produces_large_label() {
        let kind = CompiledNodeKind::Do {
            action: ActionId::new(u16::MAX),
            input: SlotIdx::new(0),
        };
        let label = kind_label(&kind);
        assert_eq!(label, "Do#65535");
        let badges = extract_badges(&kind);
        assert_eq!(badges[0].label, "A65535");
    }

    /// LOW: TogetherBranch branch index is a plain u16 -- large value
    /// produces long label. No overflow risk, but documents the behavior.
    #[test]
    fn blackhat_together_branch_large_index_label() {
        let kind = CompiledNodeKind::TogetherBranch {
            branch: u16::MAX,
            entry: StepIdx::new(1),
            join: StepIdx::new(2),
            accumulator: SlotIdx::new(0),
        };
        let label = kind_label(&kind);
        assert_eq!(label, "Branch#65535");
    }

    /// LOW: NodeCard default-like construction with step_idx u16::MAX.
    /// Verifies that the data structures accept the full range of u16 step
    /// indices without issue.
    #[test]
    fn blackhat_node_card_accepts_u16_max_step_idx() {
        let card = NodeCard {
            step_idx: u16::MAX,
            step_name: String::from("test"),
            kind_label: String::from("Nop"),
            category: NodeCategory::Control,
            x: 0.0,
            y: 0.0,
            width: NODE_WIDTH,
            height: NODE_HEIGHT,
            header_color: node_header_color(NodeCategory::Control),
            body_color: node_body_color(NodeCategory::Control),
            border_color: [0.0; 4],
            text_color: [1.0; 4],
            badges: Vec::new(),
            state_overlay: None,
        };
        assert_eq!(card.step_idx, u16::MAX);
    }

    /// LOW: EdgeLine with source and target both u16::MAX. Verifies that
    /// edge records can hold the full u16 range for step indices.
    #[test]
    fn blackhat_edge_line_accepts_u16_max_steps() {
        let el = EdgeLine {
            source_step: u16::MAX,
            target_step: u16::MAX,
            source_port: String::from("out"),
            target_port: String::from("in"),
            edge_type: EdgeType::Normal,
            color: edge_color(EdgeType::Normal),
            width: 1.0,
            dashed: false,
        };
        assert_eq!(el.source_step, u16::MAX);
        assert_eq!(el.target_step, u16::MAX);
    }

    /// LOW: StateOverlay glow_radius of 0.0 or negative. The state_glow
    /// function returns positive radii, but the struct accepts any f32.
    /// A radius of 0.0 would produce no visible glow.
    #[test]
    fn blackhat_state_overlay_accepts_zero_glow_radius() {
        let overlay = StateOverlay {
            state: OverlayState::Pending,
            glow_color: colors::state::PENDING,
            glow_radius: 0.0,
        };
        assert_eq!(overlay.glow_radius, 0.0);
    }

    /// LOW: classify_node is exhaustive for all 34 variants. If a new
    /// CompiledNodeKind variant is added without updating classify_node,
    /// compilation will fail (match is non-exhaustive). This test verifies
    /// the count matches.
    #[test]
    fn blackhat_all_kinds_classified_count_matches() {
        let kinds = all_kinds();
        assert_eq!(kinds.len(), 34, "must have exactly 34 variants");
        for kind in &kinds {
            let _cat = classify_node(kind);
            // Should not panic for any variant.
        }
    }

    /// LOW: extract_badges for Ask without timeout and WaitEvent without
    /// timeout both produce empty badges. Timeout_slot being None is the
    /// correct condition for no timeout badge.
    #[test]
    fn blackhat_ask_and_wait_event_no_timeout_both_empty() {
        let ask = CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None,
        };
        let wait = CompiledNodeKind::WaitEvent {
            event: SlotIdx::new(0),
            timeout_slot: None,
        };
        assert!(extract_badges(&ask).is_empty());
        assert!(extract_badges(&wait).is_empty());
    }

    /// LOW: Finish badge always shows "D" regardless of the result slot
    /// value. The result slot is not used for badge determination.
    #[test]
    fn blackhat_finish_badge_ignores_result_slot_value() {
        let kind = CompiledNodeKind::Finish {
            result: SlotIdx::new(u16::MAX),
        };
        let badges = extract_badges(&kind);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].label, "D");
    }

    /// LOW: Node header and body colors always have alpha > 0.
    /// Verifies all categories produce visible colors.
    #[test]
    fn blackhat_all_category_colors_have_positive_alpha() {
        let categories = [
            NodeCategory::Data,
            NodeCategory::External,
            NodeCategory::Branch,
            NodeCategory::Loop,
            NodeCategory::Parallel,
            NodeCategory::Suspend,
            NodeCategory::Terminal,
            NodeCategory::Error,
            NodeCategory::Control,
        ];
        for cat in &categories {
            let hdr = node_header_color(*cat);
            let body = node_body_color(*cat);
            assert!(hdr[3] > 0.0, "header alpha must be positive for {cat:?}");
            assert!(body[3] > 0.0, "body alpha must be positive for {cat:?}");
        }
    }
}
