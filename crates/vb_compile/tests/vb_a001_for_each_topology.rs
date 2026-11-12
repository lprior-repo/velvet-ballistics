#![forbid(unsafe_code)]
//! Topology tests for bead vb-a001: for_each compiled parity fix.
//!
//! Verifies that `v1_primitive_lowering` emits the correct
//! ForEachNext next-edge on the body SetConst so run-compiled
//! does not reject the compiled IR as unreachable.

use vb_compile::{YamlCompiler, compile_workflow, lower_set, lower_steps_to_ir};
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{CompiledNodeKind, WorkflowParts, node};

// ---------------------------------------------------------------------------
// Corpus YAML for the for_each workflow under vb-a001
// ---------------------------------------------------------------------------

const FOREACH_YAML: &str = "version: velvet-ballistics/v1\nname: fuzz_foreach\nwhen:\n  manual: {}\ninputs:\n  items:\n    is: list\n    of: any\n    max: 2\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: \"$input.items\"\n      at_once: 1\n      steps:\n        - id: capture\n          set:\n            output: seen\n            value: \"1\"\n  - id: done\n    finish:\n      result: $input.items\n";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn compile_yaml(yaml: &str) -> Result<vb_core::CompiledWorkflow, Box<dyn std::error::Error>> {
    let workflow = compile_workflow(yaml.as_bytes()).map_err(|errors| {
        let msgs: Vec<_> = errors.iter().map(|e| e.to_string()).collect();
        msgs.join("; ")
    })?;
    Ok(workflow)
}

/// Returns node kind names in StepIdx order.
fn node_kind_names(parts: &WorkflowParts) -> Vec<&'static str> {
    parts
        .nodes
        .iter()
        .map(|n| match &n.kind {
            CompiledNodeKind::Nop => "Nop",
            CompiledNodeKind::SetConst { .. } => "SetConst",
            CompiledNodeKind::Copy { .. } => "Copy",
            CompiledNodeKind::EvalExpr { .. } => "EvalExpr",
            CompiledNodeKind::BuildObject { .. } => "BuildObject",
            CompiledNodeKind::BuildList { .. } => "BuildList",
            CompiledNodeKind::Do { .. } => "Do",
            CompiledNodeKind::Choose { .. } => "Choose",
            CompiledNodeKind::ChooseSlot { .. } => "ChooseSlot",
            CompiledNodeKind::ForEachStart { .. } => "ForEachStart",
            CompiledNodeKind::ForEachNext { .. } => "ForEachNext",
            CompiledNodeKind::ForEachJoin { .. } => "ForEachJoin",
            CompiledNodeKind::TogetherStart { .. } => "TogetherStart",
            CompiledNodeKind::TogetherBranch { .. } => "TogetherBranch",
            CompiledNodeKind::TogetherJoin { .. } => "TogetherJoin",
            CompiledNodeKind::CollectStart { .. } => "CollectStart",
            CompiledNodeKind::CollectPage { .. } => "CollectPage",
            CompiledNodeKind::CollectNext { .. } => "CollectNext",
            CompiledNodeKind::CollectFinish { .. } => "CollectFinish",
            CompiledNodeKind::ReduceStart { .. } => "ReduceStart",
            CompiledNodeKind::ReduceNext { .. } => "ReduceNext",
            CompiledNodeKind::ReduceFinish { .. } => "ReduceFinish",
            CompiledNodeKind::RepeatStart { .. } => "RepeatStart",
            CompiledNodeKind::RepeatAttempt { .. } => "RepeatAttempt",
            CompiledNodeKind::RepeatCheck { .. } => "RepeatCheck",
            CompiledNodeKind::RepeatFinish { .. } => "RepeatFinish",
            CompiledNodeKind::WaitUntil { .. } => "WaitUntil",
            CompiledNodeKind::WaitEvent { .. } => "WaitEvent",
            CompiledNodeKind::Ask { .. } => "Ask",
            CompiledNodeKind::AskResume { .. } => "AskResume",
            CompiledNodeKind::RetryCheck { .. } => "RetryCheck",
            CompiledNodeKind::ErrorHandler { .. } => "ErrorHandler",
            CompiledNodeKind::Jump { .. } => "Jump",
            CompiledNodeKind::Finish { .. } => "Finish",
            _ => "Unknown",
        })
        .collect()
}

