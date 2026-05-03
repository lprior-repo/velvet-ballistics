//! Maps VB CompiledNodeKind (34 variants) to visual properties for the canvas.
//!
//! Each variant maps to:
//! - Shape (rectangle, diamond, hexagon, etc.)
//! - Color per primitive type
//! - Size hints (width, height)
//! - Badge text (action name, retry count, timeout)
//! - Icon hint

use vb_core::workflow::CompiledNodeKind;

use crate::theme::colors;

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

/// Icon hint for a workflow node, used by the renderer to select an icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconHint {
    /// No icon.
    None,
    /// Data input/output.
    Data,
    /// Copy operation.
    Copy,
    /// Expression evaluation.
    Expression,
    /// Object construction.
    Object,
    /// List construction.
    List,
    /// External action.
    Action,
    /// Branch / decision.
    Branch,
    /// Loop / iteration.
    Loop,
    /// Parallel execution.
    Parallel,
    /// Retry / repeat.
    Retry,
    /// Wait / suspend.
    Wait,
    /// Ask / prompt.
    Ask,
    /// Error handler.
    Error,
    /// Jump / goto.
    Jump,
    /// Terminal / finish.
    Terminal,
    /// No-op.
    Nop,
}

// ---------------------------------------------------------------------------
// Visual properties struct
// ---------------------------------------------------------------------------

/// Complete visual properties for a workflow node.
#[derive(Debug, Clone)]
pub struct NodeVisuals {
    /// Shape to draw.
    pub shape: NodeShape,
    /// Header fill color (RGBA, 0-1 float).
    pub header_color: [f32; 4],
    /// Body fill color (RGBA, 0-1 float).
    pub body_color: [f32; 4],
    /// Border color (RGBA, 0-1 float).
    pub border_color: [f32; 4],
    /// Text color (RGBA, 0-1 float).
    pub text_color: [f32; 4],
    /// Width hint in pixels.
    pub width_hint: f64,
    /// Height hint in pixels.
    pub height_hint: f64,
    /// Badge labels (e.g. "A0", "R3", "T").
    pub badges: Vec<Badge>,
    /// Icon hint for the renderer.
    pub icon: IconHint,
}

// ---------------------------------------------------------------------------
// Badge struct
// ---------------------------------------------------------------------------

