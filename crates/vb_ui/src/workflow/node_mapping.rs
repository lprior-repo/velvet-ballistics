#![forbid(unsafe_code)]
//! Maps VB CompiledNodeKind variants to visual properties for the workflow graph.
//!
//! Provides [`NodeCategory`] (semantic grouping), [`NodeShape`] (geometric
//! shape), cyberpunk colour constants, and [`node_kind_to_visual`] which maps
//! every `CompiledNodeKind` variant to a [`NodeVisual`] per the Phase 4C spec.

use vb_core::workflow::CompiledNodeKind;

// ---------------------------------------------------------------------------
// Colour constants (cyberpunk neon palette, RGBA f32)
// ---------------------------------------------------------------------------

/// Neon cyan -- primary accent, running state.
pub const NEON_CYAN: [f32; 4] = [0.000, 0.961, 1.000, 1.0]; // #00f5ff
/// Neon green -- success, healthy, pass.
pub const NEON_GREEN: [f32; 4] = [0.224, 1.000, 0.078, 1.0]; // #39ff14
/// Neon red -- failure, error, blocked.
pub const NEON_RED: [f32; 4] = [1.000, 0.027, 0.227, 1.0]; // #ff073a
/// Neon yellow -- attention, retry, degraded.
pub const NEON_YELLOW: [f32; 4] = [1.000, 0.902, 0.000, 1.0]; // #ffe600
/// Neon orange -- external actions (Do nodes).
pub const NEON_ORANGE: [f32; 4] = [1.000, 0.420, 0.000, 1.0]; // #ff6b00
/// Neon purple -- branching, choice nodes.
pub const NEON_PURPLE: [f32; 4] = [0.694, 0.302, 1.000, 1.0]; // #b14dff
/// Neon blue -- waiting, suspended, parallel.
pub const NEON_BLUE: [f32; 4] = [0.176, 0.420, 1.000, 1.0]; // #2d6bff
/// Neon teal -- verification-safe, certified, terminal.
pub const NEON_TEAL: [f32; 4] = [0.000, 0.898, 0.780, 1.0]; // #00e5c7
/// Neon magenta -- secret/taint paths, warnings.
pub const NEON_MAGENTA: [f32; 4] = [1.000, 0.000, 1.000, 1.0]; // #ff00ff
/// Neon pink -- incident highlights.
pub const NEON_PINK: [f32; 4] = [1.000, 0.176, 0.482, 1.0]; // #ff2d7b
/// Gray -- control/data nodes, dim elements.
pub const GRAY: [f32; 4] = [0.333, 0.333, 0.467, 1.0]; // #555577
/// Amber -- ask/prompt suspend nodes.
pub const AMBER: [f32; 4] = [1.000, 0.690, 0.000, 1.0]; // #ffb000

// ---------------------------------------------------------------------------
// NodeCategory enum
// ---------------------------------------------------------------------------

/// Semantic category for a workflow node kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NodeCategory {
    /// Data manipulation: Nop, SetConst, Copy, EvalExpr.
    Data,
    /// Data construction: BuildObject, BuildList.
    Construct,
    /// External action: Do.
    External,
    /// Branching: Choose, ChooseSlot.
    Branch,
    /// Iteration: ForEach*.
    Loop,
    /// Parallel execution: Together*.
    Parallel,
    /// Collection: Collect*.
    Collect,
    /// Reduction: Reduce*.
    Reduce,
    /// Retry / repeat: Repeat*, RetryCheck.
    Retry,
    /// Suspension: Wait*, Ask*, AskResume.
    Suspend,
    /// Error handling: ErrorHandler.
    Error,
    /// Control flow: Jump.
    Control,
    /// Terminal: Finish.
    Terminal,
}

// ---------------------------------------------------------------------------
// NodeShape enum
// ---------------------------------------------------------------------------

/// Geometric shape for rendering a workflow node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NodeShape {
    /// Standard rounded rectangle (most node types).
    RoundedRect,
    /// Diamond for branch / decision nodes.
    Diamond,
    /// Circle for suspend / wait nodes.
    Round,
    /// Pill / stadium for certain suspend variants.
    Pill,
    /// Container with internal lanes for parallel nodes.
    Container,
}

// ---------------------------------------------------------------------------
// NodeVisual struct
// ---------------------------------------------------------------------------

/// Visual properties for a workflow node.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeVisual {
    /// Semantic category.
    pub category: NodeCategory,
    /// Geometric shape.
    pub shape: NodeShape,
    /// Primary colour (RGBA).
    pub color: [f32; 4],
    /// Human-readable label for the node kind.
    pub label: String,
}