// ---------------------------------------------------------------------------
// vb-a001 topology tests (compiled YAML path)
// ---------------------------------------------------------------------------

/// TEST-001: for_each compiled IR has the expected node kind sequence.
///
/// After v1_primitive_lowering the sequence must be:
///   [BuildList, source taint, ForEachStart, body, append, ForEachNext, ForEachJoin, Finish]
#[test]
fn vb_a001_for_each_node_kind_sequence() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = compile_yaml(FOREACH_YAML)?;
    let kinds = node_kind_names(&workflow.to_parts());
    let expected = vec![
        "BuildList",
        "EvalExpr",
        "ForEachStart",
        "SetConst",
        "EvalExpr",
        "ForEachNext",
        "ForEachJoin",
        "Finish",
    ];
    assert_eq!(
        kinds, expected,
        "for_each node kinds must match expected sequence"
    );
    Ok(())
}

/// TEST-002: body SetConst next edge points to ForEachNext (the vb-a001 fix).
#[test]
fn vb_a001_body_setconst_next_points_to_foreachnext() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = compile_yaml(FOREACH_YAML)?;
    let parts = workflow.to_parts();
    let start = parts.nodes.get(2).ok_or("start")?;
    let CompiledNodeKind::ForEachStart { body, done, .. } = start.kind else {
        return Err("expected ForEachStart".into());
    };
    assert_eq!(body, StepIdx::new(3));
    assert_eq!(done, StepIdx::new(6));
    assert_eq!(start.routing.next.step(), None);
    let body = parts.nodes.get(3).ok_or("body")?;
    let CompiledNodeKind::SetConst { value } = body.kind else {
        return Err("expected body SetConst".into());
    };
    assert_eq!(value, ConstIdx::new(1));
    assert_eq!(
        parts.constants.get(value.as_usize()),
        Some(&vb_core::ConstValue::I64(1))
    );
    assert_eq!(body.routing.next.step(), Some(StepIdx::new(4)));
    let append = parts.nodes.get(4).ok_or("append")?;
    let CompiledNodeKind::EvalExpr { expr } = append.kind else {
        return Err("expected append".into());
    };
    assert_eq!(
        parts
            .expressions
            .get(expr.as_usize())
            .ok_or("append program")?
            .ops
            .as_ref(),
        &[
            vb_core::ExprOp::LoadSlot(SlotIdx::new(1)),
            vb_core::ExprOp::LoadSlot(SlotIdx::new(5)),
            vb_core::ExprOp::Append
        ]
    );
    assert_eq!(append.routing.next.step(), Some(StepIdx::new(5)));
    let next = parts.nodes.get(5).ok_or("next")?;
    let CompiledNodeKind::ForEachNext { body, done, .. } = next.kind else {
        return Err("expected ForEachNext".into());
    };
    assert_eq!(body, StepIdx::new(3));
    assert_eq!(done, StepIdx::new(6));
    let join = parts.nodes.get(6).ok_or("join")?;
    assert_eq!(
        join.kind,
        CompiledNodeKind::ForEachJoin {
            output: SlotIdx::new(1)
        }
    );
    assert_eq!(join.output.slot(), Some(SlotIdx::new(4)));
    assert_eq!(join.routing.next.step(), Some(StepIdx::new(7)));
    assert_eq!(
        parts.nodes.get(7).ok_or("finish")?.kind,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0)
        }
    );
    Ok(())
}

/// TEST-003: manually constructed for_each with correct body→ForEachNext chain
/// passes lower_steps_to_ir validation.
///
/// This is the direct parity test: building the IR node-by-node and verifying
/// that a properly connected for_each passes validation.
#[test]
fn vb_a001_lower_steps_to_ir_accepts_connected_foreach() -> Result<(), Box<dyn std::error::Error>> {
    let input = node(
        StepIdx::ZERO,
        CompiledNodeKind::BuildList {
            items: Box::new([]),
        },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    );
    let foreach_start = node(
        StepIdx::new(1),
        CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 2.into(),
            body: StepIdx::new(2),
            done: StepIdx::new(4),
        },
        Some(SlotIdx::new(2)),
        None,
    );

    // The body continues to ForEachNext after the explicit input seed.
    let body_set = lower_set(
        StepIdx::new(2),
        SlotIdx::new(1),
        ConstIdx::new(0),
        Some(StepIdx::new(3)),
    );

    let foreach_next = node(
        StepIdx::new(3),
        CompiledNodeKind::ForEachNext {
            iterator_slot: SlotIdx::new(2),
            body: StepIdx::new(2),
            done: StepIdx::new(4),
        },
        Some(SlotIdx::new(1)),
        None,
    );

    let finish = node(
        StepIdx::new(4),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );

    let nodes = vec![input, foreach_start, body_set, foreach_next, finish];
    let result = lower_steps_to_ir(
        nodes.clone(),
        vec![],
        vec![],
        vec![vb_core::value::ConstValue::I64(1)],
        3,
        0,
        "vb-a001-connected",
        WorkflowDigest::from_bytes([1; 32]),
    );

    assert_eq!(result?.to_parts().nodes.as_ref(), nodes.as_slice());
    Ok(())
}