/// A small annotation badge on a node.
#[derive(Debug, Clone)]
pub struct Badge {
    /// Badge display text.
    pub label: String,
    /// Badge background color.
    pub color: [f32; 4],
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
/// Diamond nodes are slightly taller.
pub const DIAMOND_HEIGHT: f64 = 100.0;
/// Hexagon nodes are wider for parallel constructs.
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
// Main mapping function
// ---------------------------------------------------------------------------

/// Map a `CompiledNodeKind` to its visual properties.
///
/// Returns shape, colors, size hints, badges, and icon hint for rendering.
#[must_use]
pub fn map_node(kind: &CompiledNodeKind) -> NodeVisuals {
    match kind {
        CompiledNodeKind::Nop => NodeVisuals {
            shape: NodeShape::Rectangle,
            header_color: colors::node_header::CONTROL,
            body_color: colors::node_category::CONTROL,
            border_color: colors::bg::BORDER,
            text_color: colors::text::DIM,
            width_hint: DEFAULT_WIDTH,
            height_hint: DEFAULT_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Nop,
        },

        CompiledNodeKind::SetConst { .. } => NodeVisuals {
            shape: NodeShape::Rectangle,
            header_color: colors::node_header::DATA,
            body_color: colors::node_category::DATA,
            border_color: colors::bg::BORDER,
            text_color: colors::text::PRIMARY,
            width_hint: DEFAULT_WIDTH,
            height_hint: DEFAULT_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Data,
        },

        CompiledNodeKind::Copy { .. } => NodeVisuals {
            shape: NodeShape::Rectangle,
            header_color: colors::node_header::DATA,
            body_color: colors::node_category::DATA,
            border_color: colors::bg::BORDER,
            text_color: colors::text::PRIMARY,
            width_hint: DEFAULT_WIDTH,
            height_hint: DEFAULT_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Copy,
        },

        CompiledNodeKind::EvalExpr { .. } => NodeVisuals {
            shape: NodeShape::Rectangle,
            header_color: colors::node_header::DATA,
            body_color: colors::node_category::DATA,
            border_color: colors::bg::BORDER,
            text_color: colors::text::PRIMARY,
            width_hint: DEFAULT_WIDTH,
            height_hint: DEFAULT_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Expression,
        },

        CompiledNodeKind::BuildObject { .. } => NodeVisuals {
            shape: NodeShape::Rectangle,
            header_color: colors::node_header::DATA,
            body_color: colors::node_category::DATA,
            border_color: colors::bg::BORDER,
            text_color: colors::text::PRIMARY,
            width_hint: DEFAULT_WIDTH,
            height_hint: DEFAULT_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Object,
        },

        CompiledNodeKind::BuildList { .. } => NodeVisuals {
            shape: NodeShape::Rectangle,
            header_color: colors::node_header::DATA,
            body_color: colors::node_category::DATA,
            border_color: colors::bg::BORDER,
            text_color: colors::text::PRIMARY,
            width_hint: DEFAULT_WIDTH,
            height_hint: DEFAULT_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::List,
        },

        CompiledNodeKind::Do { action, .. } => NodeVisuals {
            shape: NodeShape::Rectangle,
            header_color: colors::node_header::EXTERNAL,
            body_color: colors::node_category::EXTERNAL,
            border_color: colors::neon::ORANGE,
            text_color: colors::text::PRIMARY,
            width_hint: DEFAULT_WIDTH,
            height_hint: DEFAULT_HEIGHT,
            badges: vec![
                Badge {
                    label: format!("A{}", action.get()),
                    color: colors::neon::ORANGE,
                },
                Badge {
                    label: String::from("S"),
                    color: colors::neon::MAGENTA,
                },
            ],
            icon: IconHint::Action,
        },

        CompiledNodeKind::Choose { .. } | CompiledNodeKind::ChooseSlot { .. } => NodeVisuals {
            shape: NodeShape::Diamond,
            header_color: colors::node_header::BRANCH,
            body_color: colors::node_category::BRANCH,
            border_color: colors::neon::PURPLE,
            text_color: colors::text::PRIMARY,
            width_hint: DIAMOND_WIDTH,
            height_hint: DIAMOND_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Branch,
        },

        CompiledNodeKind::ForEachStart { .. }
        | CompiledNodeKind::ForEachNext { .. }
        | CompiledNodeKind::ForEachJoin { .. } => NodeVisuals {
            shape: NodeShape::Rectangle,
            header_color: colors::node_header::LOOP,
            body_color: colors::node_category::LOOP,
            border_color: colors::neon::BLUE,
            text_color: colors::text::PRIMARY,
            width_hint: DEFAULT_WIDTH,
            height_hint: DEFAULT_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Loop,
        },

        CompiledNodeKind::TogetherStart { .. }
        | CompiledNodeKind::TogetherBranch { .. }
        | CompiledNodeKind::TogetherJoin { .. } => NodeVisuals {
            shape: NodeShape::Hexagon,
            header_color: colors::node_header::PARALLEL,
            body_color: colors::node_category::PARALLEL,
            border_color: colors::neon::TEAL,
            text_color: colors::text::PRIMARY,
            width_hint: HEXAGON_WIDTH,
            height_hint: HEXAGON_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Parallel,
        },

        CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::CollectPage { .. }
        | CompiledNodeKind::CollectNext { .. }
        | CompiledNodeKind::CollectFinish { .. } => NodeVisuals {
            shape: NodeShape::Rectangle,
            header_color: colors::node_header::COLLECT,
            body_color: colors::node_category::COLLECT,
            border_color: colors::neon::BLUE,
            text_color: colors::text::PRIMARY,
            width_hint: DEFAULT_WIDTH,
            height_hint: DEFAULT_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Loop,
        },

        CompiledNodeKind::ReduceStart { .. }
        | CompiledNodeKind::ReduceNext { .. }
        | CompiledNodeKind::ReduceFinish { .. } => NodeVisuals {
            shape: NodeShape::Rectangle,
            header_color: colors::node_header::REDUCE,
            body_color: colors::node_category::REDUCE,
            border_color: colors::neon::BLUE,
            text_color: colors::text::PRIMARY,
            width_hint: DEFAULT_WIDTH,
            height_hint: DEFAULT_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Loop,
        },

        CompiledNodeKind::RepeatStart { max_attempts, .. } => NodeVisuals {
            shape: NodeShape::Rectangle,
            header_color: colors::node_header::ERROR,
            body_color: colors::node_category::ERROR,
            border_color: colors::neon::YELLOW,
            text_color: colors::text::PRIMARY,
            width_hint: DEFAULT_WIDTH,
            height_hint: DEFAULT_HEIGHT,
            badges: vec![Badge {
                label: format!("R{}", max_attempts),
                color: colors::neon::YELLOW,
            }],
            icon: IconHint::Retry,
        },

        CompiledNodeKind::RepeatAttempt { .. }
        | CompiledNodeKind::RepeatCheck { .. }
        | CompiledNodeKind::RepeatFinish { .. } => NodeVisuals {
            shape: NodeShape::Rectangle,
            header_color: colors::node_header::ERROR,
            body_color: colors::node_category::ERROR,
            border_color: colors::neon::YELLOW,
            text_color: colors::text::PRIMARY,
            width_hint: DEFAULT_WIDTH,
            height_hint: DEFAULT_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Retry,
        },

        CompiledNodeKind::WaitUntil { .. } => NodeVisuals {
            shape: NodeShape::Pill,
            header_color: colors::node_header::SUSPEND,
            body_color: colors::node_category::SUSPEND,
            border_color: colors::neon::GREEN,
            text_color: colors::text::PRIMARY,
            width_hint: PILL_WIDTH,
            height_hint: PILL_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Wait,
        },

        CompiledNodeKind::WaitEvent {
            timeout_slot: Some(_),
            ..
        } => NodeVisuals {
            shape: NodeShape::Pill,
            header_color: colors::node_header::SUSPEND,
            body_color: colors::node_category::SUSPEND,
            border_color: colors::neon::GREEN,
            text_color: colors::text::PRIMARY,
            width_hint: PILL_WIDTH,
            height_hint: PILL_HEIGHT,
            badges: vec![Badge {
                label: String::from("T"),
                color: colors::neon::RED,
            }],
            icon: IconHint::Wait,
        },

        CompiledNodeKind::WaitEvent {
            timeout_slot: None,
            ..
        } => NodeVisuals {
            shape: NodeShape::Pill,
            header_color: colors::node_header::SUSPEND,
            body_color: colors::node_category::SUSPEND,
            border_color: colors::neon::GREEN,
            text_color: colors::text::PRIMARY,
            width_hint: PILL_WIDTH,
            height_hint: PILL_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Wait,
        },

        CompiledNodeKind::Ask {
            timeout_slot: Some(_),
            ..
        } => NodeVisuals {
            shape: NodeShape::Pill,
            header_color: colors::node_header::SUSPEND,
            body_color: colors::node_category::SUSPEND,
            border_color: colors::neon::YELLOW,
            text_color: colors::text::PRIMARY,
            width_hint: PILL_WIDTH,
            height_hint: PILL_HEIGHT,
            badges: vec![Badge {
                label: String::from("T"),
                color: colors::neon::RED,
            }],
            icon: IconHint::Ask,
        },

        CompiledNodeKind::Ask {
            timeout_slot: None, ..
        } => NodeVisuals {
            shape: NodeShape::Pill,
            header_color: colors::node_header::SUSPEND,
            body_color: colors::node_category::SUSPEND,
            border_color: colors::neon::YELLOW,
            text_color: colors::text::PRIMARY,
            width_hint: PILL_WIDTH,
            height_hint: PILL_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Ask,
        },

        CompiledNodeKind::AskResume { .. } => NodeVisuals {
            shape: NodeShape::Pill,
            header_color: colors::node_header::SUSPEND,
            body_color: colors::node_category::SUSPEND,
            border_color: colors::neon::YELLOW,
            text_color: colors::text::PRIMARY,
            width_hint: PILL_WIDTH,
            height_hint: PILL_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Ask,
        },

        CompiledNodeKind::RetryCheck { .. } => NodeVisuals {
            shape: NodeShape::Octagon,
            header_color: colors::node_header::ERROR,
            body_color: colors::node_category::ERROR,
            border_color: colors::neon::RED,
            text_color: colors::text::PRIMARY,
            width_hint: OCTAGON_WIDTH,
            height_hint: OCTAGON_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Retry,
        },

        CompiledNodeKind::ErrorHandler { .. } => NodeVisuals {
            shape: NodeShape::Octagon,
            header_color: colors::node_header::ERROR,
            body_color: colors::node_category::ERROR,
            border_color: colors::neon::RED,
            text_color: colors::text::PRIMARY,
            width_hint: OCTAGON_WIDTH,
            height_hint: OCTAGON_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Error,
        },

        CompiledNodeKind::Jump { .. } => NodeVisuals {
            shape: NodeShape::Arrow,
            header_color: colors::node_header::CONTROL,
            body_color: colors::node_category::CONTROL,
            border_color: colors::bg::BORDER_BRIGHT,
            text_color: colors::text::ACCENT,
            width_hint: ARROW_WIDTH,
            height_hint: ARROW_HEIGHT,
            badges: Vec::new(),
            icon: IconHint::Jump,
        },

        CompiledNodeKind::Finish { .. } => NodeVisuals {
            shape: NodeShape::Circle,
            header_color: colors::node_header::TERMINAL,
            body_color: colors::node_category::TERMINAL,
            border_color: colors::neon::TEAL,
            text_color: colors::text::PRIMARY,
            width_hint: CIRCLE_SIZE,
            height_hint: CIRCLE_SIZE,
            badges: vec![Badge {
                label: String::from("D"),
                color: colors::neon::TEAL,
            }],
            icon: IconHint::Terminal,
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx};

    /// Build all 34 CompiledNodeKind variants for exhaustive testing.
    fn all_kinds() -> Vec<CompiledNodeKind> {
        vec![
            CompiledNodeKind::Nop,
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
            CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
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
                initial: ConstIdx::new(0),
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
            CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: Some(SlotIdx::new(1)),
            },
            CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
            CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: Some(SlotIdx::new(1)),
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
    fn all_34_variants_produce_valid_visuals() {
        let kinds = all_kinds();
        assert_eq!(
            kinds.len(),
            34,
            "must exercise all 34 CompiledNodeKind variants"
        );

        for kind in &kinds {
            let v = map_node(kind);
            // Colors must have valid alpha.
            assert!(
                v.header_color[3] > 0.0,
                "header alpha must be positive for {kind:?}"
            );
            assert!(
                v.body_color[3] > 0.0,
                "body alpha must be positive for {kind:?}"
            );
            assert!(
                v.border_color[3] > 0.0,
                "border alpha must be positive for {kind:?}"
            );
            assert!(
                v.text_color[3] > 0.0,
                "text alpha must be positive for {kind:?}"
            );
            // Size hints must be positive.
            assert!(v.width_hint > 0.0, "width must be positive for {kind:?}");
            assert!(
                v.height_hint > 0.0,
                "height must be positive for {kind:?}"
            );
        }
    }

    #[test]
    fn choose_is_diamond() {
        let kind = CompiledNodeKind::Choose {
            branches: Box::new([]),
            otherwise: None,
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Diamond);
    }

    #[test]
    fn choose_slot_is_diamond() {
        let kind = CompiledNodeKind::ChooseSlot {
            branches: Box::new([]),
            otherwise: None,
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Diamond);
    }

    #[test]
    fn together_start_is_hexagon() {
        let kind = CompiledNodeKind::TogetherStart {
            branches: Box::new([]),
            join: StepIdx::new(0),
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Hexagon);
    }

    #[test]
    fn together_branch_is_hexagon() {
        let kind = CompiledNodeKind::TogetherBranch {
            branch: 0,
            entry: StepIdx::new(1),
            join: StepIdx::new(2),
            accumulator: SlotIdx::new(0),
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Hexagon);
    }

    #[test]
    fn together_join_is_hexagon() {
        let kind = CompiledNodeKind::TogetherJoin {
            branch_count: 1,
            accumulator: SlotIdx::new(0),
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Hexagon);
    }

    #[test]
    fn wait_until_is_pill() {
        let kind = CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(0),
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Pill);
    }

    #[test]
    fn wait_event_is_pill() {
        let kind = CompiledNodeKind::WaitEvent {
            event: SlotIdx::new(0),
            timeout_slot: None,
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Pill);
    }

    #[test]
    fn ask_is_pill() {
        let kind = CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None,
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Pill);
    }

    #[test]
    fn ask_resume_is_pill() {
        let kind = CompiledNodeKind::AskResume {
            answer: SlotIdx::new(0),
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Pill);
    }

    #[test]
    fn error_handler_is_octagon() {
        let kind = CompiledNodeKind::ErrorHandler {
            body: StepIdx::new(1),
            handler: StepIdx::new(2),
            error_slot: None,
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Octagon);
    }

    #[test]
    fn retry_check_is_octagon() {
        let kind = CompiledNodeKind::RetryCheck {
            policy_slot: SlotIdx::new(0),
            body: StepIdx::new(1),
            exhausted: StepIdx::new(2),
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Octagon);
    }

    #[test]
    fn finish_is_circle() {
        let kind = CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Circle);
    }

