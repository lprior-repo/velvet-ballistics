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
            "reduce" | "aggregate" => Some(Self::Aggregate),
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
            Self::Aggregate => "reduce",
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
    validate_ast_step_shapes(steps)
}

pub(crate) fn validate_canonical_workflow_document_shape(
    doc: &Yaml<'_>,
) -> Result<(), CompileError> {
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

pub(super) fn validate_ast_step_shapes(steps: &saphyr::Sequence<'_>) -> Result<(), CompileError> {
    let last_step = steps.len().checked_sub(1).ok_or(CompileError::EmptySteps)?;
    for (index, step) in steps.iter().enumerate() {
        let StepSpec { primitive, body } = step_spec(step, index)?;
        validate_ast_step_position(primitive, index, last_step)?;
        validate_ast_step_body(primitive, body, index)?;
    }
    Ok(())
}

fn validate_ast_step_position(
    primitive: StepPrimitive,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    match (primitive, index == last_step) {
        (StepPrimitive::Finish, false) => Err(CompileError::StepFieldShape {
            step: index,
            field: "finish",
            expected: "the last step",
        }),
        (StepPrimitive::Finish, true) => Ok(()),
        (_, true) => Err(CompileError::LastStepMustFinish),
        (_, false) => Ok(()),
    }
}

fn validate_ast_step_body(
    primitive: StepPrimitive,
    body: &Yaml<'_>,
    index: usize,
) -> Result<(), CompileError> {
    match primitive {
        StepPrimitive::Finish => {
            primitive_body_mapping(body, index, "finish")?;
            required_step_field(body, index, "result")?;
            Ok(())
        }
        _ => Ok(()),
    }
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
    reject_unknown_primitive_fields(body, index, primitive, &["action", "input"])?;
    let action = required_primitive_string_field(body, index, "action", "action")?;
    if primitive == "run" && action.is_empty() {
        return Err(CompileError::StepFieldShape {
            step: index,
            field: "run.action",
            expected: "non-empty string",
        });
    }
    required_primitive_string_field(body, index, "input", "input")?;
    Ok(())
}

pub(super) fn validate_wait_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "wait", &["event", "timeout"])?;
    let has_event = validate_optional_primitive_string_field(body, index, "event", "a string")?;
    let has_timeout = validate_optional_primitive_string_field(body, index, "timeout", "a string")?;
    match (has_event, has_timeout) {
        (true, _) | (false, true) => Ok(()),
        _ => Err(CompileError::StepFieldShape {
            step: index,
            field: "wait",
            expected: "event or timeout",
        }),
    }
}

pub(super) fn validate_ask_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "ask", &["prompt", "timeout"])?;
    required_primitive_string_field(body, index, "prompt", "ask.prompt")?;
    validate_optional_primitive_string_field(body, index, "timeout", "a string")?;
    Ok(())
}

pub(super) fn validate_save_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    primitive: &'static str,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, primitive, &["output", "value"])?;
    let output = required_primitive_string_field(body, index, "output", "output")?;
    if primitive == "save" && output.is_empty() {
        return Err(CompileError::StepFieldShape {
            step: index,
            field: "save.output",
            expected: "non-empty string",
        });
    }
    required_primitive_string_field(body, index, "value", "value")?;
    Ok(())
}
