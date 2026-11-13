#![forbid(unsafe_code)]
//! Whole-source `for_each.at_once` profile selection and layout geometry.

use vb_compile::{CompileError, CompileErrors, compile_workflow};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowIrProfile};
use vb_core::{SlotIdx, StepIdx};

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: parallel_for_each\nwhen: {manual: {}}\n";

fn source(at_once: Option<&str>) -> String {
    let at_once = at_once.map_or_else(String::new, |value| format!("      at_once: {value}\n"));
    format!(
        "{HEADER}inputs:\n  items: {{is: list, of: any, max: 4}}\nsteps:\n  - id: seed\n    set: {{output: base, value: '42'}}\n  - id: outer\n    for_each:\n      input: '$input.items'\n      variable: entry\n{at_once}      steps:\n        - id: leaf\n          set: {{output: picked, value: '$entry'}}\n  - id: done\n    finish: {{result: '$base'}}\n"
    )
}

fn nested_source(inner_at_once: &str) -> String {
    format!(
        "{HEADER}inputs:\n  outer_items: {{is: list, of: any, max: 4}}\n  inner_items: {{is: list, of: any, max: 4}}\nsteps:\n  - id: seed\n    set: {{output: base, value: '42'}}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n        - id: inner\n          for_each:\n            input: '$input.inner_items'\n            variable: sub\n            at_once: {inner_at_once}\n            steps:\n              - id: leaf\n                set: {{output: picked, value: '$sub'}}\n  - id: done\n    finish: {{result: '$base'}}\n"
    )
}

fn compile(text: &str) -> Result<CompiledWorkflow, CompileErrors> {
    compile_workflow(text.as_bytes())
}

fn nodes(workflow: &CompiledWorkflow) -> Vec<&CompiledNode> {
    (0..workflow.node_count())
        .filter_map(|index| workflow.node(StepIdx::new(index)))
        .collect()
}

fn find<'a, F>(workflow: &'a CompiledWorkflow, predicate: F) -> Option<&'a CompiledNode>
where
    F: Fn(&CompiledNodeKind) -> bool,
{
    nodes(workflow)
        .into_iter()
        .find(|node| predicate(&node.kind))
}

fn concurrent_start(node: &CompiledNode) -> Option<(SlotIdx, u16, StepIdx, StepIdx)> {
    match node.kind {
        CompiledNodeKind::ForEachConcurrentStart {
            input,
            at_once,
            body,
            join,
            ..
        } => Some((input, at_once.get(), body, join)),
        _ => None,
    }
}

#[test]
fn serial_default_and_explicit_one_keep_legacy_profile() -> Result<(), String> {
    let default = compile(&source(None)).map_err(|errors| format!("{errors:?}"))?;
    let explicit = compile(&source(Some("1"))).map_err(|errors| format!("{errors:?}"))?;

    assert_eq!(default.ir_profile(), WorkflowIrProfile::LegacyV2);
    assert_eq!(explicit.ir_profile(), WorkflowIrProfile::LegacyV2);
    assert_eq!(default.digest(), explicit.digest());
    assert_eq!(nodes(&default).len(), 9);
    assert!(
        find(&default, |kind| matches!(
            kind,
            CompiledNodeKind::ForEachStart { .. }
        ))
        .is_some()
    );
    assert!(
        find(&default, |kind| matches!(
            kind,
            CompiledNodeKind::ForEachConcurrentStart { .. }
        ))
        .is_none()
    );
    Ok(())
}

#[test]
fn at_once_two_selects_concurrent_profile_and_geometry() -> Result<(), String> {
    let workflow = compile(&source(Some("2"))).map_err(|errors| format!("{errors:?}"))?;

    assert_eq!(workflow.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    assert_eq!(nodes(&workflow).len(), 6);
    assert!(
        find(&workflow, |kind| matches!(
            kind,
            CompiledNodeKind::ForEachStart { .. }
        ))
        .is_none()
    );
    let start = find(&workflow, |kind| {
        matches!(kind, CompiledNodeKind::ForEachConcurrentStart { .. })
    })
    .ok_or("missing concurrent start")?;
    let (_, at_once, body, join) = concurrent_start(start).ok_or("start kind")?;
    assert_eq!(at_once, 2);
    assert_eq!(body, StepIdx::new(2));
    assert_eq!(join, StepIdx::new(4));

    let body_node = workflow.node(body).ok_or("missing body")?;
    let body_output = body_node.output.slot().ok_or("missing body output")?;
    let lane_end = workflow.node(StepIdx::new(3)).ok_or("missing lane end")?;
    assert_eq!(
        lane_end.kind,
        CompiledNodeKind::ConcurrentLaneEnd {
            owner: start.id,
            result: Some(body_output),
        }
    );
    assert_eq!(lane_end.routing.next.step(), Some(join));
    let group_join = workflow.node(join).ok_or("missing join")?;
    assert_eq!(
        group_join.kind,
        CompiledNodeKind::ConcurrentGroupJoin { owner: start.id }
    );
    assert_eq!(group_join.routing.next.step(), Some(StepIdx::new(5)));
    Ok(())
}

#[test]
fn raw_at_once_stays_in_digest_while_capacity_clamps_to_contract() -> Result<(), String> {
    let trimmed = compile(&source(Some("65"))).map_err(|errors| format!("{errors:?}"))?;
    let raw_max = compile(&source(Some("4294967295"))).map_err(|errors| format!("{errors:?}"))?;

    assert_ne!(trimmed.digest(), raw_max.digest());
    for workflow in [&trimmed, &raw_max] {
        assert_eq!(workflow.ir_profile(), WorkflowIrProfile::ConcurrentV3);
        let start = find(workflow, |kind| {
            matches!(kind, CompiledNodeKind::ForEachConcurrentStart { .. })
        })
        .ok_or("missing concurrent start")?;
        let (_, at_once, _, _) = concurrent_start(start).ok_or("start kind")?;
        assert_eq!(at_once, 64);
    }
    Ok(())
}

#[test]
fn nested_parallel_for_each_lowers_at_its_own_position() -> Result<(), String> {
    let workflow = compile(&nested_source("3")).map_err(|errors| format!("{errors:?}"))?;

    assert_eq!(workflow.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    let serial = find(&workflow, |kind| matches!(kind, CompiledNodeKind::ForEachStart { .. }))
        .ok_or("missing serial outer")?;
    let concurrent = find(&workflow, |kind| {
        matches!(kind, CompiledNodeKind::ForEachConcurrentStart { .. })
    })
    .ok_or("missing concurrent inner")?;
    assert!(concurrent.id > serial.id);
    let (_, at_once, _, _) = concurrent_start(concurrent).ok_or("start kind")?;
    assert_eq!(at_once, 3);
    Ok(())
}

#[test]
fn zero_at_once_keeps_exact_shape_diagnostic() {
    let errors = compile(&source(Some("0"))).expect_err("zero at_once must fail");

    assert!(errors.0.iter().any(|error| matches!(
        error,
        CompileError::StepFieldShape {
            field: "for_each.at_once",
            expected: "a positive concurrency upper bound",
            ..
        }
    )));
}