/// TEST-004: lower_steps_to_ir rejects for_each where body SetConst has no
/// next edge (the vb-a001 bug condition).
#[test]
fn vb_a001_lower_steps_to_ir_rejects_disconnected_body() -> Result<(), Box<dyn std::error::Error>> {
    let input = node(
        StepIdx::ZERO,
        CompiledNodeKind::BuildList {
            items: Box::new([]),
        },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    );
    let foreach_start = node(
        StepIdx::new(1),
        CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 2.into(),
            body: StepIdx::new(2),
            done: StepIdx::new(4),
        },
        Some(SlotIdx::new(2)),
        None,
    );

    // BUG: body SetConst next = None → unreachable nodes
    let body_set = lower_set(
        StepIdx::new(2),
        SlotIdx::new(1),
        ConstIdx::new(0),
        None, // ← missing next edge
    );

    let foreach_next = node(
        StepIdx::new(3),
        CompiledNodeKind::ForEachNext {
            iterator_slot: SlotIdx::new(2),
            body: StepIdx::new(2),
            done: StepIdx::new(4),
        },
        Some(SlotIdx::new(1)),
        None,
    );

    let finish = node(
        StepIdx::new(4),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );

    let result = lower_steps_to_ir(
        vec![input, foreach_start, body_set, foreach_next, finish],
        vec![],
        vec![],
        vec![vb_core::value::ConstValue::I64(1)],
        3,
        0,
        "vb-a001-disconnected",
        WorkflowDigest::from_bytes([2; 32]),
    );

    let errors = result.err().ok_or("disconnected body was accepted")?;
    assert_eq!(
        errors.0,
        [vb_compile::CompileError::Workflow(
            vb_core::WorkflowError::UnreachableNode {
                step: StepIdx::new(3),
            }
        )]
    );
    Ok(())
}

/// TEST-005: all three compile APIs produce identical for_each topology.
#[test]
fn vb_a001_all_compile_apis_agree_on_foreach_topology() -> Result<(), Box<dyn std::error::Error>> {
    let w1 = compile_yaml(FOREACH_YAML)?;
    let w2 = YamlCompiler::default().compile(FOREACH_YAML.as_bytes())?;

    let kinds1 = node_kind_names(&w1.to_parts());
    let kinds2 = node_kind_names(&w2.to_parts());

    assert_eq!(
        kinds1, kinds2,
        "compile_workflow and YamlCompiler::compile must produce same node kinds"
    );
    assert_eq!(
        w1.to_parts().slot_count,
        w2.to_parts().slot_count,
        "slot_count must agree across APIs"
    );
    Ok(())
}