    #[test]
    fn jump_is_arrow() {
        let kind = CompiledNodeKind::Jump {
            target: StepIdx::new(1),
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Arrow);
    }

    #[test]
    fn do_node_has_action_and_secret_badges() {
        let kind = CompiledNodeKind::Do {
            action: ActionId::new(42),
            input: SlotIdx::new(0),
        };
        let v = map_node(&kind);
        assert_eq!(v.badges.len(), 2);
        assert_eq!(v.badges[0].label, "A42");
        assert_eq!(v.badges[1].label, "S");
    }

    #[test]
    fn repeat_start_has_retry_badge() {
        let kind = CompiledNodeKind::RepeatStart {
            max_attempts: 5,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let v = map_node(&kind);
        assert_eq!(v.badges.len(), 1);
        assert_eq!(v.badges[0].label, "R5");
    }

    #[test]
    fn wait_event_with_timeout_has_timeout_badge() {
        let kind = CompiledNodeKind::WaitEvent {
            event: SlotIdx::new(0),
            timeout_slot: Some(SlotIdx::new(1)),
        };
        let v = map_node(&kind);
        assert_eq!(v.badges.len(), 1);
        assert_eq!(v.badges[0].label, "T");
    }

    #[test]
    fn wait_event_without_timeout_has_no_badges() {
        let kind = CompiledNodeKind::WaitEvent {
            event: SlotIdx::new(0),
            timeout_slot: None,
        };
        let v = map_node(&kind);
        assert!(v.badges.is_empty());
    }

    #[test]
    fn ask_with_timeout_has_timeout_badge() {
        let kind = CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: Some(SlotIdx::new(1)),
        };
        let v = map_node(&kind);
        assert_eq!(v.badges.len(), 1);
        assert_eq!(v.badges[0].label, "T");
    }

