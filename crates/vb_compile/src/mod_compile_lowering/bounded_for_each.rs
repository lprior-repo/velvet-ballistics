use super::super::part_01::ForEachInputs;
use super::{CanonicalScope, resolve_slot_reference};
use crate::{CompileError, CompileErrors};
use vb_core::{SlotIdx, StepIdx};

pub(super) struct ForEachLowering<'a> {
    pub(super) input: &'a str,
    pub(super) variable: &'a str,
    pub(super) at_once: Option<u32>,
    pub(super) body: &'a [crate::StepAst],
    pub(super) next: Option<StepIdx>,
    pub(super) scope: CanonicalScope<'a>,
    pub(super) inputs: &'a ForEachInputs,
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
    body_len: usize,
    extra: usize,
    field: &'static str,
) -> Result<StepIdx, CompileErrors> {
    let offset = body_len.checked_add(extra).ok_or(CompileErrors(vec![
        CompileError::StepIndexOutOfRange { value: body_len },
    ]))?;
    let offset = u16::try_from(offset)
        .map_err(|_| CompileErrors(vec![CompileError::StepIndexOutOfRange { value: body_len }]))?;
    super::super::checked_step_offset(id, offset, "for_each", field)
        .map_err(|e| CompileErrors(vec![e]))
}

pub(super) fn body_edges(
    id: StepIdx,
    body_len: usize,
    next: Option<StepIdx>,
) -> Result<ForEachEdges, CompileErrors> {
    let body = super::super::checked_step_offset(id, 1, "for_each", "body")
        .map_err(|e| CompileErrors(vec![e]))?;
    let advance = body_edge(id, body_len, 1, "next")?;
    let done = match next {
        Some(n) => n,
        None => body_edge(id, body_len, 2, "done")?,
    };
    Ok(ForEachEdges {
        body,
        advance,
        done,
    })
}

fn validate_body(index: usize, config: &ForEachLowering<'_>) -> Result<(), CompileErrors> {
    if config.body.is_empty() {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "for_each.body",
            expected: "non-empty body",
        }]));
    }
    for step in config.body {
        if step.id == config.variable {
            return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
                name: config.variable.into(),
            }]));
        }
        if matches!(&step.primitive, crate::StepPrimitive::Set { output, .. } if output == config.variable)
        {
            return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
                name: config.variable.into(),
            }]));
        }
    }
    Ok(())
}

use super::super::SlotCompiler;
use vb_core::{CompiledNode, CompiledNodeKind};

fn emit_start(
    id: StepIdx,
    iterator: SlotIdx,
    item: SlotIdx,
    input: SlotIdx,
    limit: u32,
    edges: &ForEachEdges,
    builder: &mut SlotCompiler,
) {
    builder.push_node(CompiledNode {
        id,
        output: Some(iterator),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ForEachStart {
            input,
            item_slot: item,
            limit,
            body: edges.body,
            done: edges.done,
        },
    });
}

fn emit_next(iterator: SlotIdx, item: SlotIdx, edges: &ForEachEdges, builder: &mut SlotCompiler) {
    builder.push_node(CompiledNode {
        id: edges.advance,
        output: Some(item),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ForEachNext {
            iterator_slot: iterator,
            body: edges.body,
            done: edges.done,
        },
    });
}

fn validate_body_shape(index: usize, config: &ForEachLowering<'_>) -> Result<(), CompileErrors> {
    if config.body.is_empty() {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "for_each.body",
            expected: "non-empty body",
        }]));
    }
    Ok(())
}

fn emit_body(
    config: &ForEachLowering<'_>,
    edges: &ForEachEdges,
    item: SlotIdx,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
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

    let _body_output = super::super::part_14::emit_scoped_flat_body(
        config.body,
        edges.body,
        edges.advance,
        local_scope,
        builder,
        "for_each",
    )?;

    Ok(())
}

pub(super) fn lower_bounded_for_each(
    index: usize,
    id: StepIdx,
    config: ForEachLowering<'_>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    validate_body_shape(index, &config)?;

    let edges = body_edges(id, config.body.len(), config.next)?;

    let (input, limit) = resolve_input(index, &config)?;

    validate_parameters(index, &config)?;
    validate_body(index, &config)?;

    let iterator = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let item = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;

    emit_start(id, iterator, item, input, limit, &edges, builder);
    emit_body(&config, &edges, item, builder)?;
    emit_next(iterator, item, &edges, builder);

    Ok(())
}
