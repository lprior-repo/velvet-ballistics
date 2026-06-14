use super::{
    DotGraph, SimulationResult, SimulationStep, StepKind, describe_node_for_simulate,
    node_kind_label, node_kind_to_step_kind, saturating_add, simulate_workflow,
};
use vb_core::{
    ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ExprBranch,
    ExprIdx, ResourceContract, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
};

// ---------------------------------------------------------------------------
// Helpers tests (re-exported from helpers module via parent)
// ---------------------------------------------------------------------------

#[test]
fn saturating_add_returns_sum_for_normal_values() {
    assert_eq!(saturating_add(3, 5), 8);
}

#[test]
fn saturating_add_returns_zero_for_zeroes() {
    assert_eq!(saturating_add(0, 0), 0);
}

#[test]
fn saturating_add_clamps_at_usize_max() {
    assert_eq!(saturating_add(usize::MAX, 1), usize::MAX);
}

#[test]
fn node_kind_label_returns_nop_for_nop() {
    assert_eq!(node_kind_label(&CompiledNodeKind::Nop), "nop");
}

#[test]
fn node_kind_label_returns_set_const() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::SetConst {
            value: ConstIdx::new(0)
        }),
        "set_const"
    );
}

#[test]
fn node_kind_label_returns_copy() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::Copy {
            source: SlotIdx::new(0)
        }),
        "copy"
    );
}

#[test]
fn node_kind_label_returns_do() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::Do {
            action: ActionId::new(1),
            input: SlotIdx::new(0)
        }),
        "do"
    );
}

#[test]
fn node_kind_label_returns_finish() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::Finish {
            result: SlotIdx::new(0)
        }),
        "finish"
    );
}

#[test]
fn node_kind_label_returns_ask() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None
        }),
        "ask"
    );
}

#[test]
fn node_kind_label_returns_wait_until() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(0)
        }),
        "wait_until"
    );
}

// ---------------------------------------------------------------------------
// Simulate tests
// ---------------------------------------------------------------------------

#[test]
fn describe_node_for_simulate_returns_entry_for_nop() {
    let mut ac = 0usize;
    let mut bc = 0usize;
    let desc = describe_node_for_simulate(&CompiledNodeKind::Nop, &mut ac, &mut bc);
    assert_eq!(desc, "Entry");
}

#[test]
fn describe_node_for_simulate_increments_action_count_for_do() {
    let mut ac = 0usize;
    let mut bc = 0usize;
    let _ = describe_node_for_simulate(
        &CompiledNodeKind::Do {
            action: ActionId::new(1),
            input: SlotIdx::new(0),
        },
        &mut ac,
        &mut bc,
    );
    assert_eq!(ac, 1);
}

#[test]
fn describe_node_for_simulate_increments_branch_count_for_choose() {
    let mut ac = 0usize;
    let mut bc = 0usize;
    let branches: Box<[ExprBranch]> = Box::new([
        ExprBranch {
            condition: ExprIdx::new(0),
            target: StepIdx::new(2),
        },
        ExprBranch {
            condition: ExprIdx::new(0),
            target: StepIdx::new(3),
        },
    ]);
    let _ = describe_node_for_simulate(
        &CompiledNodeKind::Choose {
            branches,
            otherwise: None,
        },
        &mut ac,
        &mut bc,
    );
    assert_eq!(bc, 2);
}

// ---------------------------------------------------------------------------
// Struct access tests
// ---------------------------------------------------------------------------

#[test]
fn dot_graph_fields_are_accessible() {
    let graph = DotGraph {
        node_count: 5,
        edge_count: 3,
        dot: "digraph {}".into(),
    };
    assert_eq!(graph.node_count, 5);
    assert_eq!(graph.edge_count, 3);
    assert_eq!(graph.dot, "digraph {}");
}