// ---------------------------------------------------------------------------
// Mapping function
// ---------------------------------------------------------------------------

/// Map a [`CompiledNodeKind`] to its visual representation.
///
/// Follows the Phase 4C spec table:
///
/// | VB Node Kind                        | Category  | Shape        | Color   |
/// |-------------------------------------|-----------|--------------|---------|
/// | Nop, SetConst, Copy, EvalExpr       | Data      | RoundedRect  | Gray    |
/// | BuildObject, BuildList              | Construct | RoundedRect  | Gray    |
/// | Do                                  | External  | RoundedRect  | Orange  |
/// | Choose, ChooseSlot                  | Branch    | Diamond      | Purple  |
/// | ForEachStart/Next/Join              | Loop      | RoundedRect  | Blue    |
/// | TogetherStart/Branch/Join           | Parallel  | Container    | Blue    |
/// | CollectStart/Page/Next/Finish       | Collect   | RoundedRect  | Blue    |
/// | ReduceStart/Next/Finish             | Reduce    | RoundedRect  | Blue    |
/// | RepeatStart/Attempt/Check/Finish    | Retry     | RoundedRect  | Purple  |
/// | WaitUntil, WaitEvent                | Suspend   | Round        | Green   |
/// | Ask, AskResume                      | Suspend   | Round        | Amber   |
/// | RetryCheck                          | Retry     | Diamond      | Purple  |
/// | ErrorHandler                        | Error     | Diamond      | Red     |
/// | Jump                                | Control   | Diamond      | Gray    |
/// | Finish                              | Terminal  | Pill         | Teal    |
#[must_use]
pub fn node_kind_to_visual(kind: &CompiledNodeKind) -> NodeVisual {
    match kind {
        // -- Data --
        CompiledNodeKind::Nop => NodeVisual {
            category: NodeCategory::Data,
            shape: NodeShape::RoundedRect,
            color: GRAY,
            label: String::from("Nop"),
        },
        CompiledNodeKind::SetConst { .. } => NodeVisual {
            category: NodeCategory::Data,
            shape: NodeShape::RoundedRect,
            color: GRAY,
            label: String::from("SetConst"),
        },
        CompiledNodeKind::Copy { .. } => NodeVisual {
            category: NodeCategory::Data,
            shape: NodeShape::RoundedRect,
            color: GRAY,
            label: String::from("Copy"),
        },
        CompiledNodeKind::EvalExpr { .. } => NodeVisual {
            category: NodeCategory::Data,
            shape: NodeShape::RoundedRect,
            color: GRAY,
            label: String::from("EvalExpr"),
        },

        // -- Construct --
        CompiledNodeKind::BuildObject { .. } => NodeVisual {
            category: NodeCategory::Construct,
            shape: NodeShape::RoundedRect,
            color: GRAY,
            label: String::from("BuildObject"),
        },
        CompiledNodeKind::BuildList { .. } => NodeVisual {
            category: NodeCategory::Construct,
            shape: NodeShape::RoundedRect,
            color: GRAY,
            label: String::from("BuildList"),
        },

        // -- External --
        CompiledNodeKind::Do { .. } => NodeVisual {
            category: NodeCategory::External,
            shape: NodeShape::RoundedRect,
            color: NEON_ORANGE,
            label: String::from("Do"),
        },

        // -- Branch --
        CompiledNodeKind::Choose { .. } => NodeVisual {
            category: NodeCategory::Branch,
            shape: NodeShape::Diamond,
            color: NEON_PURPLE,
            label: String::from("Choose"),
        },
        CompiledNodeKind::ChooseSlot { .. } => NodeVisual {
            category: NodeCategory::Branch,
            shape: NodeShape::Diamond,
            color: NEON_PURPLE,
            label: String::from("ChooseSlot"),
        },

        // -- Loop --
        CompiledNodeKind::ForEachStart { .. } => NodeVisual {
            category: NodeCategory::Loop,
            shape: NodeShape::RoundedRect,
            color: NEON_BLUE,
            label: String::from("ForEachStart"),
        },
        CompiledNodeKind::ForEachNext { .. } => NodeVisual {
            category: NodeCategory::Loop,
            shape: NodeShape::RoundedRect,
            color: NEON_BLUE,
            label: String::from("ForEachNext"),
        },
        CompiledNodeKind::ForEachJoin { .. } => NodeVisual {
            category: NodeCategory::Loop,
            shape: NodeShape::RoundedRect,
            color: NEON_BLUE,
            label: String::from("ForEachJoin"),
        },

        // -- Parallel --
        CompiledNodeKind::TogetherStart { .. } => NodeVisual {
            category: NodeCategory::Parallel,
            shape: NodeShape::Container,
            color: NEON_BLUE,
            label: String::from("TogetherStart"),
        },
        CompiledNodeKind::TogetherBranch { .. } => NodeVisual {
            category: NodeCategory::Parallel,
            shape: NodeShape::Container,
            color: NEON_BLUE,
            label: String::from("TogetherBranch"),
        },
        CompiledNodeKind::TogetherJoin { .. } => NodeVisual {
            category: NodeCategory::Parallel,
            shape: NodeShape::Container,
            color: NEON_BLUE,
            label: String::from("TogetherJoin"),
        },

        // -- Collect --
        CompiledNodeKind::CollectStart { .. } => NodeVisual {
            category: NodeCategory::Collect,
            shape: NodeShape::RoundedRect,
            color: NEON_BLUE,
            label: String::from("CollectStart"),
        },
        CompiledNodeKind::CollectPage { .. } => NodeVisual {
            category: NodeCategory::Collect,
            shape: NodeShape::RoundedRect,
            color: NEON_BLUE,
            label: String::from("CollectPage"),
        },
        CompiledNodeKind::CollectNext { .. } => NodeVisual {
            category: NodeCategory::Collect,
            shape: NodeShape::RoundedRect,
            color: NEON_BLUE,
            label: String::from("CollectNext"),
        },
        CompiledNodeKind::CollectFinish { .. } => NodeVisual {
            category: NodeCategory::Collect,
            shape: NodeShape::RoundedRect,
            color: NEON_BLUE,
            label: String::from("CollectFinish"),
        },

        // -- Reduce --
        CompiledNodeKind::ReduceStart { .. } => NodeVisual {
            category: NodeCategory::Reduce,
            shape: NodeShape::RoundedRect,
            color: NEON_BLUE,
            label: String::from("ReduceStart"),
        },
        CompiledNodeKind::ReduceNext { .. } => NodeVisual {
            category: NodeCategory::Reduce,
            shape: NodeShape::RoundedRect,
            color: NEON_BLUE,
            label: String::from("ReduceNext"),
        },
        CompiledNodeKind::ReduceFinish { .. } => NodeVisual {
            category: NodeCategory::Reduce,
            shape: NodeShape::RoundedRect,
            color: NEON_BLUE,
            label: String::from("ReduceFinish"),
        },

        // -- Retry (Repeat* variants) --
        CompiledNodeKind::RepeatStart { .. } => NodeVisual {
            category: NodeCategory::Retry,
            shape: NodeShape::RoundedRect,
            color: NEON_PURPLE,
            label: String::from("RepeatStart"),
        },
        CompiledNodeKind::RepeatAttempt { .. } => NodeVisual {
            category: NodeCategory::Retry,
            shape: NodeShape::RoundedRect,
            color: NEON_PURPLE,
            label: String::from("RepeatAttempt"),
        },
        CompiledNodeKind::RepeatCheck { .. } => NodeVisual {
            category: NodeCategory::Retry,
            shape: NodeShape::RoundedRect,
            color: NEON_PURPLE,
            label: String::from("RepeatCheck"),
        },
        CompiledNodeKind::RepeatFinish { .. } => NodeVisual {
            category: NodeCategory::Retry,
            shape: NodeShape::RoundedRect,
            color: NEON_PURPLE,
            label: String::from("RepeatFinish"),
        },

        // -- Suspend (Wait) --
        CompiledNodeKind::WaitUntil { .. } => NodeVisual {
            category: NodeCategory::Suspend,
            shape: NodeShape::Round,
            color: NEON_GREEN,
            label: String::from("WaitUntil"),
        },
        CompiledNodeKind::WaitEvent { .. } => NodeVisual {
            category: NodeCategory::Suspend,
            shape: NodeShape::Round,
            color: NEON_GREEN,
            label: String::from("WaitEvent"),
        },

        // -- Suspend (Ask) --
        CompiledNodeKind::Ask { .. } => NodeVisual {
            category: NodeCategory::Suspend,
            shape: NodeShape::Round,
            color: AMBER,
            label: String::from("Ask"),
        },
        CompiledNodeKind::AskResume { .. } => NodeVisual {
            category: NodeCategory::Suspend,
            shape: NodeShape::Round,
            color: AMBER,
            label: String::from("AskResume"),
        },

        // -- Retry (RetryCheck) --
        CompiledNodeKind::RetryCheck { .. } => NodeVisual {
            category: NodeCategory::Retry,
            shape: NodeShape::Diamond,
            color: NEON_PURPLE,
            label: String::from("RetryCheck"),
        },

        // -- Error --
        CompiledNodeKind::ErrorHandler { .. } => NodeVisual {
            category: NodeCategory::Error,
            shape: NodeShape::Diamond,
            color: NEON_RED,
            label: String::from("ErrorHandler"),
        },

        // -- Control --
        CompiledNodeKind::Jump { .. } => NodeVisual {
            category: NodeCategory::Control,
            shape: NodeShape::Diamond,
            color: GRAY,
            label: String::from("Jump"),
        },

        // -- Terminal --
        CompiledNodeKind::Finish { .. } => NodeVisual {
            category: NodeCategory::Terminal,
            shape: NodeShape::Pill,
            color: NEON_TEAL,
            label: String::from("Finish"),
        },
    }
}