    #[test]
    fn ask_without_timeout_has_no_badges() {
        let kind = CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None,
        };
        let v = map_node(&kind);
        assert!(v.badges.is_empty());
    }

    #[test]
    fn finish_has_durable_badge() {
        let kind = CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        };
        let v = map_node(&kind);
        assert_eq!(v.badges.len(), 1);
        assert_eq!(v.badges[0].label, "D");
    }

    #[test]
    fn nop_has_no_badges() {
        let v = map_node(&CompiledNodeKind::Nop);
        assert!(v.badges.is_empty());
    }

    #[test]
    fn nop_has_nop_icon() {
        let v = map_node(&CompiledNodeKind::Nop);
        assert_eq!(v.icon, IconHint::Nop);
    }

    #[test]
    fn do_node_has_action_icon() {
        let kind = CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        };
        assert_eq!(map_node(&kind).icon, IconHint::Action);
    }

    #[test]
    fn choose_has_branch_icon() {
        let kind = CompiledNodeKind::Choose {
            branches: Box::new([]),
            otherwise: None,
        };
        assert_eq!(map_node(&kind).icon, IconHint::Branch);
    }

    #[test]
    fn foreach_start_has_loop_icon() {
        let kind = CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 10,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        assert_eq!(map_node(&kind).icon, IconHint::Loop);
    }

    #[test]
    fn together_start_has_parallel_icon() {
        let kind = CompiledNodeKind::TogetherStart {
            branches: Box::new([]),
            join: StepIdx::new(0),
        };
        assert_eq!(map_node(&kind).icon, IconHint::Parallel);
    }

    #[test]
    fn repeat_start_has_retry_icon() {
        let kind = CompiledNodeKind::RepeatStart {
            max_attempts: 3,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        assert_eq!(map_node(&kind).icon, IconHint::Retry);
    }

    #[test]
    fn wait_until_has_wait_icon() {
        let kind = CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(0),
        };
        assert_eq!(map_node(&kind).icon, IconHint::Wait);
    }

    #[test]
    fn ask_has_ask_icon() {
        let kind = CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None,
        };
        assert_eq!(map_node(&kind).icon, IconHint::Ask);
    }

    #[test]
    fn error_handler_has_error_icon() {
        let kind = CompiledNodeKind::ErrorHandler {
            body: StepIdx::new(1),
            handler: StepIdx::new(2),
            error_slot: None,
        };
        assert_eq!(map_node(&kind).icon, IconHint::Error);
    }

    #[test]
    fn jump_has_jump_icon() {
        let kind = CompiledNodeKind::Jump {
            target: StepIdx::new(1),
        };
        assert_eq!(map_node(&kind).icon, IconHint::Jump);
    }

    #[test]
    fn finish_has_terminal_icon() {
        let kind = CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        };
        assert_eq!(map_node(&kind).icon, IconHint::Terminal);
    }

    #[test]
    fn circle_size_is_square() {
        let kind = CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        };
        let v = map_node(&kind);
        assert_eq!(v.width_hint, CIRCLE_SIZE);
        assert_eq!(v.height_hint, CIRCLE_SIZE);
    }

    #[test]
    fn diamond_dimensions_match_constants() {
        let kind = CompiledNodeKind::Choose {
            branches: Box::new([]),
            otherwise: None,
        };
        let v = map_node(&kind);
        assert_eq!(v.width_hint, DIAMOND_WIDTH);
        assert_eq!(v.height_hint, DIAMOND_HEIGHT);
    }

    #[test]
    fn hexagon_dimensions_match_constants() {
        let kind = CompiledNodeKind::TogetherStart {
            branches: Box::new([]),
            join: StepIdx::new(0),
        };
        let v = map_node(&kind);
        assert_eq!(v.width_hint, HEXAGON_WIDTH);
        assert_eq!(v.height_hint, HEXAGON_HEIGHT);
    }

    #[test]
    fn pill_dimensions_match_constants() {
        let kind = CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(0),
        };
        let v = map_node(&kind);
        assert_eq!(v.width_hint, PILL_WIDTH);
        assert_eq!(v.height_hint, PILL_HEIGHT);
    }

    #[test]
    fn data_variants_use_data_colors() {
        let data_kinds: Vec<CompiledNodeKind> = vec![
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
            CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
            CompiledNodeKind::BuildObject {
                fields: Box::new([]),
            },
            CompiledNodeKind::BuildList {
                items: Box::new([]),
            },
        ];
        for kind in &data_kinds {
            let v = map_node(kind);
            assert_eq!(v.header_color, colors::node_header::DATA, "for {kind:?}");
            assert_eq!(v.body_color, colors::node_category::DATA, "for {kind:?}");
        }
    }

    #[test]
    fn each_shape_category_is_consistent() {
        // Rectangles: Nop, SetConst, Copy, EvalExpr, BuildObject, BuildList, ForEach*, Collect*, Reduce*, Repeat*
        let kind = CompiledNodeKind::Nop;
        assert_eq!(map_node(&kind).shape, NodeShape::Rectangle);

        // Diamonds: Choose, ChooseSlot
        let kind = CompiledNodeKind::Choose {
            branches: Box::new([]),
            otherwise: None,
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Diamond);

        // Hexagons: Together*
        let kind = CompiledNodeKind::TogetherStart {
            branches: Box::new([]),
            join: StepIdx::new(0),
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Hexagon);

        // Pills: Wait*, Ask*, AskResume
        let kind = CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(0),
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Pill);

        // Octagons: ErrorHandler, RetryCheck
        let kind = CompiledNodeKind::ErrorHandler {
            body: StepIdx::new(1),
            handler: StepIdx::new(2),
            error_slot: None,
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Octagon);

        // Circle: Finish
        let kind = CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Circle);

        // Arrow: Jump
        let kind = CompiledNodeKind::Jump {
            target: StepIdx::new(1),
        };
        assert_eq!(map_node(&kind).shape, NodeShape::Arrow);
    }

    #[test]
    fn nop_uses_dim_text() {
        let v = map_node(&CompiledNodeKind::Nop);
        assert_eq!(v.text_color, colors::text::DIM);
    }

    #[test]
    fn jump_uses_accent_text() {
        let kind = CompiledNodeKind::Jump {
            target: StepIdx::new(1),
        };
        assert_eq!(map_node(&kind).text_color, colors::text::ACCENT);
    }

    #[test]
    fn setconst_icon_is_data() {
        let kind = CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        };
        assert_eq!(map_node(&kind).icon, IconHint::Data);
    }

    #[test]
    fn copy_icon_is_copy() {
        let kind = CompiledNodeKind::Copy {
            source: SlotIdx::new(0),
        };
        assert_eq!(map_node(&kind).icon, IconHint::Copy);
    }

    #[test]
    fn evalexpr_icon_is_expression() {
        let kind = CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(0),
        };
        assert_eq!(map_node(&kind).icon, IconHint::Expression);
    }

    #[test]
    fn buildobject_icon_is_object() {
        let kind = CompiledNodeKind::BuildObject {
            fields: Box::new([]),
        };
        assert_eq!(map_node(&kind).icon, IconHint::Object);
    }

    #[test]
    fn buildlist_icon_is_list() {
        let kind = CompiledNodeKind::BuildList {
            items: Box::new([]),
        };
        assert_eq!(map_node(&kind).icon, IconHint::List);
    }

    #[test]
    fn do_node_border_is_orange() {
        let kind = CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        };
        assert_eq!(map_node(&kind).border_color, colors::neon::ORANGE);
    }

    #[test]
    fn choose_border_is_purple() {
        let kind = CompiledNodeKind::Choose {
            branches: Box::new([]),
            otherwise: None,
        };
        assert_eq!(map_node(&kind).border_color, colors::neon::PURPLE);
    }

    #[test]
    fn together_border_is_teal() {
        let kind = CompiledNodeKind::TogetherStart {
            branches: Box::new([]),
            join: StepIdx::new(0),
        };
        assert_eq!(map_node(&kind).border_color, colors::neon::TEAL);
    }

    #[test]
    fn error_handler_border_is_red() {
        let kind = CompiledNodeKind::ErrorHandler {
            body: StepIdx::new(1),
            handler: StepIdx::new(2),
            error_slot: None,
        };
        assert_eq!(map_node(&kind).border_color, colors::neon::RED);
    }

    #[test]
    fn finish_border_is_teal() {
        let kind = CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        };
        assert_eq!(map_node(&kind).border_color, colors::neon::TEAL);
    }
}