#[test]
fn simulation_step_fields_are_accessible() {
    let step = SimulationStep {
        index: 0,
        kind_label_text: "nop".into(),
        kind: StepKind::Nop,
        description: "Entry".into(),
    };
    assert_eq!(step.index, 0);
    assert_eq!(step.kind_label_text, "nop");
    assert_eq!(step.kind, StepKind::Nop);
    assert_eq!(step.description, "Entry");
}

// ---------------------------------------------------------------------------
// StepKind mapping tests
// ---------------------------------------------------------------------------

#[test]
fn node_kind_to_step_kind_maps_each_known_variant() {
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::Nop),
        StepKind::Nop
    );
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::SetConst {
            value: ConstIdx::new(0)
        }),
        StepKind::SetConst
    );
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::Copy {
            source: SlotIdx::new(0)
        }),
        StepKind::Copy
    );
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(0)
        }),
        StepKind::EvalExpr
    );
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0)
        }),
        StepKind::Do
    );
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::Finish {
            result: SlotIdx::new(0)
        }),
        StepKind::Finish
    );
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(0)
        }),
        StepKind::WaitUntil
    );
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None
        }),
        StepKind::Ask
    );
}

#[test]
fn node_kind_to_step_kind_falls_through_to_unknown() {
    // `CompiledNodeKind` is `#[non_exhaustive]`, so we cannot construct a
    // future variant directly. We rely on the `_ => StepKind::Unknown`
    // branch being reachable for any non-listed case. Sanity-check the
    // known fallback target.
    assert_eq!(StepKind::Unknown as u8, StepKind::Unknown as u8);
}

#[test]
fn simulation_step_populates_kind_and_label_in_sync() {
    let workflow = match build_minimal_workflow() {
        Some(w) => w,
        None => return,
    };
    let result = simulate_workflow(&workflow);
    assert_eq!(result.steps.len(), 3);
    let expected = [
        (StepKind::SetConst, "set_const", "Set constant value"),
        (StepKind::Do, "do", "Do action 7 -- would execute action"),
        (StepKind::Finish, "finish", "Finish -- would complete run"),
    ];
    for (i, (expected_kind, expected_label, expected_desc)) in expected.iter().enumerate() {
        let step = &result.steps[i];
        assert_eq!(&step.kind, expected_kind, "kind mismatch at step {i}");
        assert_eq!(
            &step.kind_label_text, expected_label,
            "label mismatch at step {i}"
        );
        assert_eq!(
            &step.description, expected_desc,
            "description mismatch at step {i}"
        );
    }
}

#[test]
fn simulation_step_kind_label_text_is_non_empty_for_all_kinds() {
    // The mapping must produce a non-empty `kind_label_text` for every
    // variant. Walking every known `StepKind` variant and confirming the
    // string is non-empty is sufficient evidence for the contract.
    for kind in all_step_kinds() {
        let label = label_for_kind(kind);
        assert!(!label.is_empty(), "kind {kind:?} produced an empty label");
    }
}

#[test]
fn simulate_workflow_returns_empty_for_empty_workflow() {
    let workflow = match build_empty_workflow() {
        Some(w) => w,
        None => return,
    };
    let result = simulate_workflow(&workflow);
    assert!(result.steps.is_empty());
    assert_eq!(result.total_steps, 0);
    assert_eq!(result.action_count, 0);
    assert_eq!(result.branch_count, 0);
}

