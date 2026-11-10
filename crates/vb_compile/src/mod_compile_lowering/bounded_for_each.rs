use super::super::part_01::ForEachInputs;
use super::super::{SlotCompiler, list_input};
use super::{CanonicalScope, resolve_slot_reference};
use crate::{CompileError, CompileErrors};
use vb_core::{
    CompiledNodeKind as K, ExprOp, ExprProgram, SlotIdx, StepIdx, WorkflowError, workflow::node,
};
use vb_expr::expr_typecheck::ExprType;

pub(super) struct ForEachLowering<'a> {
    pub(super) input: &'a str,
    pub(super) variable: &'a str,
    pub(super) at_once: Option<u32>,
    pub(super) body: &'a [crate::StepAst],
    pub(super) next: Option<StepIdx>,
    pub(super) scope: CanonicalScope<'a>,
    pub(super) inputs: &'a ForEachInputs,
}

fn offset(id: StepIdx, distance: usize, field: &'static str) -> Result<StepIdx, CompileErrors> {
    let distance = u16::try_from(distance)
        .map_err(|_| CompileErrors(vec![CompileError::StepIndexOutOfRange { value: distance }]))?;
    super::super::checked_step_offset(id, distance, "for_each", field)
        .map_err(|e| CompileErrors(vec![e]))
}

fn allocate(builder: &mut SlotCompiler) -> Result<SlotIdx, CompileErrors> {
    builder.allocate_slot().map_err(|e| CompileErrors(vec![e]))
}

fn resolve_input(
    index: usize,
    id: StepIdx,
    config: &ForEachLowering<'_>,
    builder: &mut SlotCompiler,
) -> Result<(SlotIdx, StepIdx, u32), CompileErrors> {
    let prelude = list_input::source_width(config.input);
    let init = offset(id, prelude, "init")?;
    let (input, ty) = if prelude == 0 {
        let input = resolve_slot_reference(
            config.input,
            config.scope.outputs,
            config.scope.step_outputs,
        )
        .map_err(|e| CompileErrors(vec![e]))?;
        (
            input,
            config
                .scope
                .slot_types
                .get(&input)
                .copied()
                .unwrap_or(ExprType::Unknown),
        )
    } else {
        let input = allocate(builder)?;
        let ty = super::lower_canonical_set_expression(
            id,
            input,
            config.input,
            Some(init),
            config.scope,
            builder,
        )?;
        (input, ty)
    };
    if !matches!(ty, ExprType::List | ExprType::Unknown) {
        return Err(CompileErrors(vec![CompileError::TypeMismatch {
            field: "for_each.input",
            expected: "list",
            found: "non-list",
        }]));
    }
    builder.record_slot(input);
    let bound = list_input::input_limit(input, builder)?;
    let declared = config
        .input
        .strip_prefix("$input.")
        .and_then(|name| config.inputs.resolve(input, name, index).ok());
    Ok((input, init, declared.map_or(bound, |max| max.min(bound))))
}

fn validate(index: usize, config: &ForEachLowering<'_>) -> Result<(), CompileErrors> {
    if config.at_once == Some(0) {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "for_each.at_once",
            expected: "a positive concurrency upper bound",
        }]));
    }
    crate::mod_compile_validation::validate_public_name("for_each.variable", config.variable)
        .map_err(|e| CompileErrors(vec![e]))?;
    if config.variable == "step" {
        return Err(CompileErrors(vec![CompileError::InvalidName {
            field: "for_each.variable",
            value: config.variable.into(),
        }]));
    }
    if config.scope.outputs.contains_key(config.variable)
        || config.scope.step_outputs.contains_key(config.variable)
    {
        return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
            name: config.variable.into(),
        }]));
    }
    for step in config.body {
        if step.id == config.variable
            || matches!(&step.primitive, crate::StepPrimitive::Set { output, .. } if output == config.variable)
        {
            return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
                name: config.variable.into(),
            }]));
        }
    }
    Ok(())
}

fn emit_body(
    config: &ForEachLowering<'_>,
    body: StepIdx,
    capture: StepIdx,
    item: SlotIdx,
    item_type: ExprType,
    builder: &mut SlotCompiler,
) -> Result<Option<SlotIdx>, CompileErrors> {
    let mut outputs = config.scope.outputs.clone();
    let mut types = config.scope.slot_types.clone();
    outputs.insert(config.variable.to_owned(), item);
    types.insert(item, item_type);
    super::super::part_14::emit_iteration_body(
        config.body,
        body,
        capture,
        CanonicalScope {
            outputs: &outputs,
            step_outputs: config.scope.step_outputs,
            slot_types: &types,
        },
        builder,
    )
}

pub(super) fn lower_bounded_for_each(
    index: usize,
    id: StepIdx,
    config: ForEachLowering<'_>,
    builder: &mut SlotCompiler,
) -> Result<SlotIdx, CompileErrors> {
    validate(index, &config)?;
    let (input, init, limit) = resolve_input(index, id, &config, builder)?;
    let taint = offset(init, 1, "source taint")?;
    let start = offset(init, 2, "start")?;
    let body = offset(init, 3, "body")?;
    let width = super::super::part_14::iteration_body_width(config.body)
        .map_err(|e| CompileErrors(vec![e]))?;
    let append = offset(body, width, "append")?;
    let advance = offset(append, 1, "next")?;
    let join = offset(advance, 1, "join")?;
    let item_type = list_input::item_type(input, builder, config.scope.slot_types);
    let accumulator = allocate(builder)?;
    let iterator = allocate(builder)?;
    let item = allocate(builder)?;
    let result = allocate(builder)?;
    builder.push_node(node(
        init,
        K::BuildList {
            items: Box::new([]),
        },
        Some(accumulator),
        Some(taint),
    ));
    let condition = builder.push_constant(vb_core::ConstValue::Bool(false))?;
    let expr = ExprProgram::try_from_ops(Box::from([
        ExprOp::LoadSlot(accumulator),
        ExprOp::LoadSlot(input),
        ExprOp::LoadConst(condition),
        ExprOp::AppendIf,
    ]))
    .map_err(|e| CompileErrors(vec![CompileError::Workflow(WorkflowError::Expression(e))]))?;
    let expr = builder.push_expression(expr)?;
    builder.push_node(node(
        taint,
        K::EvalExpr { expr },
        Some(accumulator),
        Some(start),
    ));
    builder.push_node(node(
        start,
        K::ForEachStart {
            input,
            item_slot: item,
            limit,
            body,
            done: join,
        },
        Some(iterator),
        None,
    ));
    let output = emit_body(&config, body, append, item, item_type, builder)?;
    let value = match output {
        Some(slot) => ExprOp::LoadSlot(slot),
        None => ExprOp::LoadConst(builder.push_constant(vb_core::ConstValue::Null)?),
    };
    let expr = ExprProgram::try_from_ops(Box::from([
        ExprOp::LoadSlot(accumulator),
        value,
        ExprOp::Append,
    ]))
    .map_err(|e| CompileErrors(vec![CompileError::Workflow(WorkflowError::Expression(e))]))?;
    let expr = builder
        .push_expression(expr)
        .map_err(|e| CompileErrors(vec![e]))?;
    builder.push_node(node(
        append,
        K::EvalExpr { expr },
        Some(accumulator),
        Some(advance),
    ));
    builder.push_node(node(
        advance,
        K::ForEachNext {
            iterator_slot: iterator,
            body,
            done: join,
        },
        Some(item),
        None,
    ));
    builder.push_node(node(
        join,
        K::ForEachJoin {
            output: accumulator,
        },
        Some(result),
        config.next,
    ));
    Ok(result)
}
