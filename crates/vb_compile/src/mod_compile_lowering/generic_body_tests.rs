use super::*;
use crate::{CompileError, StepAst, StepPrimitive};
use proptest::prelude::*;

include!("generic_body_reference.rs");

fn step(index: usize, kind: u8) -> StepAst {
    let primitive = match kind {
        0 => StepPrimitive::Set {
            output: format!("v{index}"),
            value: "7".into(),
        },
        1 => StepPrimitive::Do {
            action: "1".into(),
            input: "0".into(),
        },
        2 => StepPrimitive::Wait {
            event: Some("0".into()),
            timeout: None,
        },
        3 => StepPrimitive::Do {
            action: "bad".into(),
            input: "0".into(),
        },
        4 => StepPrimitive::Set {
            output: format!("v{index}"),
            value: "$missing".into(),
        },
        5 => StepPrimitive::Set {
            output: "$input.x".into(),
            value: "7".into(),
        },
        6 => StepPrimitive::Ask {
            prompt: "0".into(),
            timeout: None,
        },
        _ => StepPrimitive::Finish {
            result: crate::ScalarValue::Integer(0),
        },
    };
    StepAst {
        id: format!("s{index}"),
        name: None,
        condition: None,
        primitive,
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

fn seeded_builder(max_slot: usize) -> SlotCompiler {
    let mut builder = SlotCompiler::new();
    builder.max_slot = Some(max_slot);
    builder.constants.push(vb_core::ConstValue::I64(-19));
    builder.nodes.push(vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::SetConst {
            value: vb_core::ConstIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        None,
    ));
    builder
}

fn assert_builder_eq(left: SlotCompiler, right: SlotCompiler) {
    assert_eq!(left.body_depth, right.body_depth);
    assert_eq!(left.nodes, right.nodes);
    assert_eq!(left.constants, right.constants);
    assert_eq!(left.expressions, right.expressions);
    assert_eq!(left.accessors, right.accessors);
    assert_eq!(left.max_slot, right.max_slot);
    assert_eq!(left.input_slots, right.input_slots);
    assert_eq!(left.display_names, right.display_names);
    assert_eq!(left.secret_requirements, right.secret_requirements);
    assert_eq!(left.symbols.finish(), right.symbols.finish());
}

struct ObservedBuilder {
    nodes: Vec<vb_core::CompiledNode>,
    constants: Vec<vb_core::ConstValue>,
    max_slot: Option<usize>,
}

fn compare(
    body: &[StepAst],
    start: u16,
    max_slot: usize,
) -> (
    Result<Option<(SlotIdx, ExprType)>, CompileErrors>,
    ObservedBuilder,
) {
    let outputs = HashMap::new();
    let steps = HashMap::new();
    let types = HashMap::new();
    let scope = CanonicalScope {
        outputs: &outputs,
        step_outputs: &steps,
        slot_types: &types,
        parent: None,
    };
    let mut actual = seeded_builder(max_slot);
    let mut expected = seeded_builder(max_slot);
    let result = emit_scoped_typed_body(
        body,
        StepIdx::new(start),
        StepIdx::new(77),
        scope,
        &mut actual,
        "collect",
    );
    let reference = reference_body(
        body,
        StepIdx::new(start),
        StepIdx::new(77),
        scope,
        &mut expected,
        "collect",
    );
    assert_eq!(result, reference);
    let observed = ObservedBuilder {
        nodes: actual.nodes.clone(),
        constants: actual.constants.clone(),
        max_slot: actual.max_slot,
    };
    assert_builder_eq(actual, expected);
    let mut flat_builder = seeded_builder(max_slot);
    let mut flat_reference = seeded_builder(max_slot);
    let flat = emit_scoped_flat_body(
        body,
        StepIdx::new(start),
        StepIdx::new(77),
        scope,
        &mut flat_builder,
        "collect",
    );
    let reference_flat = reference_body(
        body,
        StepIdx::new(start),
        StepIdx::new(77),
        scope,
        &mut flat_reference,
        "collect",
    )
    .map(|output| output.map(|(slot, _)| slot));
    assert_eq!(flat, reference_flat);
    assert_builder_eq(flat_builder, flat_reference);
    (result, observed)
}

proptest! {
    #[test]
    fn generic_body_full_builder_differential(kinds in prop::collection::vec(0u8..8, 0..8), start in 1u16..32) {
        let body: Vec<_> = kinds.into_iter().enumerate().map(|(i, kind)| step(i, kind)).collect();
        let (_result, _builder) = compare(&body, start, 0);
    }
}

#[path = "generic_body_tests/cases.rs"]
mod cases;

#[path = "generic_body_tests/depth.rs"]
mod depth;
