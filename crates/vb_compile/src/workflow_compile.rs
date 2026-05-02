//! Workflow-level compilation.
//!
//! This module handles the compilation of YAML documents into workflow IR,
//! including workflow-level validation, WorkflowBuilder, and build_workflow_parts.

#![forbid(unsafe_code)]

use saphyr::Yaml;
use std::collections::HashSet;
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ResourceContract,
    SlotIdx, StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
};

use super::errors::{
    check_idempotency_gates, validate_public_name, CompileError, CompileErrors,
};
use super::ir_emitter::{lower_ask, lower_choose, lower_collect, lower_do, lower_finish, lower_for_each,
    lower_reduce, lower_repeat, lower_set, lower_together, lower_wait, slot_idx_for_step,
    step_idx, SlotCompiler};

/// Workflow constants and defaults.
pub const WORKFLOW_VERSION: &str = "velvet-ballastics/v1";

/// Builds a slot layout from workflow parts.
///
/// Returns the number of slots needed by the compiled workflow frame.
/// The slot layout is derived from the maximum slot index referenced
/// across all compiled nodes.
pub fn build_slot_layout(parts: &WorkflowParts) -> u16 {
    parts.slot_count
}

/// Builds the accessor table from workflow parts.
///
/// Returns a reference to the accessor programs table for slot-rooted
/// path traversal.
pub fn build_accessor_table(parts: &WorkflowParts) -> &[vb_core::AccessorProgram] {
    &parts.accessors
}

/// Builds the constant pool from workflow parts.
///
/// Returns a reference to the constant pool containing all literal values
/// referenced by compiled nodes and expression programs.
pub fn build_constant_pool(parts: &WorkflowParts) -> &[ConstValue] {
    &parts.constants
}

/// Top-level compilation entry point producing a validated compiled workflow.
///
/// Wraps [`YamlCompiler::compile`] with the default limits for ergonomic
/// programmatic use by downstream crates.
pub fn compile_workflow(source: &[u8]) -> Result<CompiledWorkflow, CompileErrors> {
    super::YamlCompiler::default().compile(source)
}

/// Compiles YAML source and then verifies action contracts against the
/// idempotency gate AND gate 12 (action contract completeness).
///
/// Performs the full compilation pipeline from [`compile_workflow`], then runs
/// gate 12 to verify that every Do node has a matching contract and every
/// contract has a matching Do node, and finally runs [`check_idempotency_gates`]
/// on the supplied action contracts. Returns the compiled workflow only when
/// all three checks pass. This is the recommended entry point for runtime
/// integrations that register action contracts before workflow deployment.
pub fn compile_workflow_with_contracts(
    source: &[u8],
    contracts: &[vb_core::ActionContract],
) -> Result<CompiledWorkflow, CompileErrors> {
    let workflow = compile_workflow(source)?;
    let parts = workflow.to_parts();
    vb_validate::shared::validate_with_contracts(&parts, contracts)
        .map_err(|e| CompileErrors(vec![e.into()]))?;
    check_idempotency_gates(contracts)?;
    Ok(workflow)
}

/// Builds workflow parts from parsed YAML and text.
pub fn build_workflow_parts(
    text: &str,
    doc: &Yaml<'_>,
) -> Result<WorkflowParts, CompileError> {
    validate_workflow_document_shape(doc)?;

    let name = required_string_field(doc, "name")?;
    let steps = required_sequence_field(doc, "steps")?;
    let digest = WorkflowDigest::from_bytes(blake3::hash(text.as_bytes()).into());
    let mut builder = WorkflowBuilder::new();
    let last_step = steps
        .len()
        .checked_sub(1)
        .ok_or(CompileError::EmptySteps)?;
    let source_ir_starts = build_source_ir_starts(steps)?;

    for (index, step) in steps.iter().enumerate() {
        let id = source_ir_start(&source_ir_starts, index)?;
        let next = optional_source_ir_start(&source_ir_starts, index)?;
        let nodes = compile_step(
            step,
            index,
            last_step,
            id,
            next,
            &source_ir_starts,
            &mut builder,
        )?;
        builder.nodes.extend(nodes);
    }
    Ok(WorkflowParts {
        name: Box::<str>::from(name),
        digest,
        slot_count: builder.slot_count()?,
        nodes: builder.nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: builder.constants.into_boxed_slice(),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
    })
}

fn build_source_ir_starts(steps: &saphyr::Sequence<'_>) -> Result<Vec<StepIdx>, CompileError> {
    let mut starts = Vec::with_capacity(steps.len());
    let mut cursor = 0usize;
    for (index, step) in steps.iter().enumerate() {
        starts.push(step_idx(cursor)?);
        cursor = cursor
            .checked_add(compiled_step_width(step, index)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: cursor })?;
    }
    Ok(starts)
}

fn compiled_step_width(step: &Yaml<'_>, index: usize) -> Result<usize, CompileError> {
    let StepSpec { primitive, body } = step_spec(step, index)?;
    match primitive {
        StepPrimitive::Ask | StepPrimitive::ForEach | StepPrimitive::Together => Ok(2),
        StepPrimitive::Collect | StepPrimitive::Reduce | StepPrimitive::Repeat => Ok(3),
        StepPrimitive::Finish => {
            let result = required_step_field(body, index, "result")?;
            if finish_result_slot(result, index)?.is_some() {
                Ok(1)
            } else {
                Ok(2)
            }
        }
        _ => Ok(1),
    }
}

fn source_ir_start(starts: &[StepIdx], index: usize) -> Result<StepIdx, CompileError> {
    starts
        .get(index)
        .copied()
        .ok_or(CompileError::StepIndexOutOfRange { value: index })
}

