//! Empty-body diagnostics must precede lowering work and leave caller state intact.

use super::{SlotCompiler, part_01::ForEachInputs, part_02::lower_canonical_step};
use crate::{AuthorEntry, AuthorValue, CompileError, CompileErrors, StepAst, StepPrimitive};
use std::collections::HashMap;
use vb_core::workflow::{PathSegment, node};
use vb_core::{
    AccessorProgram, CompiledInputConstraint, CompiledInputSlot, CompiledNode, CompiledNodeKind,
    ConstValue, ExprIdx, ExprOp, ExprProgram, InputSlotKind, SlotIdx, StepIdx,
};
use vb_expr::expr_typecheck::ExprType;

const DIAGNOSTIC_STEP: usize = 7;

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
    accessors: Vec<AccessorProgram>,
    max_slot: Option<usize>,
    input_slots: Vec<CompiledInputSlot>,
    outputs: HashMap<String, SlotIdx>,
    step_outputs: HashMap<String, SlotIdx>,
    step_names: Vec<Box<str>>,
    slot_types: HashMap<SlotIdx, ExprType>,
}

struct Fixture {
    builder: SlotCompiler,
    outputs: HashMap<String, SlotIdx>,
    step_outputs: HashMap<String, SlotIdx>,
    step_names: Vec<Box<str>>,
    slot_types: HashMap<SlotIdx, ExprType>,
    inputs: ForEachInputs,
}

impl Fixture {
    fn new() -> Result<Self, CompileError> {
        let mut builder = SlotCompiler::new();
        builder.reserve_slots(4)?;
        let constant = builder.push_constant(ConstValue::I64(17))?;
        builder.push_expression(ExprProgram {
            ops: vec![ExprOp::LoadConst(constant)].into_boxed_slice(),
            max_stack: 1,
        })?;
        builder.push_accessor(AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Index(0)].into_boxed_slice(),
        })?;
        builder.push_node(node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(1)),
        ));
        builder.record_input_slot(
            SlotIdx::new(0),
            InputSlotKind::List,
            CompiledInputConstraint {
                max: Some(3),
                element: Some(InputSlotKind::Any),
                ..CompiledInputConstraint::required()
            },
        );
        let schema = AuthorValue::Mapping(vec![
            AuthorEntry {
                key: "is".into(),
                value: AuthorValue::Text("list".into()),
            },
            AuthorEntry {
                key: "of".into(),
                value: AuthorValue::Text("any".into()),
            },
            AuthorEntry {
                key: "max".into(),
                value: AuthorValue::I64(3),
            },
        ]);
        let mut inputs = ForEachInputs::new();
        inputs.record(SlotIdx::new(0), "items", &schema);
        Ok(Self {
            builder,
            outputs: HashMap::from([
                ("$input.items".into(), SlotIdx::new(0)),
                ("answer".into(), SlotIdx::new(1)),
            ]),
            step_outputs: HashMap::from([("seed".into(), SlotIdx::new(1))]),
            step_names: vec![Box::from("seed")],
            slot_types: HashMap::from([
                (SlotIdx::new(0), ExprType::List),
                (SlotIdx::new(1), ExprType::I64),
            ]),
            inputs,
        })
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            nodes: self.builder.nodes.clone(),
            constants: self.builder.constants.clone(),
            expressions: self.builder.expressions.clone(),
            accessors: self.builder.accessors.clone(),
            max_slot: self.builder.max_slot,
            input_slots: self.builder.input_slots.clone(),
            outputs: self.outputs.clone(),
            step_outputs: self.step_outputs.clone(),
            step_names: self.step_names.clone(),
            slot_types: self.slot_types.clone(),
        }
    }

    fn lower(&mut self, step: &StepAst, id: StepIdx) -> Result<(), CompileErrors> {
        lower_canonical_step(
            step,
            DIAGNOSTIC_STEP,
            8,
            id,
            Some(StepIdx::new(4)),
            &mut self.outputs,
            &mut self.step_outputs,
            &mut self.step_names,
            &mut self.slot_types,
            &self.inputs,
            &mut self.builder,
        )
    }
}