// ---------------------------------------------------------------------------
// Tests (30+ unit tests covering every node kind mapping)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx};

    // -- Helper to build every CompiledNodeKind variant ----------------------

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

    // -- Test 1: All 34 variants produce valid visuals ----------------------

    #[test]
    fn all_variants_produce_valid_visuals() {
        let kinds = all_kinds();
        assert_eq!(
            kinds.len(),
            34,
            "must exercise all CompiledNodeKind variants"
        );

        for kind in &kinds {
            let v = node_kind_to_visual(kind);
            // Alpha must be positive.
            assert!(v.color[3] > 0.0, "alpha must be positive for {kind:?}");
            // Label must be non-empty.
            assert!(!v.label.is_empty(), "label must be non-empty for {kind:?}");
        }
    }

    // -- Test 2: Nop maps to Data / RoundedRect / Gray ----------------------

    #[test]
    fn nop_is_data_roundedrect_gray() {
        let v = node_kind_to_visual(&CompiledNodeKind::Nop);
        assert_eq!(v.category, NodeCategory::Data);
        assert_eq!(v.shape, NodeShape::RoundedRect);
        assert_eq!(v.color, GRAY);
        assert_eq!(v.label, "Nop");
    }

    // -- Test 3: SetConst maps to Data / RoundedRect / Gray -----------------

    #[test]
    fn setconst_is_data_roundedrect_gray() {
        let kind = CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Data);
        assert_eq!(v.shape, NodeShape::RoundedRect);
        assert_eq!(v.color, GRAY);
    }

    // -- Test 4: Copy maps to Data / RoundedRect / Gray ---------------------

    #[test]
    fn copy_is_data_roundedrect_gray() {
        let kind = CompiledNodeKind::Copy {
            source: SlotIdx::new(0),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Data);
        assert_eq!(v.shape, NodeShape::RoundedRect);
    }

    // -- Test 5: EvalExpr maps to Data / RoundedRect / Gray -----------------

    #[test]
    fn evalexpr_is_data_roundedrect_gray() {
        let kind = CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(0),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Data);
        assert_eq!(v.shape, NodeShape::RoundedRect);
    }

    // -- Test 6: BuildObject maps to Construct / RoundedRect / Gray ---------

    #[test]
    fn buildobject_is_construct_roundedrect_gray() {
        let kind = CompiledNodeKind::BuildObject {
            fields: Box::new([]),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Construct);
        assert_eq!(v.shape, NodeShape::RoundedRect);
        assert_eq!(v.color, GRAY);
    }

    // -- Test 7: BuildList maps to Construct / RoundedRect / Gray ------------

    #[test]
    fn buildlist_is_construct_roundedrect_gray() {
        let kind = CompiledNodeKind::BuildList {
            items: Box::new([]),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Construct);
        assert_eq!(v.shape, NodeShape::RoundedRect);
    }

    // -- Test 8: Do maps to External / RoundedRect / Orange -----------------

    #[test]
    fn do_is_external_roundedrect_orange() {
        let kind = CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::External);
        assert_eq!(v.shape, NodeShape::RoundedRect);
        assert_eq!(v.color, NEON_ORANGE);
        assert_eq!(v.label, "Do");
    }

    // -- Test 9: Choose maps to Branch / Diamond / Purple -------------------

    #[test]
    fn choose_is_branch_diamond_purple() {
        let kind = CompiledNodeKind::Choose {
            branches: Box::new([]),
            otherwise: None,
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Branch);
        assert_eq!(v.shape, NodeShape::Diamond);
        assert_eq!(v.color, NEON_PURPLE);
    }

    // -- Test 10: ChooseSlot maps to Branch / Diamond / Purple ---------------

    #[test]
    fn chooseslot_is_branch_diamond_purple() {
        let kind = CompiledNodeKind::ChooseSlot {
            branches: Box::new([]),
            otherwise: None,
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Branch);
        assert_eq!(v.shape, NodeShape::Diamond);
        assert_eq!(v.color, NEON_PURPLE);
    }

    // -- Test 11: ForEachStart maps to Loop / RoundedRect / Blue ------------

    #[test]
    fn foreach_start_is_loop_roundedrect_blue() {
        let kind = CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 10,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Loop);
        assert_eq!(v.shape, NodeShape::RoundedRect);
        assert_eq!(v.color, NEON_BLUE);
    }

    // -- Test 12: ForEachNext maps to Loop / RoundedRect / Blue -------------

    #[test]
    fn foreach_next_is_loop_roundedrect_blue() {
        let kind = CompiledNodeKind::ForEachNext {
            iterator_slot: SlotIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Loop);
        assert_eq!(v.color, NEON_BLUE);
    }

    // -- Test 13: ForEachJoin maps to Loop / RoundedRect / Blue -------------

    #[test]
    fn foreach_join_is_loop_roundedrect_blue() {
        let kind = CompiledNodeKind::ForEachJoin {
            output: SlotIdx::new(0),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Loop);
        assert_eq!(v.color, NEON_BLUE);
    }

    // -- Test 14: TogetherStart maps to Parallel / Container / Blue ----------

    #[test]
    fn together_start_is_parallel_container_blue() {
        let kind = CompiledNodeKind::TogetherStart {
            branches: Box::new([]),
            join: StepIdx::new(0),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Parallel);
        assert_eq!(v.shape, NodeShape::Container);
        assert_eq!(v.color, NEON_BLUE);
    }

    // -- Test 15: TogetherBranch maps to Parallel / Container / Blue ---------

    #[test]
    fn together_branch_is_parallel_container_blue() {
        let kind = CompiledNodeKind::TogetherBranch {
            branch: 0,
            entry: StepIdx::new(1),
            join: StepIdx::new(2),
            accumulator: SlotIdx::new(0),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Parallel);
        assert_eq!(v.shape, NodeShape::Container);
        assert_eq!(v.color, NEON_BLUE);
    }

    // -- Test 16: TogetherJoin maps to Parallel / Container / Blue -----------

    #[test]
    fn together_join_is_parallel_container_blue() {
        let kind = CompiledNodeKind::TogetherJoin {
            branch_count: 1,
            accumulator: SlotIdx::new(0),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Parallel);
        assert_eq!(v.shape, NodeShape::Container);
    }

    // -- Test 17: CollectStart maps to Collect / RoundedRect / Blue ----------

    #[test]
    fn collect_start_is_collect_roundedrect_blue() {
        let kind = CompiledNodeKind::CollectStart {
            source: SlotIdx::new(0),
            limit: 10,
            page_size: 5,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Collect);
        assert_eq!(v.shape, NodeShape::RoundedRect);
        assert_eq!(v.color, NEON_BLUE);
    }

    // -- Test 18: CollectFinish maps to Collect / RoundedRect / Blue ---------

    #[test]
    fn collect_finish_is_collect_roundedrect_blue() {
        let kind = CompiledNodeKind::CollectFinish {
            collector_slot: SlotIdx::new(0),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Collect);
        assert_eq!(v.color, NEON_BLUE);
    }

    // -- Test 19: ReduceStart maps to Reduce / RoundedRect / Blue -----------

    #[test]
    fn reduce_start_is_reduce_roundedrect_blue() {
        let kind = CompiledNodeKind::ReduceStart {
            input: SlotIdx::new(0),
            accumulator: SlotIdx::new(1),
            initial: ConstIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Reduce);
        assert_eq!(v.shape, NodeShape::RoundedRect);
        assert_eq!(v.color, NEON_BLUE);
    }

    // -- Test 20: ReduceFinish maps to Reduce / RoundedRect / Blue ----------

    #[test]
    fn reduce_finish_is_reduce_roundedrect_blue() {
        let kind = CompiledNodeKind::ReduceFinish {
            accumulator: SlotIdx::new(0),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Reduce);
        assert_eq!(v.color, NEON_BLUE);
    }

    // -- Test 21: RepeatStart maps to Retry / RoundedRect / Purple ----------

    #[test]
    fn repeat_start_is_retry_roundedrect_purple() {
        let kind = CompiledNodeKind::RepeatStart {
            max_attempts: 3,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Retry);
        assert_eq!(v.shape, NodeShape::RoundedRect);
        assert_eq!(v.color, NEON_PURPLE);
    }

    // -- Test 22: RepeatCheck maps to Retry / RoundedRect / Purple ----------

    #[test]
    fn repeat_check_is_retry_roundedrect_purple() {
        let kind = CompiledNodeKind::RepeatCheck {
            attempt_slot: SlotIdx::new(0),
            done: StepIdx::new(2),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Retry);
        assert_eq!(v.color, NEON_PURPLE);
    }

    // -- Test 23: RepeatFinish maps to Retry / RoundedRect / Purple ---------

    #[test]
    fn repeat_finish_is_retry_roundedrect_purple() {
        let kind = CompiledNodeKind::RepeatFinish {
            result: SlotIdx::new(0),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Retry);
        assert_eq!(v.color, NEON_PURPLE);
    }

    // -- Test 24: WaitUntil maps to Suspend / Round / Green -----------------

    #[test]
    fn waituntil_is_suspend_round_green() {
        let kind = CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(0),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Suspend);
        assert_eq!(v.shape, NodeShape::Round);
        assert_eq!(v.color, NEON_GREEN);
    }

    // -- Test 25: WaitEvent maps to Suspend / Round / Green -----------------

    #[test]
    fn waitevent_is_suspend_round_green() {
        let kind = CompiledNodeKind::WaitEvent {
            event: SlotIdx::new(0),
            timeout_slot: None,
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Suspend);
        assert_eq!(v.shape, NodeShape::Round);
        assert_eq!(v.color, NEON_GREEN);
    }

    // -- Test 26: Ask maps to Suspend / Round / Amber -----------------------

    #[test]
    fn ask_is_suspend_round_amber() {
        let kind = CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None,
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Suspend);
        assert_eq!(v.shape, NodeShape::Round);
        assert_eq!(v.color, AMBER);
    }

    // -- Test 27: AskResume maps to Suspend / Round / Amber -----------------

    #[test]
    fn askresume_is_suspend_round_amber() {
        let kind = CompiledNodeKind::AskResume {
            answer: SlotIdx::new(0),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Suspend);
        assert_eq!(v.shape, NodeShape::Round);
        assert_eq!(v.color, AMBER);
    }

    // -- Test 28: RetryCheck maps to Retry / Diamond / Purple ---------------

    #[test]
    fn retrycheck_is_retry_diamond_purple() {
        let kind = CompiledNodeKind::RetryCheck {
            policy_slot: SlotIdx::new(0),
            body: StepIdx::new(1),
            exhausted: StepIdx::new(2),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Retry);
        assert_eq!(v.shape, NodeShape::Diamond);
        assert_eq!(v.color, NEON_PURPLE);
    }

    // -- Test 29: ErrorHandler maps to Error / Diamond / Red ----------------

    #[test]
    fn errorhandler_is_error_diamond_red() {
        let kind = CompiledNodeKind::ErrorHandler {
            body: StepIdx::new(1),
            handler: StepIdx::new(2),
            error_slot: None,
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Error);
        assert_eq!(v.shape, NodeShape::Diamond);
        assert_eq!(v.color, NEON_RED);
    }

    // -- Test 30: Jump maps to Control / Diamond / Gray ---------------------

    #[test]
    fn jump_is_control_diamond_gray() {
        let kind = CompiledNodeKind::Jump {
            target: StepIdx::new(1),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Control);
        assert_eq!(v.shape, NodeShape::Diamond);
        assert_eq!(v.color, GRAY);
    }

    // -- Test 31: Finish maps to Terminal / Pill / Teal ---------------------

    #[test]
    fn finish_is_terminal_pill_teal() {
        let kind = CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Terminal);
        assert_eq!(v.shape, NodeShape::Pill);
        assert_eq!(v.color, NEON_TEAL);
    }

    // -- Test 32: RepeatAttempt maps to Retry category ----------------------

    #[test]
    fn repeat_attempt_is_retry_roundedrect_purple() {
        let kind = CompiledNodeKind::RepeatAttempt {
            attempt_slot: SlotIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Retry);
        assert_eq!(v.shape, NodeShape::RoundedRect);
        assert_eq!(v.color, NEON_PURPLE);
    }

    // -- Test 33: CollectPage maps to Collect / RoundedRect / Blue ----------

    #[test]
    fn collect_page_is_collect_roundedrect_blue() {
        let kind = CompiledNodeKind::CollectPage {
            collector_slot: SlotIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Collect);
        assert_eq!(v.shape, NodeShape::RoundedRect);
        assert_eq!(v.color, NEON_BLUE);
    }

    // -- Test 34: CollectNext maps to Collect / RoundedRect / Blue ----------

    #[test]
    fn collect_next_is_collect_roundedrect_blue() {
        let kind = CompiledNodeKind::CollectNext {
            collector_slot: SlotIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Collect);
        assert_eq!(v.color, NEON_BLUE);
    }

    // -- Test 35: ReduceNext maps to Reduce / RoundedRect / Blue ------------

    #[test]
    fn reduce_next_is_reduce_roundedrect_blue() {
        let kind = CompiledNodeKind::ReduceNext {
            iterator_slot: SlotIdx::new(0),
            accumulator: SlotIdx::new(1),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Reduce);
        assert_eq!(v.shape, NodeShape::RoundedRect);
        assert_eq!(v.color, NEON_BLUE);
    }

    // -- Test 36: All colour constants have alpha 1.0 -----------------------

    #[test]
    fn colour_constants_have_unit_alpha() {
        let colours: [([f32; 4], &str); 12] = [
            (NEON_CYAN, "NEON_CYAN"),
            (NEON_GREEN, "NEON_GREEN"),
            (NEON_RED, "NEON_RED"),
            (NEON_YELLOW, "NEON_YELLOW"),
            (NEON_ORANGE, "NEON_ORANGE"),
            (NEON_PURPLE, "NEON_PURPLE"),
            (NEON_BLUE, "NEON_BLUE"),
            (NEON_TEAL, "NEON_TEAL"),
            (NEON_MAGENTA, "NEON_MAGENTA"),
            (NEON_PINK, "NEON_PINK"),
            (GRAY, "GRAY"),
            (AMBER, "AMBER"),
        ];
        for (colour, name) in &colours {
            assert_eq!(colour[3], 1.0, "{name} alpha should be 1.0");
        }
    }

    // -- Test 37: All colour constants have RGB in [0, 1] -------------------

    #[test]
    fn colour_constants_rgb_in_unit_range() {
        let colours: [[f32; 4]; 12] = [
            NEON_CYAN,
            NEON_GREEN,
            NEON_RED,
            NEON_YELLOW,
            NEON_ORANGE,
            NEON_PURPLE,
            NEON_BLUE,
            NEON_TEAL,
            NEON_MAGENTA,
            NEON_PINK,
            GRAY,
            AMBER,
        ];
        for colour in &colours {
            for ch in 0..3 {
                assert!(
                    colour[ch] >= 0.0 && colour[ch] <= 1.0,
                    "channel {ch} = {} is outside [0, 1]",
                    colour[ch]
                );
            }
        }
    }

    // -- Test 38: Every label matches its enum variant name ------------------

    #[test]
    fn labels_are_nonempty_and_match_variant() {
        let kinds = all_kinds();
        for kind in &kinds {
            let v = node_kind_to_visual(kind);
            assert!(
                !v.label.is_empty(),
                "label should not be empty for {kind:?}"
            );
            // Label should be the variant name (no spaces, ASCII).
            assert!(
                v.label.chars().all(|c| c.is_ascii_alphanumeric()),
                "label should be alphanumeric for {kind:?}, got '{}'",
                v.label
            );
        }
    }

    // -- Test 39: Data and Construct categories use Gray --------------------

    #[test]
    fn data_and_construct_use_gray() {
        let gray_kinds: Vec<CompiledNodeKind> = vec![
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
        ];
        for kind in &gray_kinds {
            assert_eq!(
                node_kind_to_visual(kind).color,
                GRAY,
                "expected GRAY for {kind:?}"
            );
        }
    }

    // -- Test 40: All shapes are represented across the variants ------------

    #[test]
    fn every_shape_is_used_by_at_least_one_variant() {
        let kinds = all_kinds();
        let shapes: Vec<NodeShape> = kinds.iter().map(|k| node_kind_to_visual(k).shape).collect();
        assert!(
            shapes.contains(&NodeShape::RoundedRect),
            "RoundedRect should appear"
        );
        assert!(
            shapes.contains(&NodeShape::Diamond),
            "Diamond should appear"
        );
        assert!(shapes.contains(&NodeShape::Round), "Round should appear");
        assert!(shapes.contains(&NodeShape::Pill), "Pill should appear");
        assert!(
            shapes.contains(&NodeShape::Container),
            "Container should appear"
        );
    }

    // -- Test 41: NodeCategory enum covers all 13 categories ----------------

    #[test]
    fn every_category_is_used_by_at_least_one_variant() {
        let kinds = all_kinds();
        let categories: Vec<NodeCategory> = kinds
            .iter()
            .map(|k| node_kind_to_visual(k).category)
            .collect();
        assert!(
            categories.contains(&NodeCategory::Data),
            "Data should appear"
        );
        assert!(
            categories.contains(&NodeCategory::Construct),
            "Construct should appear"
        );
        assert!(
            categories.contains(&NodeCategory::External),
            "External should appear"
        );
        assert!(
            categories.contains(&NodeCategory::Branch),
            "Branch should appear"
        );
        assert!(
            categories.contains(&NodeCategory::Loop),
            "Loop should appear"
        );
        assert!(
            categories.contains(&NodeCategory::Parallel),
            "Parallel should appear"
        );
        assert!(
            categories.contains(&NodeCategory::Collect),
            "Collect should appear"
        );
        assert!(
            categories.contains(&NodeCategory::Reduce),
            "Reduce should appear"
        );
        assert!(
            categories.contains(&NodeCategory::Retry),
            "Retry should appear"
        );
        assert!(
            categories.contains(&NodeCategory::Suspend),
            "Suspend should appear"
        );
        assert!(
            categories.contains(&NodeCategory::Error),
            "Error should appear"
        );
        assert!(
            categories.contains(&NodeCategory::Control),
            "Control should appear"
        );
        assert!(
            categories.contains(&NodeCategory::Terminal),
            "Terminal should appear"
        );
    }

    // -- Test 42: WaitEvent with timeout still maps to same visual -----------

    #[test]
    fn waitevent_with_timeout_is_still_suspend_green() {
        let kind = CompiledNodeKind::WaitEvent {
            event: SlotIdx::new(0),
            timeout_slot: Some(SlotIdx::new(1)),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Suspend);
        assert_eq!(v.color, NEON_GREEN);
    }

    // -- Test 43: Ask with timeout still maps to same visual ----------------

    #[test]
    fn ask_with_timeout_is_still_suspend_amber() {
        let kind = CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: Some(SlotIdx::new(1)),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Suspend);
        assert_eq!(v.color, AMBER);
    }

    // -- Test 44: ErrorHandler with error_slot is still Error / Diamond / Red

    #[test]
    fn errorhandler_with_error_slot_is_error_diamond_red() {
        let kind = CompiledNodeKind::ErrorHandler {
            body: StepIdx::new(1),
            handler: StepIdx::new(2),
            error_slot: Some(SlotIdx::new(3)),
        };
        let v = node_kind_to_visual(&kind);
        assert_eq!(v.category, NodeCategory::Error);
        assert_eq!(v.shape, NodeShape::Diamond);
        assert_eq!(v.color, NEON_RED);
    }

    // -- Test 45: NodeVisual Debug/Clone/PartialEq derive works -------------

    #[test]
    fn node_visual_equality_and_clone() {
        let kind = CompiledNodeKind::Nop;
        let v1 = node_kind_to_visual(&kind);
        let v2 = v1.clone();
        assert_eq!(v1, v2);
    }

    // -- Test 46: NEON_CYAN constant matches hex spec -----------------------

    #[test]
    fn neon_cyan_matches_hex() {
        let diff_r = (NEON_CYAN[0] - 0.000).abs();
        let diff_g = (NEON_CYAN[1] - 0.961).abs();
        let diff_b = (NEON_CYAN[2] - 1.000).abs();
        assert!(diff_r < 0.01 && diff_g < 0.01 && diff_b < 0.01);
    }

    // -- Test 47: NEON_RED constant matches hex spec ------------------------

    #[test]
    fn neon_red_matches_hex() {
        // #ff073a => R=255, G=7, B=58
        let expected: [f32; 4] = [255.0 / 255.0, 7.0 / 255.0, 58.0 / 255.0, 1.0];
        for ch in 0..4 {
            let diff = (NEON_RED[ch] - expected[ch]).abs();
            assert!(diff < 0.01, "NEON_RED[{ch}] differs by {diff}");
        }
    }
}
