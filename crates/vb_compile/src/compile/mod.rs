#![forbid(unsafe_code)]
//! Core compilation logic for YAML workflow to IR lowering.

use crate::SourceMark;
use crate::{CompileError, CompileErrors, YamlCompiler};
use saphyr::Yaml;
use std::collections::{HashMap, HashSet};
use vb_core::{
    AccessorProgram, ActionContract, ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow,
    ConstIdx, ConstValue, ExprIdx, ExprProgram, Idempotency, ResourceContract, RetrySafety,
    SideEffect, SlotBranch, SlotIdx, StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
};

pub use crate::expression::{ParsedExpression, ExpressionHelper, ExpressionLiteral, BinaryOp, UnaryOp};
pub use crate::expression_bytecode::{compile_expr_to_bytecode, compile_expr_to_bytecode_with_accessors};

pub mod type_taint;

const WORKFLOW_VERSION: &str = "velvet-ballastics/v1";

pub fn compile_workflow(source: &[u8]) -> Result<CompiledWorkflow, CompileErrors> {
    YamlCompiler::default().compile(source)
}

pub fn compile_source(
    source: &vb_yaml::ast::WorkflowSource,
) -> Result<CompiledWorkflow, CompileErrors> {
    validate_canonical_compile_scope(source)?;
    let mut builder = SlotCompiler::new();
    let mut outputs: HashMap<&str, SlotIdx> = HashMap::new();
    let steps = source.steps();
    let last = steps
        .len()
        .checked_sub(1)
        .ok_or(CompileErrors(vec![CompileError::EmptySteps]))?;
    let mut step_names: Vec<Box<str>> = Vec::with_capacity(steps.len());
    for step in steps {
        step_names.push(Box::from(step.id.as_str()));
    }
    for (index, step) in steps.iter().enumerate() {
        let id = step_idx(index).map_err(|e| CompileErrors(vec![e]))?;
        let next = if index == last {
            None
        } else {
            Some(
                step_idx(index.checked_add(1).ok_or_else(|| {
                    CompileErrors(vec![CompileError::StepIndexOutOfRange { value: index }])
                })?)
                .map_err(|e| CompileErrors(vec![e]))?,
            )
        };
        match &step.primitive {
            vb_yaml::ast::StepPrimitive::Set { output, value } => {
                if outputs.contains_key(output.as_str()) {
                    return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
                        name: output.clone().into_boxed_str(),
                    }]));
                }
                let parsed = value.parse::<i64>().map_err(|_| {
                    CompileErrors(vec![CompileError::StepFieldShape {
                        step: index,
                        field: "set.value",
                        expected: "integer string",
                    }])
                })?;
                let const_idx = builder
                    .push_constant(ConstValue::I64(parsed))
                    .map_err(|e| CompileErrors(vec![e]))?;
                let slot = slot_idx_for_step(index).map_err(|e| CompileErrors(vec![e]))?;
                outputs.insert(output.as_str(), slot);
                builder.push_node(lower_set(id, slot, const_idx, next));
            }
            vb_yaml::ast::StepPrimitive::Finish { result } => {
                if index != last {
                    return Err(CompileErrors(vec![CompileError::StepFieldShape {
                        step: index,
                        field: "finish",
                        expected: "the last step",
                    }]));
                }
                let slot = canonical_finish_slot(result, &outputs)?;
                let node = lower_finish(id, slot, &mut builder);
                builder.push_node(node);
            }
            other => {
                return Err(CompileErrors(vec![
                    CompileError::UnsupportedStepPrimitive {
                        step: index,
                        primitive: canonical_primitive_name(other),
                    },
                ]));
            }
        }
    }
    let parts = WorkflowParts {
        name: Box::from(source.name()),
        digest: canonical_digest(source)?,
        slot_count: builder.slot_count().map_err(|e| CompileErrors(vec![e]))?,
        symbols_count: 0,
        nodes: builder.nodes.into_boxed_slice(),
        expressions: builder.expressions.into_boxed_slice(),
        accessors: builder.accessors.into_boxed_slice(),
        constants: builder.constants.into_boxed_slice(),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: step_names.into_boxed_slice(),
    };
    vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))?;
    CompiledWorkflow::try_from_parts(parts).map_err(|e| CompileErrors(vec![e.into()]))
}