fn optional_source_ir_start(
    starts: &[StepIdx],
    index: usize,
) -> Result<Option<StepIdx>, CompileError> {
    let next = index
        .checked_add(1)
        .ok_or(CompileError::StepIndexOutOfRange { value: index })?;
    Ok(starts.get(next).copied())
}

pub(crate) fn validate_workflow_document_shape(doc: &Yaml<'_>) -> Result<(), CompileError> {
    validate_top_level_keys(doc)?;
    validate_workflow_version(doc)?;
    validate_workflow_trigger(doc)?;
    validate_optional_top_level_shapes(doc)?;
    validate_phase_zero_result(doc)?;
    let name = required_string_field(doc, "name")?;
    validate_public_name("name", name)?;
    let steps = required_sequence_field(doc, "steps")?;
    if steps.is_empty() {
        return Err(CompileError::EmptySteps);
    }
    validate_step_ids(steps)?;
    validate_phase_zero_step_shapes(steps)
}

fn validate_phase_zero_step_shapes(steps: &saphyr::Sequence<'_>) -> Result<(), CompileError> {
    let last_step = steps
        .len()
        .checked_sub(1)
        .ok_or(CompileError::EmptySteps)?;
    for (index, step) in steps.iter().enumerate() {
        validate_phase_zero_step_shape(step, index, last_step)?;
    }
    Ok(())
}

fn validate_phase_zero_step_shape(
    step: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    let StepSpec { primitive, body } = step_spec(step, index)?;
    match primitive {
        StepPrimitive::Run | StepPrimitive::Do => {
            validate_run_shape(body, index, last_step, primitive.as_str())
        }
        StepPrimitive::Set | StepPrimitive::Save => {
            validate_save_shape(body, index, last_step, primitive.as_str())
        }
        StepPrimitive::Choose => validate_choose_shape(body, index, last_step),
        StepPrimitive::ForEach => validate_for_each_shape(body, index, last_step),
        StepPrimitive::Together => validate_together_shape(body, index, last_step),
        StepPrimitive::Collect => validate_collect_shape(body, index, last_step),
        StepPrimitive::Reduce => validate_reduce_shape(body, index, last_step),
        StepPrimitive::Repeat => validate_repeat_shape(body, index, last_step),
        StepPrimitive::Wait => validate_wait_shape(body, index, last_step),
        StepPrimitive::Ask => validate_ask_shape(body, index, last_step),
        StepPrimitive::Finish => validate_finish_shape(body, index, last_step),
    }
}

fn validate_run_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    primitive: &'static str,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    if !body.is_mapping() {
        return Err(CompileError::UnsupportedStepPrimitive {
            step: index,
            primitive,
        });
    }
    reject_unknown_primitive_fields(body, index, primitive, &["action", "input"])?;
    required_action(body, index, primitive)?;
    required_slot(body, index, "input")?;
    Ok(())
}

fn validate_wait_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "wait", &["until", "event", "timeout"])?;
    let until = optional_slot_field(body, index, "until")?;
    let event = optional_slot_field(body, index, "event")?;
    let timeout = optional_slot_field(body, index, "timeout")?;
    match (until, event, timeout) {
        (Some(_), None, None) | (None, Some(_), _) => Ok(()),
        _ => Err(CompileError::StepFieldShape {
            step: index,
            field: "wait",
            expected: "until without timeout or event with optional timeout",
        }),
    }
}

fn validate_ask_shape(body: &Yaml<'_>, index: usize, last_step: usize) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "ask", &["prompt", "answer", "timeout"])?;
    required_slot(body, index, "prompt")?;
    required_slot(body, index, "answer")?;
    optional_slot_field(body, index, "timeout")?;
    Ok(())
}

fn validate_save_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    primitive: &'static str,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    if body.is_mapping() {
        Ok(())
    } else {
        Err(CompileError::StepFieldShape {
            step: index,
            field: primitive,
            expected: "an object",
        })
    }
}

fn validate_choose_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(
        body,
        index,
        "choose",
        &["condition", "on_true", "on_false"],
    )?;
    required_step_field(body, index, "condition")?;
    required_branch_target(body, index, "on_true")?;
    required_branch_target(body, index, "on_false")?;
    Ok(())
}

fn validate_for_each_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unsupported_for_each_fields(body, index)?;
    reject_unknown_primitive_fields(body, index, "for_each", &["input", "item", "limit"])?;
    required_slot(body, index, "input")?;
    required_slot(body, index, "item")?;
    required_u32_field(body, index, "for_each", "limit")?;
    Ok(())
}

fn reject_unsupported_for_each_fields(body: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    let Some(mapping) = body.as_mapping() else {
        return Ok(());
    };
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            continue;
        };
        if field == "at_once" {
            return Err(CompileError::UnsupportedStepPrimitive {
                step,
                primitive: "for_each",
            });
        }
    }
    Ok(())
}

fn validate_together_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "together", &["branches"])?;
    required_branch_targets(body, index, "branches")?;
    Ok(())
}

fn validate_collect_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(
        body,
        index,
        "collect",
        &["source", "limit", "page_size"],
    )?;
    required_slot(body, index, "source")?;
    required_u32_field(body, index, "collect", "limit")?;
    required_u32_field(body, index, "collect", "page_size")?;
    Ok(())
}

fn validate_reduce_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "reduce", &["input", "accumulator", "initial"])?;
    required_slot(body, index, "input")?;
    required_slot(body, index, "accumulator")?;
    let initial = required_step_field(body, index, "initial")?;
    slot_value(initial, index)?;
    Ok(())
}

