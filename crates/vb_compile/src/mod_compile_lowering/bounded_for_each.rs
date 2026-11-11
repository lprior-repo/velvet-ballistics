#![forbid(unsafe_code)]

use super::super::part_01::ForEachInputs;
use super::super::part_14::BodyCtx;
use super::{CanonicalScope, resolve_slot_reference, validate_output_alias};
use crate::{CompileError, CompileErrors};
use vb_core::{SlotIdx, StepIdx};

pub(in crate::mod_compile_lowering) struct ForEachLowering<'a> {
    pub(in crate::mod_compile_lowering) input: &'a str,
    pub(in crate::mod_compile_lowering) variable: &'a str,
    pub(in crate::mod_compile_lowering) at_once: Option<u32>,
    pub(in crate::mod_compile_lowering) body: &'a [crate::StepAst],
    pub(in crate::mod_compile_lowering) next: Option<StepIdx>,
    pub(in crate::mod_compile_lowering) scope: CanonicalScope<'a>,
    pub(in crate::mod_compile_lowering) inputs: &'a ForEachInputs,
    pub(in crate::mod_compile_lowering) nested: BodyCtx<'a>,
}

fn domain_error(index: usize) -> CompileErrors {
    CompileErrors(vec![CompileError::StepFieldShape {
        step: index,
        field: "for_each.input",
        expected: "direct declared list<any> input with finite schema max",
    }])
}

fn resolve_input(
    index: usize,
    config: &ForEachLowering<'_>,
) -> Result<(SlotIdx, u32), CompileErrors> {
    let name = config
        .input
        .strip_prefix("$input.")
        .ok_or_else(|| domain_error(index))?;
    let slot = resolve_slot_reference(
        config.input,
        config.scope.outputs,
        config.scope.step_outputs,
    )
    .map_err(|e| CompileErrors(vec![e]))?;
    let max = config
        .inputs
        .resolve(slot, name, index)
        .map_err(|e| CompileErrors(vec![e]))?;
    Ok((slot, max))
}

fn validate_concurrency(index: usize, at_once: Option<u32>) -> Result<(), CompileErrors> {
    match at_once {
        None | Some(1) => Ok(()),
        Some(_) => Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "for_each.at_once",
            expected: "omitted or 1 until concurrent for_each lowering is implemented",
        }])),
    }
}

fn validate_variable_name(variable: &str) -> Result<(), CompileErrors> {
    crate::mod_compile_validation::validate_public_name("for_each.variable", variable)
        .map_err(|e| CompileErrors(vec![e]))?;
    validate_output_alias(variable).map_err(|e| CompileErrors(vec![e]))?;
    if variable == "step" {
        return Err(CompileErrors(vec![CompileError::InvalidName {
            field: "for_each.variable",
            value: variable.into(),
        }]));
    }
    Ok(())
}

fn validate_variable_capture(
    variable: &str,
    config: &ForEachLowering<'_>,
) -> Result<(), CompileErrors> {
    if config.scope.outputs.contains_key(variable)
        || config.scope.step_outputs.contains_key(variable)
    {
        return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
            name: variable.into(),
        }]));
    }
    Ok(())
}

pub(super) fn validate_parameters(
    index: usize,
    config: &ForEachLowering<'_>,
) -> Result<(), CompileErrors> {
    validate_concurrency(index, config.at_once)?;
    validate_variable_name(config.variable)?;
    validate_variable_capture(config.variable, config)?;
    Ok(())
}

pub(super) struct ForEachEdges {
    pub body: StepIdx,
    pub advance: StepIdx,
    pub done: StepIdx,
}

fn body_edge(
    id: StepIdx,
    body_width: usize,
    extra: usize,
    field: &'static str,
) -> Result<StepIdx, CompileErrors> {
    let offset = body_width.checked_add(extra).ok_or(CompileErrors(vec![
        CompileError::StepIndexOutOfRange { value: body_width },
    ]))?;
    let offset = u16::try_from(offset).map_err(|_| {
        CompileErrors(vec![CompileError::StepIndexOutOfRange {
            value: body_width,
        }])
    })?;
    super::super::checked_step_offset(id, offset, "for_each", field)
        .map_err(|e| CompileErrors(vec![e]))
}