fn validate_canonical_compile_scope(
    source: &vb_yaml::ast::WorkflowSource,
) -> Result<(), CompileErrors> {
    let mut errors = Vec::new();
    if !source.inputs().is_empty() {
        errors.push(CompileError::UnsupportedTopLevelDeclaration { field: "inputs" });
    }
    if !source.vars().is_empty() {
        errors.push(CompileError::UnsupportedTopLevelDeclaration { field: "vars" });
    }
    if !source.secrets().is_empty() {
        errors.push(CompileError::UnsupportedTopLevelDeclaration { field: "secrets" });
    }
    if !source.examples().is_empty() {
        errors.push(CompileError::UnsupportedTopLevelDeclaration { field: "examples" });
    }
    if source.result().is_some() {
        errors.push(CompileError::UnsupportedTopLevelResult);
    }
    let mut step_ids = HashSet::with_capacity(source.steps().len());
    for (index, step) in source.steps().iter().enumerate() {
        if !step_ids.insert(step.id.as_str()) {
            errors.push(CompileError::DuplicateStepId {
                id: Box::from(step.id.as_str()),
            });
        }
        if step.name.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("name"),
            });
        }
        if step.condition.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("if"),
            });
        }
        if step.with.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("with"),
            });
        }
        if step.retry.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("try_again"),
            });
        }
        if step.on_error.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("on_error"),
            });
        }
        if step.then.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("then"),
            });
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileErrors(errors))
    }
}

fn canonical_finish_slot(
    result: &vb_yaml::ast::ScalarValue,
    outputs: &HashMap<&str, SlotIdx>,
) -> Result<SlotIdx, CompileErrors> {
    match result {
        vb_yaml::ast::ScalarValue::String(name) => {
            outputs.get(name.as_str()).copied().ok_or_else(|| {
                CompileErrors(vec![CompileError::UnknownOutputName {
                    name: name.clone().into_boxed_str(),
                }])
            })
        }
        vb_yaml::ast::ScalarValue::Integer(value) => {
            let raw = u16::try_from(*value).map_err(|_| {
                CompileErrors(vec![CompileError::SlotIndexOutOfRange { value: *value }])
            })?;
            Ok(SlotIdx::new(raw))
        }
    }
}

fn canonical_primitive_name(primitive: &vb_yaml::ast::StepPrimitive) -> &'static str {
    match primitive {
        vb_yaml::ast::StepPrimitive::Set { .. } => "set",
        vb_yaml::ast::StepPrimitive::Save { .. } => "save",
        vb_yaml::ast::StepPrimitive::Do { .. } => "do",
        vb_yaml::ast::StepPrimitive::Choose { .. } => "choose",
        vb_yaml::ast::StepPrimitive::ForEach { .. } => "for_each",
        vb_yaml::ast::StepPrimitive::Together { .. } => "together",
        vb_yaml::ast::StepPrimitive::Collect { .. } => "collect",
        vb_yaml::ast::StepPrimitive::Reduce { .. } => "reduce",
        vb_yaml::ast::StepPrimitive::Repeat { .. } => "repeat",
        vb_yaml::ast::StepPrimitive::Wait { .. } => "wait",
        vb_yaml::ast::StepPrimitive::Ask { .. } => "ask",
        vb_yaml::ast::StepPrimitive::Finish { .. } => "finish",
    }
}

fn canonical_digest(source: &vb_yaml::ast::WorkflowSource) -> WorkflowDigest {
    let mut hasher = blake3::Hasher::new();
    hasher.update(source.version().as_bytes());
    hasher.update(source.name().as_bytes());
    match source.trigger() {
        vb_yaml::ast::TriggerAst::Manual => hasher.update(b"manual"),
        vb_yaml::ast::TriggerAst::Schedule { cron } => {
            hasher.update(b"schedule");
            hasher.update(cron.as_bytes())
        }
        vb_yaml::ast::TriggerAst::Event { event_type } => {
            hasher.update(b"event");
            hasher.update(event_type.as_bytes())
        }
        vb_yaml::ast::TriggerAst::Webhook => hasher.update(b"webhook"),
    };
    for step in source.steps() {
        hasher.update(step.id.as_bytes());
        digest_step_primitive(&mut hasher, &step.primitive);
    }
    WorkflowDigest::from_bytes(hasher.finalize().into())
}