fn validate_repeat_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "repeat", &["max_attempts"])?;
    required_u16_field(body, index, "repeat", "max_attempts")?;
    Ok(())
}

fn validate_finish_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    if index != last_step {
        return Err(CompileError::StepFieldShape {
            step: index,
            field: "finish",
            expected: "the last step",
        });
    }
    reject_unknown_primitive_fields(body, index, "finish", &["result"])?;
    required_step_field(body, index, "result")?;
    Ok(())
}

fn validate_phase_zero_result(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("result") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "result",
        expected: "a mapping",
    })?;
    if mapping.is_empty() {
        Ok(())
    } else {
        Err(CompileError::UnsupportedTopLevelResult)
    }
}

fn validate_optional_top_level_shapes(doc: &Yaml<'_>) -> Result<(), CompileError> {
    optional_inputs_mapping(doc)?;
    optional_vars_mapping(doc)?;
    optional_secret_mapping(doc)?;
    optional_examples_sequence(doc)
}

fn optional_inputs_mapping(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("inputs") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "inputs",
        expected: "a mapping",
    })?;
    for (key, _) in mapping {
        let Some(name) = key.as_str() else {
            return Err(super::errors::non_string_key_error());
        };
        validate_public_name("inputs", name)?;
    }
    Ok(())
}

fn optional_vars_mapping(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("vars") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "vars",
        expected: "a mapping",
    })?;
    for (key, value) in mapping {
        let Some(name) = key.as_str() else {
            return Err(super::errors::non_string_key_error());
        };
        validate_public_name("vars", name)?;
        slot_value(value, 0)?;
    }
    Ok(())
}

fn optional_secret_mapping(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("secrets") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "secrets",
        expected: "a mapping",
    })?;
    for (key, value) in mapping {
        let Some(name) = key.as_str() else {
            return Err(super::errors::non_string_key_error());
        };
        validate_public_name("secrets", name)?;
        if value.as_str().is_none() {
            return Err(CompileError::FieldShape {
                field: "secrets",
                expected: "a mapping of secret names to environment variable names",
            });
        }
    }
    Ok(())
}

fn optional_examples_sequence(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("examples") else {
        return Ok(());
    };
    let examples = node.as_sequence().ok_or(CompileError::FieldShape {
        field: "examples",
        expected: "a sequence",
    })?;
    for example in examples {
        if !example.is_mapping() {
            return Err(CompileError::FieldShape {
                field: "examples",
                expected: "a sequence of mappings",
            });
        }
        let name = required_example_name(example)?;
        validate_public_name("examples", name)?;
    }
    Ok(())
}

fn required_example_name<'a>(example: &'a Yaml<'a>) -> Result<&'a str, CompileError> {
    let name = example
        .as_mapping_get("name")
        .ok_or(CompileError::MissingField {
            field: "examples.name",
        })?;
    name.as_str().ok_or(CompileError::FieldShape {
        field: "examples.name",
        expected: "a string",
    })
}

fn validate_step_ids(steps: &saphyr::Sequence<'_>) -> Result<(), CompileError> {
    let mut seen = HashSet::with_capacity(steps.len());
    for (index, step) in steps.iter().enumerate() {
        let id = required_step_id(step, index)?;
        validate_public_name("step id", id)?;
        if !seen.insert(id) {
            return Err(CompileError::DuplicateStepId {
                id: Box::<str>::from(id),
            });
        }
    }
    Ok(())
}

fn required_step_id<'a>(step: &'a Yaml<'a>, index: usize) -> Result<&'a str, CompileError> {
    if !step.is_mapping() {
        return Err(CompileError::StepShape { step: index });
    }
    let node = step
        .as_mapping_get("id")
        .ok_or(CompileError::MissingStepId { step: index })?;
    node.as_str().ok_or(CompileError::StepFieldShape {
        step: index,
        field: "id",
        expected: "a string",
    })
}

fn validate_top_level_keys(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(mapping) = doc.as_mapping() else {
        return Err(CompileError::TopLevelNotMapping);
    };
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            return Err(super::errors::non_string_key_error());
        };
        if !is_top_level_field(field) {
            return Err(CompileError::UnknownTopLevelField {
                field: Box::<str>::from(field),
            });
        }
    }
    Ok(())
}

fn is_top_level_field(field: &str) -> bool {
    matches!(
        field,
        "version"
            | "name"
            | "when"
            | "steps"
            | "inputs"
            | "vars"
            | "secrets"
            | "result"
            | "examples"
    )
}

fn validate_workflow_version(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let version = required_string_field(doc, "version")?;
    if version == WORKFLOW_VERSION {
        Ok(())
    } else {
        Err(CompileError::InvalidVersion {
            actual: Box::<str>::from(version),
        })
    }
}

fn validate_workflow_trigger(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let triggers = required_mapping_field(doc, "when")?;
    if triggers.len() != 1 {
        return Err(CompileError::InvalidTriggerCount {
            count: triggers.len(),
        });
    }
    let Some((key, value)) = triggers.iter().next() else {
        return Err(CompileError::InvalidTriggerCount { count: 0 });
    };
    let Some(trigger) = key.as_str() else {
        return Err(super::errors::non_string_key_error());
    };
    match trigger {
        "manual" => validate_manual_trigger(value),
        "webhook" => validate_webhook_trigger(value),
        "schedule" => validate_schedule_trigger(value),
        "event" => validate_event_trigger(value),
        value => Err(CompileError::UnknownTriggerKind {
            trigger: Box::<str>::from(value),
        }),
    }
}

fn validate_manual_trigger(node: &Yaml<'_>) -> Result<(), CompileError> {
    let mapping = trigger_mapping("manual", node)?;
    reject_unknown_trigger_fields("manual", mapping, &[])
}