fn step(id: &str, primitive: StepPrimitive) -> StepAst {
    StepAst {
        id: id.into(),
        name: None,
        condition: None,
        primitive,
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

fn loop_step(input: &str, variable: &str, at_once: Option<u32>, empty: bool) -> StepAst {
    let body = if empty {
        Vec::new()
    } else {
        vec![step(
            "copy",
            StepPrimitive::Set {
                output: "copied".into(),
                value: "$entry".into(),
            },
        )]
    };
    step(
        "bounded_loop",
        StepPrimitive::ForEach {
            input: input.into(),
            variable: variable.into(),
            at_once,
            body,
        },
    )
}

fn empty_body_error() -> CompileError {
    CompileError::StepFieldShape {
        step: DIAGNOSTIC_STEP,
        field: "for_each.body",
        expected: "non-empty body",
    }
}

fn assert_precedence_pair(
    input: &str,
    variable: &str,
    at_once: Option<u32>,
    id: StepIdx,
    nonempty_error: CompileError,
) -> Result<(), CompileError> {
    for empty in [true, false] {
        let mut fixture = Fixture::new()?;
        let before = fixture.snapshot();
        let expected = if empty {
            empty_body_error()
        } else {
            nonempty_error.clone()
        };
        let result = fixture.lower(&loop_step(input, variable, at_once, empty), id);
        assert_eq!(result, Err(CompileErrors(vec![expected])));
        assert_eq!(
            fixture.snapshot(),
            before,
            "empty={empty}, input={input}, variable={variable}"
        );
    }
    Ok(())
}

#[test]
fn empty_body_preserves_every_seeded_builder_and_caller_field() -> Result<(), CompileError> {
    let mut fixture = Fixture::new()?;
    let before = fixture.snapshot();
    let result = fixture.lower(
        &loop_step("$input.items", "entry", Some(1), true),
        StepIdx::new(1),
    );
    assert_eq!(result, Err(CompileErrors(vec![empty_body_error()])));
    assert_eq!(fixture.snapshot(), before);
    Ok(())
}

#[test]
fn empty_body_precedes_edge_overflow_even_with_other_invalid_fields() -> Result<(), CompileError> {
    assert_precedence_pair(
        "0",
        "step",
        Some(0),
        StepIdx::new(u16::MAX),
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "for_each",
            field: "body",
            value: usize::from(u16::MAX),
            limit: usize::from(u16::MAX),
        },
    )
}

#[test]
fn empty_body_precedes_input_and_each_parameter_check() -> Result<(), CompileError> {
    assert_precedence_pair(
        "0",
        "entry",
        Some(1),
        StepIdx::new(1),
        CompileError::StepFieldShape {
            step: DIAGNOSTIC_STEP,
            field: "for_each.input",
            expected: "direct declared list<any> input with finite schema max",
        },
    )?;
    for at_once in [0, 2] {
        assert_precedence_pair(
            "$input.items",
            "entry",
            Some(at_once),
            StepIdx::new(1),
            CompileError::StepFieldShape {
                step: DIAGNOSTIC_STEP,
                field: "for_each.at_once",
                expected: "omitted or 1 until concurrent for_each lowering is implemented",
            },
        )?;
    }
    assert_precedence_pair(
        "$input.items",
        "step",
        Some(1),
        StepIdx::new(1),
        CompileError::InvalidName {
            field: "for_each.variable",
            value: "step".into(),
        },
    )?;
    for variable in ["answer", "seed"] {
        assert_precedence_pair(
            "$input.items",
            variable,
            Some(1),
            StepIdx::new(1),
            CompileError::DuplicateOutputName {
                name: variable.into(),
            },
        )?;
    }
    Ok(())
}

#[test]
fn nonempty_control_emits_exact_loop_and_preserves_outer_bindings()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = Fixture::new()?;
    let mut expected = fixture.snapshot();
    expected.nodes.extend([
        node(
            StepIdx::new(1),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(5),
                limit: 3,
                body: StepIdx::new(2),
                done: StepIdx::new(4),
            },
            Some(SlotIdx::new(4)),
            None,
        ),
        node(
            StepIdx::new(2),
            CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(1),
            },
            Some(SlotIdx::new(6)),
            Some(StepIdx::new(3)),
        ),
        node(
            StepIdx::new(3),
            CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(4),
                body: StepIdx::new(2),
                done: StepIdx::new(4),
            },
            Some(SlotIdx::new(5)),
            None,
        ),
    ]);
    expected.expressions.push(ExprProgram {
        ops: vec![ExprOp::LoadSlot(SlotIdx::new(5))].into_boxed_slice(),
        max_stack: 1,
    });
    expected.max_slot = Some(6);
    expected.step_names.extend([
        Box::from("bounded_loop"),
        Box::from("bounded_loop"),
        Box::from("bounded_loop"),
    ]);
    fixture.lower(
        &loop_step("$input.items", "entry", Some(1), false),
        StepIdx::new(1),
    )?;
    assert_eq!(fixture.snapshot(), expected);
    Ok(())
}