fn digest_step_primitive(hasher: &mut blake3::Hasher, primitive: &vb_yaml::ast::StepPrimitive) {
    match primitive {
        vb_yaml::ast::StepPrimitive::Set { output, value } => {
            hasher.update(b"set");
            hasher.update(output.as_bytes());
            hasher.update(value.as_bytes());
        }
        vb_yaml::ast::StepPrimitive::Finish { result } => {
            hasher.update(b"finish");
            match result {
                vb_yaml::ast::ScalarValue::String(value) => hasher.update(value.as_bytes()),
                vb_yaml::ast::ScalarValue::Integer(value) => hasher.update(&value.to_le_bytes()),
            };
        }
        other => {
            hasher.update(canonical_primitive_name(other).as_bytes());
        }
    }
}

pub fn compile_workflow_with_contracts(
    source: &[u8],
    contracts: &[ActionContract],
) -> Result<CompiledWorkflow, CompileErrors> {
    let workflow = compile_workflow(source)?;
    let parts = workflow.to_parts();
    vb_validate::shared::validate_with_contracts(&parts, contracts)
        .map_err(|e| CompileErrors(vec![e.into()]))?;
    check_idempotency_gates(contracts)?;
    Ok(workflow)
}

pub fn build_slot_layout(parts: &WorkflowParts) -> u16 {
    parts.slot_count
}

pub fn build_accessor_table(parts: &WorkflowParts) -> &[AccessorProgram] {
    &parts.accessors
}

pub fn build_constant_pool(parts: &WorkflowParts) -> &[ConstValue] {
    &parts.constants
}

#[allow(clippy::too_many_arguments)]
pub fn lower_steps_to_ir(
    nodes: Vec<CompiledNode>,
    expressions: Vec<ExprProgram>,
    accessors: Vec<AccessorProgram>,
    constants: Vec<ConstValue>,
    slot_count: u16,
    symbols_count: u32,
    name: &str,
    digest: WorkflowDigest,
) -> Result<CompiledWorkflow, CompileErrors> {
    let parts = WorkflowParts {
        name: Box::from(name),
        digest,
        nodes: nodes.into_boxed_slice(),
        expressions: expressions.into_boxed_slice(),
        accessors: accessors.into_boxed_slice(),
        constants: constants.into_boxed_slice(),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))?;
    CompiledWorkflow::try_from_parts(parts).map_err(|e| CompileErrors(vec![e.into()]))
}

pub fn lower_set(
    id: StepIdx,
    output: SlotIdx,
    value: ConstIdx,
    next: Option<StepIdx>,
) -> CompiledNode {
    CompiledNode {
        id,
        output: Some(output),
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::SetConst { value },
    }
}

pub fn lower_do(
    id: StepIdx,
    action: vb_core::ActionId,
    input: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> CompiledNode {
    builder.record_slot(input);
    CompiledNode {
        id,
        output,
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::Do { action, input },
    }
}

pub fn lower_choose(
    id: StepIdx,
    branches: Vec<SlotBranch>,
    otherwise: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<CompiledNode, CompileError> {
    for branch in &branches {
        builder.record_slot(branch.condition);
    }
    let branches = branches.into_boxed_slice();
    validate_branch_route(&branches, otherwise)?;
    Ok(CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        },
    })
}

pub fn lower_for_each(
    id: StepIdx,
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: u32,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    builder.record_slot(input);
    builder.record_slot(item_slot);
    let iterator_slot = item_slot;
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::ForEachStart {
                input,
                item_slot,
                limit,
                body,
                done,
            },
        },
        CompiledNode {
            id: body,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::ForEachNext {
                iterator_slot,
                body,
                done,
            },
        },
    ])
}

pub fn lower_together(
    id: StepIdx,
    branches: Vec<StepIdx>,
    join: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    let branch_count = u16::try_from(branches.len()).map_err(|_| {
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "together",
            field: "branches",
            value: branches.len(),
            limit: usize::from(u16::MAX),
        }
    })?;
    let accumulator = alloc_accumulator_slot(builder)?;
    let mut nodes = vec![CompiledNode {
        id,
        output: Some(accumulator),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::TogetherStart {
            branches: branches.into_boxed_slice(),
            join,
        },
    }];
    nodes.push(CompiledNode {
        id: join,
        output: Some(accumulator),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        },
    });
    Ok(nodes)
}

fn alloc_accumulator_slot(builder: &mut SlotCompiler) -> Result<SlotIdx, CompileError> {
    let next = builder.slot_count()?;
    let slot = SlotIdx::new(next);
    builder.record_slot(slot);
    Ok(slot)
}

pub fn lower_collect(
    id: StepIdx,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    builder.record_slot(source);
    let collector_slot = source;
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::CollectStart {
                source,
                limit,
                page_size,
                body,
                done,
            },
        },
        CompiledNode {
            id: body,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::CollectPage {
                collector_slot,
                body,
                done,
            },
        },
        CompiledNode {
            id: done,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::CollectFinish { collector_slot },
        },
    ])
}