fn validate_webhook_trigger(node: &Yaml<'_>) -> Result<(), CompileError> {
    let mapping = trigger_mapping("webhook", node)?;
    reject_unknown_trigger_fields("webhook", mapping, &["path", "method", "unique"])?;
    let path = required_trigger_string_field(node, "webhook", "path")?;
    if !path.starts_with('/') {
        return Err(CompileError::InvalidTriggerField {
            trigger: "webhook",
            field: "path",
            expected: "a string starting with /",
        });
    }
    let method = required_trigger_string_field(node, "webhook", "method")?;
    if !is_webhook_method(method) {
        return Err(CompileError::InvalidTriggerField {
            trigger: "webhook",
            field: "method",
            expected: "one of GET, POST, PUT, PATCH, DELETE",
        });
    }
    optional_trigger_string_field(node, "webhook", "unique")
}

fn validate_schedule_trigger(node: &Yaml<'_>) -> Result<(), CompileError> {
    let mapping = trigger_mapping("schedule", node)?;
    reject_unknown_trigger_fields("schedule", mapping, &["cron", "timezone"])?;
    let cron = required_trigger_string_field(node, "schedule", "cron")?;
    if cron.split_whitespace().count() != 5 {
        return Err(CompileError::InvalidTriggerField {
            trigger: "schedule",
            field: "cron",
            expected: "a five-field cron expression",
        });
    }
    optional_trigger_string_field(node, "schedule", "timezone")
}

fn validate_event_trigger(node: &Yaml<'_>) -> Result<(), CompileError> {
    let mapping = trigger_mapping("event", node)?;
    reject_unknown_trigger_fields("event", mapping, &["name"])?;
    required_trigger_string_field(node, "event", "name").map(|_| ())
}

fn trigger_mapping<'a>(
    trigger: &str,
    node: &'a Yaml<'a>,
) -> Result<&'a saphyr::Mapping<'a>, CompileError> {
    node.as_mapping().ok_or_else(|| CompileError::TriggerShape {
        trigger: Box::<str>::from(trigger),
        expected: "a mapping",
    })
}

fn reject_unknown_trigger_fields(
    trigger: &'static str,
    mapping: &saphyr::Mapping<'_>,
    allowed: &[&str],
) -> Result<(), CompileError> {
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            return Err(super::errors::non_string_key_error());
        };
        if !allowed.contains(&field) {
            return Err(CompileError::UnknownTriggerField {
                trigger,
                field: Box::<str>::from(field),
            });
        }
    }
    Ok(())
}

fn required_trigger_string_field<'a>(
    node: &'a Yaml<'a>,
    trigger: &'static str,
    field: &'static str,
) -> Result<&'a str, CompileError> {
    let value = node
        .as_mapping_get(field)
        .ok_or(CompileError::MissingTriggerField { trigger, field })?;
    value.as_str().ok_or(CompileError::InvalidTriggerField {
        trigger,
        field,
        expected: "a string",
    })
}

fn optional_trigger_string_field(
    node: &Yaml<'_>,
    trigger: &'static str,
    field: &'static str,
) -> Result<(), CompileError> {
    match node.as_mapping_get(field) {
        Some(value) if value.as_str().is_none() => Err(CompileError::InvalidTriggerField {
            trigger,
            field,
            expected: "a string",
        }),
        _ => Ok(()),
    }
}

fn is_webhook_method(method: &str) -> bool {
    matches!(method, "GET" | "POST" | "PUT" | "PATCH" | "DELETE")
}

fn required_string_field<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a str, CompileError> {
    let node = doc
        .as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?;
    node.as_str().ok_or(CompileError::FieldShape {
        field,
        expected: "a string",
    })
}

fn required_sequence_field<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a saphyr::Sequence<'a>, CompileError> {
    let node = doc
        .as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?;
    node.as_sequence().ok_or(CompileError::FieldShape {
        field,
        expected: "a sequence",
    })
}

fn required_mapping_field<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a saphyr::Mapping<'a>, CompileError> {
    let node = doc
        .as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?;
    node.as_mapping().ok_or(CompileError::FieldShape {
        field,
        expected: "a mapping",
    })
}

/// Workflow builder for accumulating compiled nodes.
#[derive(Debug, Default)]
struct WorkflowBuilder {
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    max_slot: Option<usize>,
}

impl WorkflowBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn push_constant(&mut self, value: ConstValue) -> Result<ConstIdx, CompileError> {
        let index =
            u16::try_from(self.constants.len()).map_err(|_| CompileError::Workflow(
                WorkflowError::ConstOutOfBounds {
                    constant: ConstIdx::new(u16::MAX),
                },
            ))?;
        self.constants.push(value);
        Ok(ConstIdx::new(index))
    }

    fn record_slot(&mut self, slot: SlotIdx) {
        let value = slot.as_usize();
        self.max_slot = Some(match self.max_slot {
            Some(current) => current.max(value),
            None => value,
        });
    }

    fn slot_count(&self) -> Result<u16, CompileError> {
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
}