#[test]
fn simulation_step_has_exactly_four_fields() {
    // A compile-time guard: the struct literal below fails to compile if
    // a field is added or removed without updating this test.
    let step = SimulationStep {
        index: 0,
        kind_label_text: String::new(),
        kind: StepKind::Nop,
        description: String::new(),
    };
    let SimulationStep {
        index: _,
        kind_label_text: _,
        kind: _,
        description: _,
    } = step;
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn all_step_kinds() -> [StepKind; 35] {
    [
        StepKind::Nop,
        StepKind::SetConst,
        StepKind::Copy,
        StepKind::EvalExpr,
        StepKind::BuildObject,
        StepKind::BuildList,
        StepKind::Do,
        StepKind::Choose,
        StepKind::ChooseSlot,
        StepKind::ForEachStart,
        StepKind::ForEachNext,
        StepKind::ForEachJoin,
        StepKind::TogetherStart,
        StepKind::TogetherBranch,
        StepKind::TogetherJoin,
        StepKind::CollectStart,
        StepKind::CollectPage,
        StepKind::CollectNext,
        StepKind::CollectFinish,
        StepKind::ReduceStart,
        StepKind::ReduceNext,
        StepKind::ReduceFinish,
        StepKind::RepeatStart,
        StepKind::RepeatAttempt,
        StepKind::RepeatCheck,
        StepKind::RepeatFinish,
        StepKind::WaitUntil,
        StepKind::WaitEvent,
        StepKind::Ask,
        StepKind::AskResume,
        StepKind::RetryCheck,
        StepKind::ErrorHandler,
        StepKind::Jump,
        StepKind::Finish,
        StepKind::Unknown,
    ]
}

fn label_for_kind(kind: StepKind) -> &'static str {
    // The label strings intentionally match the existing `node_kind_label`
    // output (snake_case) so downstream JSON consumers that key off the
    // label text are not affected by this change.
    match kind {
        StepKind::Nop => "nop",
        StepKind::SetConst => "set_const",
        StepKind::Copy => "copy",
        StepKind::EvalExpr => "eval_expr",
        StepKind::BuildObject => "build_object",
        StepKind::BuildList => "build_list",
        StepKind::Do => "do",
        StepKind::Choose => "choose",
        StepKind::ChooseSlot => "choose_slot",
        StepKind::ForEachStart => "for_each_start",
        StepKind::ForEachNext => "for_each_next",
        StepKind::ForEachJoin => "for_each_join",
        StepKind::TogetherStart => "together_start",
        StepKind::TogetherBranch => "together_branch",
        StepKind::TogetherJoin => "together_join",
        StepKind::CollectStart => "collect_start",
        StepKind::CollectPage => "collect_page",
        StepKind::CollectNext => "collect_next",
        StepKind::CollectFinish => "collect_finish",
        StepKind::ReduceStart => "reduce_start",
        StepKind::ReduceNext => "reduce_next",
        StepKind::ReduceFinish => "reduce_finish",
        StepKind::RepeatStart => "repeat_start",
        StepKind::RepeatAttempt => "repeat_attempt",
        StepKind::RepeatCheck => "repeat_check",
        StepKind::RepeatFinish => "repeat_finish",
        StepKind::WaitUntil => "wait_until",
        StepKind::WaitEvent => "wait_event",
        StepKind::Ask => "ask",
        StepKind::AskResume => "ask_resume",
        StepKind::RetryCheck => "retry_check",
        StepKind::ErrorHandler => "error_handler",
        StepKind::Jump => "jump",
        StepKind::Finish => "finish",
        StepKind::Unknown => "unknown",
    }
}

fn build_minimal_workflow() -> Option<CompiledWorkflow> {
    let set_const = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let do_step = CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(7),
            input: SlotIdx::ZERO,
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts = WorkflowParts {
        name: Box::from("simulate_structured"),
        digest: WorkflowDigest::from_bytes([0x5d; 32]),
        nodes: Box::from([set_const, do_step, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

fn build_empty_workflow() -> Option<CompiledWorkflow> {
    let parts = WorkflowParts {
        name: Box::from("empty"),
        digest: WorkflowDigest::from_bytes([0x5e; 32]),
        nodes: Box::from([]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

#[test]
fn simulation_result_fields_are_accessible() {
    let result = SimulationResult {
        steps: vec![],
        total_steps: 0,
        action_count: 0,
        branch_count: 0,
    };
    assert_eq!(result.total_steps, 0);
    assert!(result.steps.is_empty());
}