pub fn lower_reduce(
    id: StepIdx,
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    builder.record_slot(input);
    builder.record_slot(accumulator);
    let iterator_slot = accumulator;
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::ReduceStart {
                input,
                accumulator,
                initial,
                body,
                done,
            },
        },
        CompiledNode {
            id: body,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::ReduceNext {
                iterator_slot,
                accumulator,
                body,
                done,
            },
        },
        CompiledNode {
            id: done,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::ReduceFinish { accumulator },
        },
    ])
}

pub fn lower_repeat(
    id: StepIdx,
    max_attempts: u16,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    let attempt_slot = slot_idx_for_step(
        id.as_usize()
            .checked_add(1)
            .ok_or(CompileError::SlotIndexOutOfRange { value: i64::MAX })?,
    )?;
    builder.record_slot(attempt_slot);
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts,
                body,
                done,
            },
        },
        CompiledNode {
            id: body,
            output: Some(attempt_slot),
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::RepeatAttempt {
                attempt_slot,
                body,
                done,
            },
        },
        CompiledNode {
            id: done,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::RepeatFinish {
                result: attempt_slot,
            },
        },
    ])
}

#[non_exhaustive]
pub enum WaitKind {
    Until { deadline: SlotIdx },
    Event {
        event: SlotIdx,
        timeout: Option<SlotIdx>,
    },
}

pub fn lower_wait(id: StepIdx, kind: WaitKind, builder: &mut SlotCompiler) -> CompiledNode {
    let compiled_kind = match kind {
        WaitKind::Until { deadline } => {
            builder.record_slot(deadline);
            CompiledNodeKind::WaitUntil {
                deadline_slot: deadline,
            }
        }
        WaitKind::Event { event, timeout } => {
            builder.record_slot(event);
            if let Some(slot) = timeout {
                builder.record_slot(slot);
            }
            CompiledNodeKind::WaitEvent {
                event,
                timeout_slot: timeout,
            }
        }
    };
    CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: compiled_kind,
    }
}

pub fn lower_ask(
    id: StepIdx,
    prompt: SlotIdx,
    answer: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    builder.record_slot(prompt);
    builder.record_slot(answer);
    let resume = id
        .checked_add(1)
        .ok_or(CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "ask",
            field: "resume_step",
            value: id.as_usize(),
            limit: usize::from(u16::MAX),
        })?;
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Ask {
                prompt,
                timeout_slot,
            },
        },
        CompiledNode {
            id: resume,
            output: Some(answer),
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::AskResume { answer },
        },
    ])
}

pub fn lower_finish(id: StepIdx, result: SlotIdx, builder: &mut SlotCompiler) -> CompiledNode {
    builder.record_slot(result);
    CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::Finish { result },
    }
}

pub fn validate_ir(parts: WorkflowParts) -> Result<CompiledWorkflow, CompileErrors> {
    vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))?;
    CompiledWorkflow::try_from_parts(parts).map_err(|e| CompileErrors(vec![e.into()]))
}

pub fn compute_compiled_digest(source: &[u8]) -> WorkflowDigest {
    WorkflowDigest::from_bytes(blake3::hash(source).into())
}

pub fn emit_compiled_artifact(workflow: &CompiledWorkflow) -> Result<Box<[u8]>, CompileErrors> {
    let parts = workflow.to_parts();
    postcard::to_allocvec(&parts)
        .map(std::vec::Vec::into_boxed_slice)
        .map_err(|error| {
            CompileErrors(vec![CompileError::ExpressionLoweringUnsupported {
                feature: format!("postcard serialization failed: {error}").into_boxed_str(),
            }])
        })
}

pub fn compile_to_generated_rust(workflow: &CompiledWorkflow) -> Result<String, CompileErrors> {
    vb_codegen::emit_rust_workflow(workflow).map_err(|error| {
        CompileErrors(vec![CompileError::ExpressionLoweringUnsupported {
            feature: error.to_string().into_boxed_str(),
        }])
    })
}

