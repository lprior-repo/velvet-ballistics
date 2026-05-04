#![forbid(unsafe_code)]
//! Step compilation helper functions.

use saphyr::Yaml;
use vb_core::{ActionId, ConstValue, SlotIdx, StepIdx};

use super::slot_compiler::CompileError;
use super::compile_step_primitives::ChooseCondition;

// ============================================================================
// Slot and value helpers
// ============================================================================

pub fn slot_idx_for_step(value: usize) -> Result<SlotIdx, CompileError> {
    let value = u16::try_from(value).map_err(|_| CompileError::StepIndexOutOfRange { value })?;
    Ok(SlotIdx::new(value))
}

pub fn required_slot(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<SlotIdx, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "an integer slot index",
    })?;
    let value = u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
    Ok(SlotIdx::new(value))
}

pub fn optional_slot_field(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<Option<SlotIdx>, CompileError> {
    match body.as_mapping_get(field) {
        Some(_) => required_slot(body, step, field).map(Some),
        None => Ok(None),
    }
}

pub fn required_u32_field(
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

pub fn required_u16_field(
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

pub fn required_action(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
) -> Result<ActionId, CompileError> {
    let node = required_step_field(body, step, "action")?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field: "action",
        expected: "an integer action id",
    })?;
    let value = u16::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive,
        field: "action",
        value: usize::from(u16::MAX),
        limit: usize::from(u16::MAX),
    })?;
    Ok(ActionId::new(value))
}

pub fn required_step_field<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    field: &'static str,
) -> Result<&'a Yaml<'a>, CompileError> {
    body.as_mapping_get(field)
        .ok_or(CompileError::MissingStepField { step, field })
}

// ============================================================================
// Branch target helpers
// ============================================================================