fn compile_step(
    step: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    source_ir_starts: &[StepIdx],
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    let StepSpec { primitive, body } = step_spec(step, index)?;
    let node = match primitive {
        StepPrimitive::Run | StepPrimitive::Do => compile_run(
            body,
            index,
            last_step,
            id,
            next,
            primitive.as_str(),
            builder,
        ),
        StepPrimitive::Set | StepPrimitive::Save => compile_save(
            body,
            index,
            last_step,
            id,
            next,
            primitive.as_str(),
            builder,
        ),
        StepPrimitive::Choose => {
            compile_choose(body, index, last_step, id, source_ir_starts, builder)
        }
        StepPrimitive::ForEach => return compile_for_each(body, index, last_step, id, builder),
        StepPrimitive::Together => {
            return compile_together(body, index, last_step, id, source_ir_starts, builder);
        }
        StepPrimitive::Collect => {
            return compile_collect(body, index, last_step, id, next, builder);
        }
        StepPrimitive::Reduce => {
            return compile_reduce(body, index, last_step, id, next, builder);
        }
        StepPrimitive::Repeat => {
            return compile_repeat(body, index, last_step, id, next, builder);
        }
        StepPrimitive::Wait => compile_wait(body, index, last_step, id, next, builder),
        StepPrimitive::Ask => return compile_ask(body, index, last_step, id, next, builder),
        StepPrimitive::Finish => return compile_finish(body, index, last_step, id, builder),
    }?;
    Ok(vec![node])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepPrimitive {
    Set,
    Run,
    Do,
    Save,
    Choose,
    ForEach,
    Together,
    Collect,
    Reduce,
    Repeat,
    Wait,
    Ask,
    Finish,
}

impl StepPrimitive {
    fn from_field(field: &str) -> Option<Self> {
        match field {
            "set" => Some(Self::Set),
            "run" => Some(Self::Run),
            "do" => Some(Self::Do),
            "save" => Some(Self::Save),
            "choose" => Some(Self::Choose),
            "for_each" => Some(Self::ForEach),
            "together" => Some(Self::Together),
            "collect" => Some(Self::Collect),
            "reduce" => Some(Self::Reduce),
            "repeat" => Some(Self::Repeat),
            "wait" => Some(Self::Wait),
            "ask" => Some(Self::Ask),
            "finish" => Some(Self::Finish),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Run => "run",
            Self::Do => "do",
            Self::Save => "save",
            Self::Choose => "choose",
            Self::ForEach => "for_each",
            Self::Together => "together",
            Self::Collect => "collect",
            Self::Reduce => "reduce",
            Self::Repeat => "repeat",
            Self::Wait => "wait",
            Self::Ask => "ask",
            Self::Finish => "finish",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct StepSpec<'a> {
    primitive: StepPrimitive,
    body: &'a Yaml<'a>,
}

#[derive(Debug, Clone, Copy)]
enum ChooseCondition {
    Slot(SlotIdx),
    Literal(bool),
}

fn step_spec<'a>(step: &'a Yaml<'a>, index: usize) -> Result<StepSpec<'a>, CompileError> {
    let Some(mapping) = step.as_mapping() else {
        return Err(CompileError::StepShape { step: index });
    };
    let mut selected = None;

    for (key, body) in mapping {
        let Some(field) = key.as_str() else {
            return Err(CompileError::StepShape { step: index });
        };
        if let Some(primitive) = StepPrimitive::from_field(field) {
            if selected.is_some() {
                return Err(CompileError::MultipleStepPrimitives { step: index });
            }
            selected = Some(StepSpec { primitive, body });
        } else {
            validate_phase_zero_step_metadata(field, body, index)?;
        }
    }

    selected.ok_or(CompileError::MissingStepPrimitive { step: index })
}

fn validate_phase_zero_step_metadata(
    field: &str,
    body: &Yaml<'_>,
    step: usize,
) -> Result<(), CompileError> {
    match field {
        "id" => Ok(()),
        "name" => validate_step_display_name(body, step),
        "if" | "with" | "try_again" | "on_error" | "then" => {
            Err(CompileError::UnsupportedStepControlField {
                step,
                field: Box::<str>::from(field),
            })
        }
        _ => Err(CompileError::UnknownStepField {
            step,
            field: Box::<str>::from(field),
        }),
    }
}

fn validate_step_display_name(body: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    if body.as_str().is_some() {
        Ok(())
    } else {
        Err(CompileError::StepFieldShape {
            step,
            field: "name",
            expected: "a string",
        })
    }
}

fn compile_run(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    primitive: &'static str,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, primitive, &["action", "input"])?;
    let action = required_action(body, index, primitive)?;
    let input = required_slot(body, index, "input")?;
    let output = slot_idx_for_step(index)?;
    builder.record_slot(input);
    builder.record_slot(output);
    Ok(lower_do(
        id,
        action,
        input,
        Some(output),
        Some(required_next_step(next, index)?),
        &mut SlotCompiler::new(),
    ))
}

fn compile_save(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    primitive: &'static str,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_non_mapping_step_body(body, index, primitive, "an object")?;
    let output = slot_idx_for_step(index)?;
    let constant = save_slot_value(body, index, primitive)?;
    let constant = builder.push_constant(constant)?;
    builder.record_slot(output);
    set_const_node(id, output, constant, required_next_step(next, index)?)
}

fn reject_non_mapping_step_body(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
    expected: &'static str,
) -> Result<(), CompileError> {
    if body.is_mapping() {
        Ok(())
    } else {
        Err(CompileError::StepFieldShape {
            step,
            field,
            expected,
        })
    }
}

#[allow(clippy::unnecessary_wraps)]
fn set_const_node(
    id: StepIdx,
    output: SlotIdx,
    value: ConstIdx,
    next: StepIdx,
) -> Result<CompiledNode, CompileError> {
    Ok(CompiledNode {
        id,
        output: Some(output),
        next: Some(next),
        kind: CompiledNodeKind::SetConst { value },
    })
}

fn save_slot_value(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
) -> Result<ConstValue, CompileError> {
    let Some(mapping) = body.as_mapping() else {
        return Err(CompileError::StepFieldShape {
            step,
            field: primitive,
            expected: "an object",
        });
    };
    if mapping.len() != 1 {
        return Err(CompileError::UnsupportedConstantValue { step });
    }
    match mapping.iter().next() {
        Some((key, value)) if key.as_str() == Some("value") => slot_value(value, step),
        Some((key, _)) if key.as_str().is_none() => Err(super::errors::non_string_key_error()),
        Some(_) | None => Err(CompileError::UnsupportedConstantValue { step }),
    }
}

fn compile_choose(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    source_ir_starts: &[StepIdx],
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(
        body,
        index,
        "choose",
        &["condition", "on_true", "on_false"],
    )?;
    let condition = required_choose_condition(body, index)?;
    let on_true = mapped_branch_target(body, index, "on_true", source_ir_starts)?;
    let on_false = mapped_branch_target(body, index, "on_false", source_ir_starts)?;
    match condition {
        ChooseCondition::Slot(condition) => {
            compile_slot_choose(id, condition, on_true, on_false, builder)
        }
        ChooseCondition::Literal(value) => {
            compile_literal_choose(index, id, value, on_true, on_false, builder)
        }
    }
}

#[allow(clippy::unnecessary_wraps)]
fn compile_slot_choose(
    id: StepIdx,
    condition: SlotIdx,
    on_true: StepIdx,
    on_false: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    builder.record_slot(condition);
    Ok(CompiledNode {
        id,
        output: None,
        next: None,
        kind: CompiledNodeKind::ChooseSlot {
            branches: vec![vb_core::SlotBranch {
                condition,
                target: on_true,
            }]
            .into_boxed_slice(),
            otherwise: Some(on_false),
        },
    })
}

fn compile_literal_choose(
    index: usize,
    id: StepIdx,
    value: bool,
    on_true: StepIdx,
    on_false: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    let output = slot_idx_for_step(index)?;
    let constant = builder.push_constant(ConstValue::Bool(value))?;
    builder.record_slot(output);
    Ok(CompiledNode {
        id,
        output: Some(output),
        next: Some(if value { on_true } else { on_false }),
        kind: CompiledNodeKind::SetConst { value: constant },
    })
}

fn compile_for_each(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unsupported_for_each_fields(body, index)?;
    reject_unknown_primitive_fields(body, index, "for_each", &["input", "item", "limit"])?;
    let input = required_slot(body, index, "input")?;
    let item = required_slot(body, index, "item")?;
    let limit = required_u32_field(body, index, "for_each", "limit")?;
    let body_step = checked_step_offset(id, 1, "for_each", "body")?;
    let done = checked_step_offset(id, 2, "for_each", "done")?;
    builder.record_slot(input);
    builder.record_slot(item);
    lower_for_each(
        id,
        input,
        item,
        limit,
        body_step,
        done,
        &mut SlotCompiler::new(),
    )
}

fn compile_together(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    source_ir_starts: &[StepIdx],
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "together", &["branches"])?;
    let branch_sources = required_branch_targets(body, index, "branches")?;
    let mut branches = Vec::with_capacity(branch_sources.len());
    for source in branch_sources {
        branches.push(source_ir_start(source_ir_starts, source.as_usize())?);
    }
    let branch_count = u16::try_from(branches.len()).map_err(|_| {
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "together",
            field: "branches",
            value: branches.len(),
            limit: usize::from(u16::MAX),
        }
    })?;
    let accumulator = alloc_workflow_slot(builder)?;
    let join = checked_step_offset(id, 1, "together", "join")?;
    Ok(vec![
        CompiledNode {
            id,
            output: Some(accumulator),
            next: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: branches.into_boxed_slice(),
                join,
            },
        },
        CompiledNode {
            id: join,
            output: Some(accumulator),
            next: None,
            kind: CompiledNodeKind::TogetherJoin {
                branch_count,
                accumulator,
            },
        },
    ])
}

