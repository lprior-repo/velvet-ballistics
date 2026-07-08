use super::*;
use crate::mod_compile_errors::CompileError;
use saphyr::Yaml;

impl StepPrimitive {
    pub(crate) fn from_field(field: &str) -> Option<Self> {
        match field {
            "set" => Some(Self::Set),
            "run" => Some(Self::Run),
            "do" => Some(Self::Do),
            "save" => Some(Self::Save),
            "choose" => Some(Self::Choose),
            "for_each" => Some(Self::ForEach),
            "together" | "parallel" => Some(Self::Parallel),
            "collect" => Some(Self::Collect),
            "aggregate" | "reduce" => Some(Self::Aggregate),
            "repeat" => Some(Self::Repeat),
            "wait" => Some(Self::Wait),
            "ask" => Some(Self::Ask),
            "finish" => Some(Self::Finish),
            _ => None,
        }
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Run => "run",
            Self::Do => "do",
            Self::Save => "save",
            Self::Choose => "choose",
            Self::ForEach => "for_each",
            Self::Parallel => "parallel",
            Self::Collect => "collect",
            Self::Aggregate => "aggregate",
            Self::Repeat => "repeat",
            Self::Wait => "wait",
            Self::Ask => "ask",
            Self::Finish => "finish",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StepSpec<'a> {
    pub(crate) primitive: StepPrimitive,
    pub(crate) body: &'a Yaml<'a>,
}

pub(crate) fn step_spec<'a>(
    step: &'a Yaml<'a>,
    index: usize,
) -> Result<StepSpec<'a>, CompileError> {
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

pub(crate) fn validate_phase_zero_step_metadata(
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

pub(super) fn validate_step_display_name(body: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
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

pub(super) fn validate_phase_zero_step_shapes(
    steps: &saphyr::Sequence<'_>,
) -> Result<(), CompileError> {
    let last_step = steps.len().checked_sub(1).ok_or(CompileError::EmptySteps)?;
    for (index, step) in steps.iter().enumerate() {
        validate_phase_zero_step_shape(step, index, last_step)?;
    }
    Ok(())
}

pub(super) fn validate_phase_zero_step_shape(
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
        StepPrimitive::Parallel => validate_parallel_shape(body, index, last_step),
        StepPrimitive::Collect => validate_collect_shape(body, index, last_step),
        StepPrimitive::Aggregate => validate_aggregate_shape(body, index, last_step),
        StepPrimitive::Repeat => validate_repeat_shape(body, index, last_step),
        StepPrimitive::Wait => validate_wait_shape(body, index, last_step),
        StepPrimitive::Ask => validate_ask_shape(body, index, last_step),
        StepPrimitive::Finish => validate_finish_shape(body, index, last_step),
    }
}

pub(super) fn validate_run_shape(
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

pub(super) fn validate_wait_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "wait", &["until", "event", "timeout"])?;
    let until = optional_slot_or_text_field(body, index, "until", "wait.until")?;
    let event = optional_slot_or_text_field(body, index, "event", "wait.event")?;
    let timeout = optional_slot_or_text_field(body, index, "timeout", "wait.timeout")?;
    match (until, event, timeout) {
        (true, false, false) | (false, true, _) | (false, false, true) => Ok(()),
        _ => Err(CompileError::StepFieldShape {
            step: index,
            field: "wait",
            expected: "until, timeout, or event with optional timeout",
        }),
    }
}

pub(super) fn validate_ask_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    if body.as_mapping_get("answer").is_none() {
        return validate_canonical_ask_shape(body, index);
    }
    reject_unknown_primitive_fields(body, index, "ask", &["prompt", "answer", "timeout"])?;
    required_slot(body, index, "prompt")?;
    required_slot(body, index, "answer")?;
    optional_slot_field(body, index, "timeout")?;
    Ok(())
}

fn validate_canonical_ask_shape(body: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    reject_unknown_primitive_fields(body, index, "ask", &["prompt", "timeout"])?;
    required_non_empty_string_field(body, index, "prompt", "ask.prompt")?;
    optional_non_empty_string_field(body, index, "timeout", "ask.timeout")
}

fn optional_slot_or_text_field(
    body: &Yaml<'_>,
    index: usize,
    key: &'static str,
    field: &'static str,
) -> Result<bool, CompileError> {
    let Some(node) = body.as_mapping_get(key) else {
        return Ok(false);
    };
    if let Some(value) = node.as_integer() {
        u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
        return Ok(true);
    }
    match node.as_str() {
        Some(value) if !value.is_empty() => Ok(true),
        _ => Err(CompileError::StepFieldShape {
            step: index,
            field,
            expected: "an integer slot index or non-empty string",
        }),
    }
}

fn required_non_empty_string_field(
    body: &Yaml<'_>,
    index: usize,
    key: &'static str,
    field: &'static str,
) -> Result<(), CompileError> {
    let node = required_step_field(body, index, key)?;
    match node.as_str() {
        Some(value) if !value.is_empty() => Ok(()),
        _ => Err(CompileError::StepFieldShape {
            step: index,
            field,
            expected: "a non-empty string",
        }),
    }
}

fn optional_non_empty_string_field(
    body: &Yaml<'_>,
    index: usize,
    key: &'static str,
    field: &'static str,
) -> Result<(), CompileError> {
    let Some(node) = body.as_mapping_get(key) else {
        return Ok(());
    };
    match node.as_str() {
        Some(value) if !value.is_empty() => Ok(()),
        _ => Err(CompileError::StepFieldShape {
            step: index,
            field,
            expected: "a non-empty string",
        }),
    }
}

pub(super) fn validate_save_shape(
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