pub fn check_idempotency_gates(contracts: &[ActionContract]) -> Result<(), CompileErrors> {
    let mut errors = Vec::new();
    let mut i = 0;
    while i < contracts.len() {
        let Some(contract) = contracts.get(i) else {
            break;
        };
        if contract.side_effect == SideEffect::Pure {
            i = match i.checked_add(1) {
                Some(next) => next,
                None => break,
            };
            continue;
        }
        if contract.retry_safety == RetrySafety::NotRetrySafe
            || contract.retry_safety == RetrySafety::Unknown
        {
            errors.push(CompileError::IdempotencyViolation {
                action: contract.id,
                side_effect: contract.side_effect,
                reason: Box::from(
                    "side-effecting action declares RetrySafety::NotRetrySafe or ::Unknown",
                ),
            });
            i = match i.checked_add(1) {
                Some(next) => next,
                None => break,
            };
            continue;
        }
        if contract.idempotency == Idempotency::AtLeastOnceExternal {
            errors.push(CompileError::IdempotencyViolation {
                action: contract.id,
                side_effect: contract.side_effect,
                reason: Box::from(
                    "side-effecting action declares Idempotency::AtLeastOnceExternal \
                     without guaranteed idempotent retry",
                ),
            });
        }
        i = match i.checked_add(1) {
            Some(next) => next,
            None => break,
        };
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileErrors(errors))
    }
}

#[derive(Debug, Default)]
pub struct SlotCompiler {
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
    accessors: Vec<AccessorProgram>,
    max_slot: Option<usize>,
}

impl SlotCompiler {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_constant(&mut self, value: ConstValue) -> Result<ConstIdx, CompileError> {
        let index = u16::try_from(self.constants.len()).map_err(|_| {
            CompileError::Workflow(WorkflowError::ConstOutOfBounds {
                constant: ConstIdx::new(u16::MAX),
            })
        })?;
        self.constants.push(value);
        Ok(ConstIdx::new(index))
    }

    pub fn push_expression(&mut self, program: ExprProgram) -> Result<ExprIdx, CompileError> {
        let index = u16::try_from(self.expressions.len()).map_err(|_| {
            CompileError::ExpressionLoweringUnsupported {
                feature: "expression table overflow".into(),
            }
        })?;
        self.expressions.push(program);
        Ok(ExprIdx::new(index))
    }

    pub fn push_accessor(
        &mut self,
        program: AccessorProgram,
    ) -> Result<vb_core::AccessorIdx, CompileError> {
        let index = u16::try_from(self.accessors.len()).map_err(|_| {
            CompileError::ExpressionLoweringUnsupported {
                feature: "accessor table overflow".into(),
            }
        })?;
        self.accessors.push(program);
        Ok(vb_core::AccessorIdx::new(index))
    }

    pub fn record_slot(&mut self, slot: SlotIdx) {
        let value = slot.as_usize();
        self.max_slot = Some(match self.max_slot {
            Some(current) => current.max(value),
            None => value,
        });
    }

    pub fn push_node(&mut self, node: CompiledNode) {
        self.nodes.push(node);
    }

    pub fn slot_count(&self) -> Result<u16, CompileError> {
        match self.max_slot {
            Some(value) => {
                let count = value
                    .checked_add(1)
                    .ok_or(CompileError::SlotIndexOutOfRange { value: i64::MAX })?;
                u16::try_from(count).map_err(|_| CompileError::SlotIndexOutOfRange {
                    value: i64::from(u16::MAX),
                })
            }
            None => Ok(0),
        }
    }

    pub fn build_parts(
        self,
        name: &str,
        digest: WorkflowDigest,
    ) -> Result<WorkflowParts, CompileError> {
        Ok(WorkflowParts {
            name: Box::from(name),
            digest,
            slot_count: self.slot_count()?,
            symbols_count: 0,
            nodes: self.nodes.into_boxed_slice(),
            expressions: self.expressions.into_boxed_slice(),
            accessors: self.accessors.into_boxed_slice(),
            constants: self.constants.into_boxed_slice(),
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
    }
}

fn validate_branch_route(
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
) -> Result<(), CompileError> {
    if branches.is_empty() && otherwise.is_none() {
        Err(CompileError::Workflow(WorkflowError::EmptyBranchTable))
    } else {
        Ok(())
    }
}

fn step_idx(value: usize) -> Result<StepIdx, CompileError> {
    let value = u16::try_from(value).map_err(|_| CompileError::StepIndexOutOfRange { value })?;
    Ok(StepIdx::new(value))
}

fn slot_idx_for_step(value: usize) -> Result<SlotIdx, CompileError> {
    let value = u16::try_from(value).map_err(|_| CompileError::StepIndexOutOfRange { value })?;
    Ok(SlotIdx::new(value))
}