fn compile_collect(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(
        body,
        index,
        "collect",
        &["source", "limit", "page_size"],
    )?;
    let source = required_slot(body, index, "source")?;
    let limit = required_u32_field(body, index, "collect", "limit")?;
    let page_size = required_u32_field(body, index, "collect", "page_size")?;
    let body_step = checked_step_offset(id, 1, "collect", "body")?;
    let done = checked_step_offset(id, 2, "collect", "done")?;
    builder.record_slot(source);
    let mut nodes = lower_collect(
        id,
        source,
        limit,
        page_size,
        body_step,
        done,
        &mut SlotCompiler::new(),
    )?;
    // CollectFinish (index 2) chains to the next step.
    if let Some(finish) = nodes.get_mut(2) {
        finish.next = next;
    }
    Ok(nodes)
}

fn compile_reduce(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "reduce", &["input", "accumulator", "initial"])?;
    let input = required_slot(body, index, "input")?;
    let accumulator = required_slot(body, index, "accumulator")?;
    let initial = slot_value(required_step_field(body, index, "initial")?, index)?;
    let initial = builder.push_constant(initial)?;
    let body_step = checked_step_offset(id, 1, "reduce", "body")?;
    let done = checked_step_offset(id, 2, "reduce", "done")?;
    builder.record_slot(input);
    builder.record_slot(accumulator);
    let mut nodes = lower_reduce(
        id,
        input,
        accumulator,
        initial,
        body_step,
        done,
        &mut SlotCompiler::new(),
    )?;
    // ReduceFinish (index 2) chains to the next step.
    if let Some(finish) = nodes.get_mut(2) {
        finish.next = next;
    }
    Ok(nodes)
}