pub(super) fn body_edges(
    id: StepIdx,
    body_width: usize,
    next: Option<StepIdx>,
) -> Result<ForEachEdges, CompileErrors> {
    let body = super::super::checked_step_offset(id, 1, "for_each", "body")
        .map_err(|e| CompileErrors(vec![e]))?;
    let advance = body_edge(id, body_width, 1, "next")?;
    let done = match next {
        Some(n) => n,
        None => body_edge(id, body_width, 2, "done")?,
    };
    Ok(ForEachEdges {
        body,
        advance,
        done,
    })
}

fn empty_error(index: usize) -> CompileErrors {
    CompileErrors(vec![CompileError::StepFieldShape {
        step: index,
        field: "for_each.body",
        expected: "non-empty body",
    }])
}

fn validate_body(index: usize, config: &ForEachLowering<'_>) -> Result<(), CompileErrors> {
    if config.body.is_empty() {
        return Err(empty_error(index));
    }
    super::for_each_capture::validate_for_each_capture(config.variable, config.body)
}

use super::super::SlotCompiler;
use vb_core::CompiledNodeKind;
use vb_core::workflow::node;

fn emit_start(
    id: StepIdx,
    iterator: SlotIdx,
    item: SlotIdx,
    input: SlotIdx,
    limit: u32,
    edges: &ForEachEdges,
    builder: &mut SlotCompiler,
) {
    builder.push_node(node(
        id,
        CompiledNodeKind::ForEachStart {
            input,
            item_slot: item,
            limit: limit.into(),
            body: edges.body,
            done: edges.done,
        },
        Some(iterator),
        None,
    ));
}

fn emit_next(iterator: SlotIdx, item: SlotIdx, edges: &ForEachEdges, builder: &mut SlotCompiler) {
    builder.push_node(node(
        edges.advance,
        CompiledNodeKind::ForEachNext {
            iterator_slot: iterator,
            body: edges.body,
            done: edges.done,
        },
        Some(item),
        None,
    ));
}

fn emit_body(
    config: &ForEachLowering<'_>,
    edges: &ForEachEdges,
    item: SlotIdx,
    builder: &mut SlotCompiler,
) -> Result<Option<SlotIdx>, CompileErrors> {
    let mut local_outputs = config.scope.outputs.clone();
    let local_step_outputs = config.scope.step_outputs.clone();
    let mut local_types = config.scope.slot_types.clone();

    local_outputs.insert(config.variable.to_owned(), item);
    local_types.insert(item, vb_expr::expr_typecheck::ExprType::Unknown);

    let local_scope = CanonicalScope {
        outputs: &local_outputs,
        step_outputs: &local_step_outputs,
        slot_types: &local_types,
    };

    let (body_output, _) = super::super::part_14::emit_scoped_flat_body(
        config.body,
        edges.body,
        edges.advance,
        local_scope,
        config.nested,
        builder,
        "for_each",
        None,
    )?;

    Ok(body_output)
}

pub(in crate::mod_compile_lowering) fn lower_bounded_for_each(
    index: usize,
    id: StepIdx,
    config: ForEachLowering<'_>,
    builder: &mut SlotCompiler,
) -> Result<Option<SlotIdx>, CompileErrors> {
    validate_body(index, &config)?;
    validate_parameters(index, &config)?;

    let width = super::super::body_width(config.body, 0).map_err(|e| CompileErrors(vec![e]))?;
    let edges = body_edges(id, width, config.next)?;

    let (input, limit) = resolve_input(index, &config)?;

    let iterator = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let item = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;

    emit_start(id, iterator, item, input, limit, &edges, builder);
    let body_output = emit_body(&config, &edges, item, builder)?;
    emit_next(iterator, item, &edges, builder);

    Ok(body_output)
}