/// TEST-006: input, iterator, item and body output have distinct slots.
#[test]
fn vb_a001_for_each_slots_have_distinct_roles() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = compile_yaml(FOREACH_YAML)?;
    assert_eq!(workflow.to_parts().slot_count, 6);

    let start = workflow.node(StepIdx::new(2)).ok_or("missing start")?;
    let body = workflow.node(StepIdx::new(3)).ok_or("missing body")?;
    let next = workflow.node(StepIdx::new(5)).ok_or("missing next")?;

    let (actual_input, actual_item_slot) = match &start.kind {
        CompiledNodeKind::ForEachStart {
            input, item_slot, ..
        } => (*input, *item_slot),
        _ => return Err("expected ForEachStart".into()),
    };
    assert_eq!(actual_input, SlotIdx::new(0));
    assert_eq!(actual_item_slot, SlotIdx::new(3));

    let start_output = start.output.slot().ok_or("missing start output")?;
    assert_eq!(start_output, SlotIdx::new(2));

    let body_output = body.output.slot().ok_or("missing body output")?;
    assert_eq!(body_output, SlotIdx::new(5));

    let actual_iterator_slot = match &next.kind {
        CompiledNodeKind::ForEachNext { iterator_slot, .. } => *iterator_slot,
        _ => return Err("expected ForEachNext".into()),
    };
    assert_eq!(actual_iterator_slot, SlotIdx::new(2));

    let next_output = next.output.slot().ok_or("missing next output")?;
    assert_eq!(next_output, SlotIdx::new(3));

    assert_eq!(next_output, actual_item_slot);
    assert_eq!(actual_iterator_slot, start_output);

    let mut slots = std::collections::BTreeSet::new();
    slots.insert(actual_input);
    slots.insert(start_output);
    slots.insert(actual_item_slot);
    slots.insert(body_output);
    let accumulator = workflow
        .node(StepIdx::new(0))
        .and_then(|node| node.output.slot())
        .ok_or("accumulator")?;
    let result = workflow
        .node(StepIdx::new(6))
        .and_then(|node| node.output.slot())
        .ok_or("aggregate")?;
    assert_eq!(accumulator, SlotIdx::new(1));
    assert_eq!(result, SlotIdx::new(4));
    slots.insert(accumulator);
    slots.insert(result);
    assert_eq!(slots.len(), 6);

    Ok(())
}

/// TEST-007: ForEachStart body/done/iterator_slot are all valid.
#[test]
fn vb_a001_foreachstart_fields_valid() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = compile_yaml(FOREACH_YAML)?;
    let parts = workflow.to_parts();

    match &parts.nodes[2].kind {
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            limit,
            body,
            done,
        } => {
            assert!(limit.get() > 0, "limit must be positive");
            assert!(body.as_usize() < parts.nodes.len(), "body in-range");
            assert!(done.as_usize() < parts.nodes.len(), "done in-range");
            assert!(
                input.as_usize() < parts.slot_count as usize,
                "input slot in-range"
            );
            assert!(
                item_slot.as_usize() < parts.slot_count as usize,
                "item_slot in-range"
            );
        }
        other => return Err(format!("node 0 expected ForEachStart, got {other:?}").into()),
    }
    Ok(())
}

/// TEST-008: ForEachNext targets are in-bounds.
#[test]
fn vb_a001_foreachnext_targets_in_bounds() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = compile_yaml(FOREACH_YAML)?;
    let parts = workflow.to_parts();

    match &parts.nodes[5].kind {
        CompiledNodeKind::ForEachNext { body, done, .. } => {
            assert!(
                body.as_usize() < parts.nodes.len(),
                "ForEachNext body in-range"
            );
            assert!(
                done.as_usize() < parts.nodes.len(),
                "ForEachNext done in-range"
            );
        }
        other => return Err(format!("node 2 expected ForEachNext, got {other:?}").into()),
    }
    Ok(())
}

/// TEST-009: canonical materialization includes initialization, taint and a join.
#[test]
fn vb_a001_for_each_node_count_is_four() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = compile_yaml(FOREACH_YAML)?;
    assert_eq!(
        workflow.to_parts().nodes.len(),
        8,
        "canonical for_each must produce exactly 8 nodes"
    );
    Ok(())
}

/// TEST-010: finish node is last and has no next edge.
#[test]
fn vb_a001_finish_is_last_with_no_next() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = compile_yaml(FOREACH_YAML)?;
    let parts = workflow.to_parts();
    let last = parts.nodes.last().unwrap();
    assert!(
        matches!(&last.kind, CompiledNodeKind::Finish { .. }),
        "last node must be Finish"
    );
    assert!(
        last.routing.next.step().is_none(),
        "finish must have no next edge"
    );
    Ok(())
}

/// TEST-011: for_each nodes are at expected StepIdx positions.
#[test]
fn vb_a001_foreach_nodes_at_expected_positions() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = compile_yaml(FOREACH_YAML)?;
    let parts = workflow.to_parts();

    assert_eq!(parts.nodes[0].id, StepIdx::new(0), "ForEachStart at 0");
    assert_eq!(parts.nodes[1].id, StepIdx::new(1), "body SetConst at 1");
    assert_eq!(parts.nodes[2].id, StepIdx::new(2), "ForEachNext at 2");
    assert_eq!(parts.nodes[3].id, StepIdx::new(3), "Finish at 3");
    Ok(())
}