fn compile_repeat(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "repeat", &["max_attempts"])?;
    let max_attempts = required_u16_field(body, index, "repeat", "max_attempts")?;
    let body_step = checked_step_offset(id, 1, "repeat", "body")?;
    let done = checked_step_offset(id, 2, "repeat", "done")?;
    let attempt_slot = slot_idx_for_step(
        id.as_usize()
            .checked_add(1)
            .ok_or({
                CompileError::PrimitiveLoweringLimitExceeded {
                    primitive: "repeat",
                    field: "attempt_slot",
                    value: id.as_usize(),
                    limit: usize::from(u16::MAX),
                }
            })?,
    )?;
    builder.record_slot(attempt_slot);
    let mut nodes = lower_repeat(
        id,
        max_attempts,
        body_step,
        done,
        &mut SlotCompiler::new(),
    )?;
    // RepeatFinish (index 2) chains to the next step.
    if let Some(finish) = nodes.get_mut(2) {
        finish.next = next;
    }
    Ok(nodes)
}

fn compile_wait(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "wait", &["until", "event", "timeout"])?;
    let until = optional_slot_field(body, index, "until")?;
    let event = optional_slot_field(body, index, "event")?;
    let timeout = optional_slot_field(body, index, "timeout")?;
    let mut node = match (until, event, timeout) {
        (Some(deadline), None, None) => {
            builder.record_slot(deadline);
            lower_wait(id, deadline, None, false, &mut SlotCompiler::new())
        }
        (None, Some(event_slot), timeout_slot) => {
            builder.record_slot(event_slot);
            if let Some(slot) = timeout_slot {
                builder.record_slot(slot);
            }
            lower_wait(id, event_slot, timeout_slot, true, &mut SlotCompiler::new())
        }
        _ => {
            return Err(CompileError::StepFieldShape {
                step: index,
                field: "wait",
                expected: "until without timeout or event with optional timeout",
            });
        }
    };
    node.next = next;
    Ok(node)
}

fn compile_ask(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "ask", &["prompt", "answer", "timeout"])?;
    let prompt = required_slot(body, index, "prompt")?;
    let answer = required_slot(body, index, "answer")?;
    let timeout = optional_slot_field(body, index, "timeout")?;
    builder.record_slot(prompt);
    builder.record_slot(answer);
    if let Some(slot) = timeout {
        builder.record_slot(slot);
    }
    let mut nodes = lower_ask(id, prompt, answer, timeout, &mut SlotCompiler::new())?;
    // Ask (index 0) chains to AskResume for structural reachability.
    if let (Some(_ask_node), Some(resume_node)) = (nodes.first(), nodes.get(1)) {
        let resume_id = resume_node.id;
        if let Some(ask_node) = nodes.first_mut() {
            ask_node.next = Some(resume_id);
        }
    }
    // AskResume (index 1) chains to the next step.
    if let Some(resume) = nodes.get_mut(1) {
        resume.next = next;
    }
    Ok(nodes)
}

fn compile_finish(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    if index != last_step {
        return Err(CompileError::StepFieldShape {
            step: index,
            field: "finish",
            expected: "the last step",
        });
    }
    reject_unknown_primitive_fields(body, index, "finish", &["result"])?;
    let result = required_step_field(body, index, "result")?;
    compile_finish_result(result, index, id, builder)
}

fn compile_finish_result(
    result: &Yaml<'_>,
    index: usize,
    id: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    if let Some(slot) = finish_result_slot(result, index)? {
        builder.record_slot(slot);
        return Ok(vec![CompiledNode {
            id,
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish { result: slot },
        }]);
    }
    let value = slot_value(result, index)?;
    let value = builder.push_constant(value)?;
    let output = slot_idx_for_step(index)?;
    builder.record_slot(output);
    let finish_id = id
        .checked_add(1)
        .ok_or(CompileError::StepIndexOutOfRange { value: id.as_usize() })?;
    Ok(vec![
        CompiledNode {
            id,
            output: Some(output),
            next: Some(finish_id),
            kind: CompiledNodeKind::SetConst { value },
        },
        CompiledNode {
            id: finish_id,
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish { result: output },
        },
    ])
}

fn finish_result_slot(result: &Yaml<'_>, index: usize) -> Result<Option<SlotIdx>, CompileError> {
    let Some(value) = result.as_integer() else {
        return Ok(None);
    };
    if !finish_integer_is_slot(value, index) {
        return Ok(None);
    }
    let value = u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
    Ok(Some(SlotIdx::new(value)))
}

fn finish_integer_is_slot(value: i64, index: usize) -> bool {
    match usize::try_from(value) {
        Ok(slot) => slot <= index,
        Err(_) => false,
    }
}

fn reject_last_non_finish(index: usize, last_step: usize) -> Result<(), CompileError> {
    if index == last_step {
        Err(CompileError::LastStepMustFinish)
    } else {
        Ok(())
    }
}

fn required_next_step(next: Option<StepIdx>, index: usize) -> Result<StepIdx, CompileError> {
    next.ok_or(CompileError::StepIndexOutOfRange { value: index })
}

fn mapped_branch_target(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
    source_ir_starts: &[StepIdx],
) -> Result<StepIdx, CompileError> {
    let source = required_branch_target(body, step, field)?;
    source_ir_start(source_ir_starts, source.as_usize())
}

fn reject_unknown_primitive_fields(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    allowed: &[&str],
) -> Result<(), CompileError> {
    let mapping = primitive_body_mapping(body, step, primitive)?;
    for (key, _) in mapping {
        reject_unknown_primitive_field(key, step, primitive, allowed)?;
    }
    Ok(())
}

fn primitive_body_mapping<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    primitive: &'static str,
) -> Result<&'a saphyr::Mapping<'a>, CompileError> {
    body.as_mapping().ok_or(CompileError::StepFieldShape {
        step,
        field: primitive,
        expected: "a mapping",
    })
}