pub fn required_branch_targets(
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

pub fn required_branch_target(
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

pub fn required_choose_condition(
    body: &Yaml<'_>,
    step: usize,
) -> Result<ChooseCondition, CompileError> {
    let node = required_step_field(body, step, "condition")?;
    if let Some(value) = node.as_bool() {
        return Ok(ChooseCondition::Literal(value));
    }
    required_slot(body, step, "condition").map(ChooseCondition::Slot)
}

// ============================================================================
// Step offset helpers
// ============================================================================

pub fn checked_step_offset(
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

pub fn alloc_workflow_slot(builder: &mut super::compile_step::WorkflowBuilder) -> Result<SlotIdx, CompileError> {
    let value = builder.slot_count()?;
    let slot = SlotIdx::new(value);
    builder.record_slot(slot);
    Ok(slot)
}

pub fn source_ir_start(starts: &[StepIdx], index: usize) -> Result<StepIdx, CompileError> {
    starts
        .get(index)
        .copied()
        .ok_or(CompileError::StepIndexOutOfRange { value: index })
}

pub fn mapped_branch_target(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
    source_ir_starts: &[StepIdx],
) -> Result<StepIdx, CompileError> {
    let source = required_branch_target(body, step, field)?;
    source_ir_start(source_ir_starts, source.as_usize())
}

pub fn required_next_step(next: Option<StepIdx>, index: usize) -> Result<StepIdx, CompileError> {
    next.ok_or(CompileError::StepIndexOutOfRange { value: index })
}

// ============================================================================
// Validation helpers
// ============================================================================

pub fn reject_last_non_finish(index: usize, last_step: usize) -> Result<(), CompileError> {
    if index == last_step {
        Err(CompileError::LastStepMustFinish)
    } else {
        Ok(())
    }
}

pub fn reject_unknown_primitive_fields(
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

pub fn reject_non_mapping_step_body(
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

pub fn reject_unsupported_for_each_fields(body: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
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

// ============================================================================
// Constant value helpers
// ============================================================================

pub fn slot_value(node: &Yaml<'_>, step: usize) -> Result<ConstValue, CompileError> {
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

fn list_slot_value(
    _sequence: &saphyr::Sequence<'_>,
    step: usize,
) -> Result<ConstValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

fn object_slot_value(
    _mapping: &saphyr::Mapping<'_>,
    step: usize,
) -> Result<ConstValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

#[cfg(test)]
mod tests {
    use super::*;
    use saphyr::LoadableYamlNode;

    fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
        if condition {
            Ok(())
        } else {
            Err(message.to_owned())
        }
    }

    fn yaml_node(source: &str) -> Result<Yaml<'_>, String> {
        let docs = Yaml::load_from_str(source).map_err(|e| format!("yaml load: {e:?}"))?;
        docs.first()
            .cloned()
            .ok_or_else(|| "empty document".to_owned())
    }

    // -- slot_value: constant extraction from YAML values --

    #[test]
    fn slot_value_parses_null() -> Result<(), String> {
        let node = yaml_node("null")?;
        let value = slot_value(&node, 0).map_err(|e| format!("slot_value: {e:?}"))?;
        ensure(value == ConstValue::Null, "null should map to Null")
    }

    #[test]
    fn slot_value_parses_true() -> Result<(), String> {
        let node = yaml_node("true")?;
        let value = slot_value(&node, 0).map_err(|e| format!("slot_value: {e:?}"))?;
        ensure(value == ConstValue::Bool(true), "true should map to Bool(true)")
    }

    #[test]
    fn slot_value_parses_false() -> Result<(), String> {
        let node = yaml_node("false")?;
        let value = slot_value(&node, 0).map_err(|e| format!("slot_value: {e:?}"))?;
        ensure(value == ConstValue::Bool(false), "false should map to Bool(false)")
    }

    #[test]
    fn slot_value_parses_integer() -> Result<(), String> {
        let node = yaml_node("42")?;
        let value = slot_value(&node, 0).map_err(|e| format!("slot_value: {e:?}"))?;
        ensure(value == ConstValue::I64(42), "integer should map to I64")
    }

    #[test]
    fn slot_value_parses_negative_integer() -> Result<(), String> {
        let node = yaml_node("-7")?;
        let value = slot_value(&node, 0).map_err(|e| format!("slot_value: {e:?}"))?;
        ensure(value == ConstValue::I64(-7), "negative integer should map to I64(-7)")
    }

    #[test]
    fn slot_value_rejects_string() -> Result<(), String> {
        let node = yaml_node("hello")?;
        match slot_value(&node, 5) {
            Err(CompileError::UnsupportedConstantValue { step: 5 }) => Ok(()),
            other => Err(format!("expected UnsupportedConstantValue, got {other:?}")),
        }
    }

    #[test]
    fn slot_value_rejects_sequence() -> Result<(), String> {
        let node = yaml_node("[1, 2]")?;
        match slot_value(&node, 0) {
            Err(CompileError::UnsupportedConstantValue { .. }) => Ok(()),
            other => Err(format!("expected UnsupportedConstantValue for sequence, got {other:?}")),
        }
    }

    #[test]
    fn slot_value_rejects_mapping() -> Result<(), String> {
        let node = yaml_node("a: 1")?;
        match slot_value(&node, 0) {
            Err(CompileError::UnsupportedConstantValue { .. }) => Ok(()),
            other => Err(format!("expected UnsupportedConstantValue for mapping, got {other:?}")),
        }
    }

    // -- slot_idx_for_step --

    #[test]
    fn slot_idx_for_step_valid() -> Result<(), String> {
        let slot = slot_idx_for_step(0)?;
        ensure(slot.as_u16() == 0, "step 0 should produce slot 0")?;
        let slot = slot_idx_for_step(100)?;
        ensure(slot.as_u16() == 100, "step 100 should produce slot 100")
    }

    #[test]
    fn slot_idx_for_step_overflow() -> Result<(), String> {
        match slot_idx_for_step(70000) {
            Err(CompileError::StepIndexOutOfRange { .. }) => Ok(()),
            other => Err(format!("expected StepIndexOutOfRange, got {other:?}")),
        }
    }

    // -- reject_last_non_finish --

    #[test]
    fn reject_last_non_finish_allows_non_last() -> Result<(), String> {
        reject_last_non_finish(0, 2)?;
        reject_last_non_finish(1, 2)?;
        Ok(())
    }

    #[test]
    fn reject_last_non_finish_rejects_last() -> Result<(), String> {
        match reject_last_non_finish(2, 2) {
            Err(CompileError::LastStepMustFinish) => Ok(()),
            other => Err(format!("expected LastStepMustFinish, got {other:?}")),
        }
    }

    // -- required_slot --

    #[test]
    fn required_slot_parses_valid_integer() -> Result<(), String> {
        let body = yaml_node("input: 3")?;
        let slot = required_slot(&body, 0, "input")?;
        ensure(slot.as_u16() == 3, "should parse slot 3")
    }

    #[test]
    fn required_slot_rejects_missing_field() -> Result<(), String> {
        let body = yaml_node("other: 3")?;
        match required_slot(&body, 0, "input") {
            Err(CompileError::MissingStepField { step: 0, field: "input" }) => Ok(()),
            other => Err(format!("expected MissingStepField, got {other:?}")),
        }
    }

    #[test]
    fn required_slot_rejects_non_integer() -> Result<(), String> {
        let body = yaml_node("input: hello")?;
        match required_slot(&body, 0, "input") {
            Err(CompileError::StepFieldShape { .. }) => Ok(()),
            other => Err(format!("expected StepFieldShape, got {other:?}")),
        }
    }

    #[test]
    fn required_slot_rejects_out_of_range() -> Result<(), String> {
        let body = yaml_node("input: 70000")?;
        match required_slot(&body, 0, "input") {
            Err(CompileError::SlotIndexOutOfRange { .. }) => Ok(()),
            other => Err(format!("expected SlotIndexOutOfRange, got {other:?}")),
        }
    }

    // -- required_u32_field --

    #[test]
    fn required_u32_field_parses_valid() -> Result<(), String> {
        let body = yaml_node("limit: 10")?;
        let value = required_u32_field(&body, 0, "for_each", "limit")?;
        ensure(value == 10, "limit should be 10")
    }

    #[test]
    fn required_u32_field_rejects_negative() -> Result<(), String> {
        let body = yaml_node("limit: -1")?;
        match required_u32_field(&body, 0, "for_each", "limit") {
            Err(CompileError::PrimitiveLoweringLimitExceeded { .. }) => Ok(()),
            other => Err(format!("expected PrimitiveLoweringLimitExceeded, got {other:?}")),
        }
    }

    // -- required_u16_field --

    #[test]
    fn required_u16_field_parses_valid() -> Result<(), String> {
        let body = yaml_node("max_attempts: 3")?;
        let value = required_u16_field(&body, 0, "repeat", "max_attempts")?;
        ensure(value == 3, "max_attempts should be 3")
    }

    #[test]
    fn required_u16_field_rejects_too_large() -> Result<(), String> {
        let body = yaml_node("max_attempts: 70000")?;
        match required_u16_field(&body, 0, "repeat", "max_attempts") {
            Err(CompileError::PrimitiveLoweringLimitExceeded { .. }) => Ok(()),
            other => Err(format!("expected PrimitiveLoweringLimitExceeded, got {other:?}")),
        }
    }

    // -- required_action --

    #[test]
    fn required_action_parses_valid() -> Result<(), String> {
        let body = yaml_node("action: 5")?;
        let action = required_action(&body, 0, "do")?;
        ensure(action.as_u16() == 5, "action id should be 5")
    }

    // -- reject_unknown_primitive_fields --

    #[test]
    fn reject_unknown_primitive_fields_allows_known() -> Result<(), String> {
        let body = yaml_node("action: 1\ninput: 0")?;
        reject_unknown_primitive_fields(&body, 0, "do", &["action", "input"])
    }

    #[test]
    fn reject_unknown_primitive_fields_rejects_unknown() -> Result<(), String> {
        let body = yaml_node("action: 1\ninput: 0\nextra: true")?;
        match reject_unknown_primitive_fields(&body, 0, "do", &["action", "input"]) {
            Err(CompileError::UnknownStepPrimitiveField { step: 0, .. }) => Ok(()),
            other => Err(format!("expected UnknownStepPrimitiveField, got {other:?}")),
        }
    }

    // -- checked_step_offset --

    #[test]
    fn checked_step_offset_valid() -> Result<(), String> {
        let result = checked_step_offset(StepIdx::new(5), 2, "for_each", "body")?;
        ensure(result.as_usize() == 7, "offset should be 7")
    }

    #[test]
    fn checked_step_offset_overflow() -> Result<(), String> {
        match checked_step_offset(StepIdx::new(65535), 1, "for_each", "body") {
            Err(CompileError::PrimitiveLoweringLimitExceeded { .. }) => Ok(()),
            other => Err(format!("expected overflow error, got {other:?}")),
        }
    }

    // -- source_ir_start --

    #[test]
    fn source_ir_start_returns_valid() -> Result<(), String> {
        let starts = [StepIdx::new(0), StepIdx::new(3), StepIdx::new(7)];
        let result = source_ir_start(&starts, 1)?;
        ensure(result.as_usize() == 3, "should return starts[1]")
    }

    #[test]
    fn source_ir_start_rejects_out_of_bounds() -> Result<(), String> {
        let starts = [StepIdx::new(0)];
        match source_ir_start(&starts, 5) {
            Err(CompileError::StepIndexOutOfRange { .. }) => Ok(()),
            other => Err(format!("expected StepIndexOutOfRange, got {other:?}")),
        }
    }

    // -- reject_unsupported_for_each_fields --

    #[test]
    fn reject_unsupported_for_each_allows_standard() -> Result<(), String> {
        let body = yaml_node("input: 0\nitem: 1\nlimit: 10")?;
        reject_unsupported_for_each_fields(&body, 0)
    }

    #[test]
    fn reject_unsupported_for_each_rejects_at_once() -> Result<(), String> {
        let body = yaml_node("input: 0\nat_once: 5")?;
        match reject_unsupported_for_each_fields(&body, 0) {
            Err(CompileError::UnsupportedStepPrimitive {
                step: 0,
                primitive: "for_each",
            }) => Ok(()),
            other => Err(format!("expected UnsupportedStepPrimitive, got {other:?}")),
        }
    }

    // -- required_branch_target --

    #[test]
    fn required_branch_target_parses_valid() -> Result<(), String> {
        let body = yaml_node("on_true: 1")?;
        let target = required_branch_target(&body, 0, "on_true")?;
        ensure(target.as_usize() == 1, "target should be 1")
    }

    // -- required_branch_targets --

    #[test]
    fn required_branch_targets_parses_valid_sequence() -> Result<(), String> {
        let body = yaml_node("branches: [1, 2, 3]")?;
        let targets = required_branch_targets(&body, 0, "branches")?;
        ensure(targets.len() == 3, "should have 3 targets")?;
        ensure(targets[0].as_usize() == 1, "first target should be 1")?;
        ensure(targets[2].as_usize() == 3, "third target should be 3")
    }

    #[test]
    fn required_branch_targets_rejects_empty_sequence() -> Result<(), String> {
        let body = yaml_node("branches: []")?;
        match required_branch_targets(&body, 0, "branches") {
            Err(CompileError::StepFieldShape { .. }) => Ok(()),
            other => Err(format!("expected StepFieldShape for empty, got {other:?}")),
        }
    }

    // -- optional_slot_field --

    #[test]
    fn optional_slot_field_returns_none_when_absent() -> Result<(), String> {
        let body = yaml_node("other: 1")?;
        let result = optional_slot_field(&body, 0, "timeout")?;
        ensure(result.is_none(), "absent field should return None")
    }

    #[test]
    fn optional_slot_field_returns_some_when_present() -> Result<(), String> {
        let body = yaml_node("timeout: 5")?;
        let result = optional_slot_field(&body, 0, "timeout")?;
        let slot = result.ok_or("expected Some")?;
        ensure(slot.as_u16() == 5, "present field should parse to slot 5")
    }

    // -- non_string_key_error --

    #[test]
    fn non_string_key_error_has_unavailable_mark() -> Result<(), String> {
        let error = non_string_key_error();
        ensure(
            matches!(error, CompileError::NonStringKey { mark } if !mark.available),
            "mark should be unavailable",
        )
    }
}
