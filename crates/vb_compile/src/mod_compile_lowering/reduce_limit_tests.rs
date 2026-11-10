use super::*;
use vb_core::{CompiledInputConstraint, InputSlotKind};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn reduce_limit_tracks_declared_input_through_copies() -> TestResult {
    let mut builder = SlotCompiler::new();
    builder.record_input_slot(
        SlotIdx::new(0),
        InputSlotKind::List,
        CompiledInputConstraint {
            max: Some(2),
            ..CompiledInputConstraint::required()
        },
    );
    for (step, source, output) in [(0, 0, 1), (1, 1, 2)] {
        builder.push_node(node(
            StepIdx::new(step),
            CompiledNodeKind::Copy {
                source: SlotIdx::new(source),
            },
            Some(SlotIdx::new(output)),
            None,
        ));
    }
    for slot in [0, 1, 2] {
        assert_eq!(input_limit(SlotIdx::new(slot), &builder)?, 2);
    }
    Ok(())
}

#[test]
fn unknown_or_overwritten_reduce_origin_keeps_the_hard_cap() -> TestResult {
    let max = u32::try_from(vb_core::limits::MAX_LIST_ITEMS_PER_VALUE)?;
    let mut builder = SlotCompiler::new();
    assert_eq!(input_limit(SlotIdx::new(0), &builder)?, max);
    builder.record_input_slot(
        SlotIdx::new(0),
        InputSlotKind::List,
        CompiledInputConstraint {
            max: Some(2),
            ..CompiledInputConstraint::required()
        },
    );
    builder.push_node(node(
        StepIdx::new(0),
        CompiledNodeKind::BuildList {
            items: Box::new([]),
        },
        Some(SlotIdx::new(0)),
        None,
    ));
    assert_eq!(input_limit(SlotIdx::new(0), &builder)?, max);
    Ok(())
}

#[test]
fn canonical_reduce_emits_declared_bound_and_unknown_fallback() -> TestResult {
    for (schema, expected) in [
        ("is: list, of: number, max: 2", 2),
        (
            "is: list, of: number",
            u32::try_from(vb_core::limits::MAX_LIST_ITEMS_PER_VALUE)?,
        ),
    ] {
        let source = format!(
            "version: velvet-ballistics/v1\nname: reduce_limit\nwhen: {{manual: {{}}}}\ninputs: {{items: {{{schema}}}}}\nsteps:\n  - id: sum\n    reduce:\n      variable: accumulator\n      input: $input.items\n      initial: '0'\n      steps:\n        - id: increment\n          set: {{output: accumulator, value: '$accumulator + 1'}}\n  - id: done\n    finish: {{result: '$sum'}}\n"
        );
        let workflow = crate::compile_workflow(source.as_bytes())?;
        let limits: Vec<_> = workflow
            .to_parts()
            .nodes
            .iter()
            .filter_map(|node| match node.kind {
                CompiledNodeKind::ReduceStart { limit, .. } => Some(limit),
                _ => None,
            })
            .collect();
        assert_eq!(limits, vec![expected]);
    }
    Ok(())
}

#[test]
fn legacy_reduce_lowering_keeps_the_hard_cap() -> TestResult {
    let mut builder = SlotCompiler::new();
    let nodes = crate::lower_reduce(
        StepIdx::new(0),
        SlotIdx::new(0),
        SlotIdx::new(1),
        vb_core::ConstIdx::new(0),
        StepIdx::new(1),
        StepIdx::new(2),
        &mut builder,
    )?;
    let start = nodes.first().ok_or("missing reduce start")?;
    assert!(
        matches!(start.kind, CompiledNodeKind::ReduceStart { limit, .. }
        if usize::try_from(limit)? == vb_core::limits::MAX_LIST_ITEMS_PER_VALUE)
    );
    Ok(())
}