fn reject_unknown_primitive_field(
    key: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    allowed: &[&str],
) -> Result<(), CompileError> {
    let Some(field) = key.as_str() else {
        return Err(CompileError::StepShape { step });
    };
    if allowed.contains(&field) {
        Ok(())
    } else {
        Err(CompileError::UnknownStepPrimitiveField {
            step,
            primitive,
            field: Box::<str>::from(field),
        })
    }
}

fn required_slot(body: &Yaml<'_>, step: usize, field: &'static str) -> Result<SlotIdx, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "an integer slot index",
    })?;
    let value = u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
    Ok(SlotIdx::new(value))
}

fn required_u32_field(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    field: &'static str,
) -> Result<u32, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "a non-negative u32 integer",
    })?;
    u32::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive,
        field,
        value: integer_error_value(value),
        limit: usize::try_from(u32::MAX).map_or(usize::MAX, |limit| limit),
    })
}

fn required_u16_field(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    field: &'static str,
) -> Result<u16, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "a non-negative u16 integer",
    })?;
    u16::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive,
        field,
        value: integer_error_value(value),
        limit: usize::from(u16::MAX),
    })
}

fn integer_error_value(value: i64) -> usize {
    match usize::try_from(value) {
        Ok(value) => value,
        Err(_) => usize::MAX,
    }
}

fn required_branch_targets(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<Vec<StepIdx>, CompileError> {
    let node = required_step_field(body, step, field)?;
    let sequence = node.as_sequence().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "a sequence of integer step indexes",
    })?;
    if sequence.is_empty() {
        return Err(CompileError::StepFieldShape {
            step,
            field,
            expected: "at least one integer step index",
        });
    }
    let mut targets = Vec::with_capacity(sequence.len());
    let mut index = 0usize;
    while index < sequence.len() {
        let Some(node) = sequence.get(index) else {
            return Err(CompileError::StepIndexOutOfRange { value: index });
        };
        let value = node.as_integer().ok_or(CompileError::StepFieldShape {
            step,
            field,
            expected: "a sequence of integer step indexes",
        })?;
        let value =
            u16::try_from(value).map_err(|_| CompileError::BranchTargetOutOfRange { value })?;
        targets.push(StepIdx::new(value));
        index = index
            .checked_add(1)
            .ok_or(CompileError::StepIndexOutOfRange { value: index })?;
    }
    Ok(targets)
}

fn checked_step_offset(
    id: StepIdx,
    offset: u16,
    primitive: &'static str,
    field: &'static str,
) -> Result<StepIdx, CompileError> {
    id.checked_add(offset)
        .ok_or(CompileError::PrimitiveLoweringLimitExceeded {
            primitive,
            field,
            value: id.as_usize(),
            limit: usize::from(u16::MAX),
        })
}

fn alloc_workflow_slot(builder: &mut WorkflowBuilder) -> Result<SlotIdx, CompileError> {
    let value = builder.slot_count()?;
    let slot = SlotIdx::new(value);
    builder.record_slot(slot);
    Ok(slot)
}

fn required_action(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
) -> Result<vb_core::ActionId, CompileError> {
    let node = required_step_field(body, step, "action")?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field: "action",
        expected: "an integer action id",
    })?;
    let value =
        u16::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
            primitive,
            field: "action",
            value: usize::from(u16::MAX),
            limit: usize::from(u16::MAX),
        })?;
    Ok(vb_core::ActionId::new(value))
}

fn required_choose_condition(
    body: &Yaml<'_>,
    step: usize,
) -> Result<ChooseCondition, CompileError> {
    let node = required_step_field(body, step, "condition")?;
    if let Some(value) = node.as_bool() {
        return Ok(ChooseCondition::Literal(value));
    }
    required_slot(body, step, "condition").map(ChooseCondition::Slot)
}

fn required_branch_target(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<StepIdx, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "an integer step index",
    })?;
    let value = u16::try_from(value).map_err(|_| CompileError::BranchTargetOutOfRange { value })?;
    Ok(StepIdx::new(value))
}

fn slot_value(node: &Yaml<'_>, step: usize) -> Result<ConstValue, CompileError> {
    match node {
        Yaml::Value(saphyr::Scalar::Null) => Ok(ConstValue::Null),
        Yaml::Value(saphyr::Scalar::Boolean(value)) => Ok(ConstValue::Bool(*value)),
        Yaml::Value(saphyr::Scalar::Integer(value)) => Ok(ConstValue::I64(*value)),
        Yaml::Value(saphyr::Scalar::String(value))
        | Yaml::Representation(value, _, None) => text_slot_value(value.as_ref(), step),
        Yaml::Sequence(sequence) => list_slot_value(sequence, step),
        Yaml::Mapping(mapping) => object_slot_value(mapping, step),
        _ => Err(CompileError::UnsupportedConstantValue { step }),
    }
}

fn text_slot_value(_value: &str, step: usize) -> Result<ConstValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

fn list_slot_value(_sequence: &saphyr::Sequence<'_>, step: usize) -> Result<ConstValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

fn object_slot_value(
    _mapping: &saphyr::Mapping<'_>,
    step: usize,
) -> Result<ConstValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

fn required_step_field<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    field: &'static str,
) -> Result<&'a Yaml<'a>, CompileError> {
    body.as_mapping_get(field)
        .ok_or(CompileError::MissingStepField { step, field })
}

fn optional_slot_field(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<Option<SlotIdx>, CompileError> {
    match body.as_mapping_get(field) {
        Some(_) => required_slot(body, step, field).map(Some),
        None => Ok(None),
    }
}
