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

    // ========================================================================
    // Additional boundary and edge-case tests
    // ========================================================================

    // -- slot_idx_for_step: boundary values --

    #[test]
    fn slot_idx_for_step_at_u16_max() -> Result<(), String> {
        let slot = slot_idx_for_step(usize::from(u16::MAX))?;
        ensure(slot.as_u16() == u16::MAX, "should accept u16::MAX")
    }

    #[test]
    fn slot_idx_for_step_zero() -> Result<(), String> {
        let slot = slot_idx_for_step(0)?;
        ensure(slot.as_u16() == 0, "should accept zero")
    }

    #[test]
    fn slot_idx_for_step_just_above_u16_max() -> Result<(), String> {
        let value = usize::from(u16::MAX).checked_add(1).ok_or("overflow")?;
        match slot_idx_for_step(value) {
            Err(CompileError::StepIndexOutOfRange { .. }) => Ok(()),
            other => Err(format!("expected StepIndexOutOfRange, got {other:?}")),
        }
    }

    // -- required_slot: boundary values --

    #[test]
    fn required_slot_accepts_zero() -> Result<(), String> {
        let body = yaml_node("input: 0")?;
        let slot = required_slot(&body, 0, "input")?;
        ensure(slot.as_u16() == 0, "slot zero should be accepted")
    }

    #[test]
    fn required_slot_accepts_u16_max() -> Result<(), String> {
        let body = yaml_node("input: 65535")?;
        let slot = required_slot(&body, 0, "input")?;
        ensure(slot.as_u16() == 65535, "slot u16::MAX should be accepted")
    }

    #[test]
    fn required_slot_rejects_negative() -> Result<(), String> {
        let body = yaml_node("input: -1")?;
        match required_slot(&body, 0, "input") {
            Err(CompileError::SlotIndexOutOfRange { .. }) => Ok(()),
            other => Err(format!("expected SlotIndexOutOfRange for negative, got {other:?}")),
        }
    }

    #[test]
    fn required_slot_propagates_step_index_in_error() -> Result<(), String> {
        let body = yaml_node("other: 1")?;
        match required_slot(&body, 7, "missing_field") {
            Err(CompileError::MissingStepField { step: 7, field: "missing_field" }) => Ok(()),
            other => Err(format!("expected MissingStepField with step=7, got {other:?}")),
        }
    }

    // -- required_step_field: additional cases --

    #[test]
    fn required_step_field_returns_body_value() -> Result<(), String> {
        let body = yaml_node("target: 42")?;
        let node = required_step_field(&body, 0, "target")?;
        ensure(
            node.as_integer() == Some(42),
            "should return the YAML node for the field",
        )
    }

    #[test]
    fn required_step_field_rejects_non_mapping() -> Result<(), String> {
        let body = yaml_node("[1, 2, 3]")?;
        match required_step_field(&body, 0, "field") {
            Err(CompileError::MissingStepField { .. }) => Ok(()),
            other => Err(format!("expected MissingStepField for non-mapping body, got {other:?}")),
        }
    }

    // -- optional_slot_field: error propagation when present but invalid --

    #[test]
    fn optional_slot_field_propagates_parse_error_when_present_but_non_integer() -> Result<(), String> {
        let body = yaml_node("timeout: not_a_number")?;
        match optional_slot_field(&body, 0, "timeout") {
            Err(CompileError::StepFieldShape { .. }) => Ok(()),
            other => Err(format!("expected StepFieldShape for non-integer, got {other:?}")),
        }
    }

    #[test]
    fn optional_slot_field_propagates_range_error_when_present_but_too_large() -> Result<(), String> {
        let body = yaml_node("timeout: 70000")?;
        match optional_slot_field(&body, 0, "timeout") {
            Err(CompileError::SlotIndexOutOfRange { .. }) => Ok(()),
            other => Err(format!("expected SlotIndexOutOfRange for out-of-range, got {other:?}")),
        }
    }

    // -- required_u32_field: boundary values --

    #[test]
    fn required_u32_field_accepts_zero() -> Result<(), String> {
        let body = yaml_node("limit: 0")?;
        let value = required_u32_field(&body, 0, "for_each", "limit")?;
        ensure(value == 0, "u32 zero should be accepted")
    }

    #[test]
    fn required_u32_field_accepts_u32_max() -> Result<(), String> {
        let body = yaml_node("limit: 4294967295")?;
        let value = required_u32_field(&body, 0, "for_each", "limit")?;
        ensure(value == u32::MAX, "u32::MAX should be accepted")
    }

    #[test]
    fn required_u32_field_rejects_missing_field() -> Result<(), String> {
        let body = yaml_node("other: 1")?;
        match required_u32_field(&body, 0, "for_each", "limit") {
            Err(CompileError::MissingStepField { step: 0, field: "limit" }) => Ok(()),
            other => Err(format!("expected MissingStepField, got {other:?}")),
        }
    }

    #[test]
    fn required_u32_field_rejects_non_integer() -> Result<(), String> {
        let body = yaml_node("limit: abc")?;
        match required_u32_field(&body, 0, "for_each", "limit") {
            Err(CompileError::StepFieldShape { step: 0, field: "limit", .. }) => Ok(()),
            other => Err(format!("expected StepFieldShape for non-integer, got {other:?}")),
        }
    }

    #[test]
    fn required_u32_field_rejects_i64_max() -> Result<(), String> {
        let body = yaml_node("limit: 9223372036854775807")?;
        match required_u32_field(&body, 0, "for_each", "limit") {
            Err(CompileError::PrimitiveLoweringLimitExceeded { .. }) => Ok(()),
            other => Err(format!("expected PrimitiveLoweringLimitExceeded, got {other:?}")),
        }
    }

    // -- required_u16_field: boundary values --

    #[test]
    fn required_u16_field_accepts_zero() -> Result<(), String> {
        let body = yaml_node("max_attempts: 0")?;
        let value = required_u16_field(&body, 0, "repeat", "max_attempts")?;
        ensure(value == 0, "u16 zero should be accepted")
    }

    #[test]
    fn required_u16_field_accepts_u16_max() -> Result<(), String> {
        let body = yaml_node("max_attempts: 65535")?;
        let value = required_u16_field(&body, 0, "repeat", "max_attempts")?;
        ensure(value == u16::MAX, "u16::MAX should be accepted")
    }

    #[test]
    fn required_u16_field_rejects_missing() -> Result<(), String> {
        let body = yaml_node("other: 1")?;
        match required_u16_field(&body, 0, "repeat", "max_attempts") {
            Err(CompileError::MissingStepField { step: 0, field: "max_attempts" }) => Ok(()),
            other => Err(format!("expected MissingStepField, got {other:?}")),
        }
    }

    #[test]
    fn required_u16_field_rejects_non_integer() -> Result<(), String> {
        let body = yaml_node("max_attempts: true")?;
        match required_u16_field(&body, 0, "repeat", "max_attempts") {
            Err(CompileError::StepFieldShape { step: 0, field: "max_attempts", .. }) => Ok(()),
            other => Err(format!("expected StepFieldShape, got {other:?}")),
        }
    }

    #[test]
    fn required_u16_field_rejects_negative() -> Result<(), String> {
        let body = yaml_node("max_attempts: -1")?;
        match required_u16_field(&body, 0, "repeat", "max_attempts") {
            Err(CompileError::PrimitiveLoweringLimitExceeded { .. }) => Ok(()),
            other => Err(format!("expected PrimitiveLoweringLimitExceeded, got {other:?}")),
        }
    }

    // -- required_action: error paths --

    #[test]
    fn required_action_rejects_missing() -> Result<(), String> {
        let body = yaml_node("input: 0")?;
        match required_action(&body, 0, "do") {
            Err(CompileError::MissingStepField { step: 0, field: "action" }) => Ok(()),
            other => Err(format!("expected MissingStepField, got {other:?}")),
        }
    }

    #[test]
    fn required_action_rejects_non_integer() -> Result<(), String> {
        let body = yaml_node("action: hello")?;
        match required_action(&body, 0, "do") {
            Err(CompileError::StepFieldShape { step: 0, field: "action", .. }) => Ok(()),
            other => Err(format!("expected StepFieldShape for non-integer action, got {other:?}")),
        }
    }

    #[test]
    fn required_action_rejects_out_of_range() -> Result<(), String> {
        let body = yaml_node("action: 70000")?;
        match required_action(&body, 0, "do") {
            Err(CompileError::PrimitiveLoweringLimitExceeded { .. }) => Ok(()),
            other => Err(format!("expected PrimitiveLoweringLimitExceeded, got {other:?}")),
        }
    }

    #[test]
    fn required_action_rejects_negative() -> Result<(), String> {
        let body = yaml_node("action: -1")?;
        match required_action(&body, 0, "do") {
            Err(CompileError::PrimitiveLoweringLimitExceeded { .. }) => Ok(()),
            other => Err(format!("expected PrimitiveLoweringLimitExceeded for negative, got {other:?}")),
        }
    }

    // -- required_branch_targets: error paths --

    #[test]
    fn required_branch_targets_rejects_missing_field() -> Result<(), String> {
        let body = yaml_node("other: 1")?;
        match required_branch_targets(&body, 0, "branches") {
            Err(CompileError::MissingStepField { step: 0, field: "branches" }) => Ok(()),
            other => Err(format!("expected MissingStepField, got {other:?}")),
        }
    }

    #[test]
    fn required_branch_targets_rejects_non_sequence() -> Result<(), String> {
        let body = yaml_node("branches: 42")?;
        match required_branch_targets(&body, 0, "branches") {
            Err(CompileError::StepFieldShape { step: 0, field: "branches", .. }) => Ok(()),
            other => Err(format!("expected StepFieldShape for non-sequence, got {other:?}")),
        }
    }

    #[test]
    fn required_branch_targets_rejects_non_integer_element() -> Result<(), String> {
        let body = yaml_node("branches: [1, two]")?;
        match required_branch_targets(&body, 0, "branches") {
            Err(CompileError::StepFieldShape { step: 0, field: "branches", .. }) => Ok(()),
            other => Err(format!("expected StepFieldShape for non-integer element, got {other:?}")),
        }
    }

    #[test]
    fn required_branch_targets_rejects_out_of_range_element() -> Result<(), String> {
        let body = yaml_node("branches: [1, 70000]")?;
        match required_branch_targets(&body, 0, "branches") {
            Err(CompileError::BranchTargetOutOfRange { .. }) => Ok(()),
            other => Err(format!("expected BranchTargetOutOfRange, got {other:?}")),
        }
    }

    #[test]
    fn required_branch_targets_single_element() -> Result<(), String> {
        let body = yaml_node("branches: [5]")?;
        let targets = required_branch_targets(&body, 0, "branches")?;
        ensure(targets.len() == 1, "should have 1 target")?;
        ensure(targets[0].as_usize() == 5, "single target should be 5")
    }

    // -- required_branch_target: error paths --

    #[test]
    fn required_branch_target_rejects_missing_field() -> Result<(), String> {
        let body = yaml_node("other: 1")?;
        match required_branch_target(&body, 0, "on_true") {
            Err(CompileError::MissingStepField { step: 0, field: "on_true" }) => Ok(()),
            other => Err(format!("expected MissingStepField, got {other:?}")),
        }
    }

    #[test]
    fn required_branch_target_rejects_non_integer() -> Result<(), String> {
        let body = yaml_node("on_true: hello")?;
        match required_branch_target(&body, 0, "on_true") {
            Err(CompileError::StepFieldShape { step: 0, field: "on_true", .. }) => Ok(()),
            other => Err(format!("expected StepFieldShape, got {other:?}")),
        }
    }

    #[test]
    fn required_branch_target_rejects_out_of_range() -> Result<(), String> {
        let body = yaml_node("on_true: 70000")?;
        match required_branch_target(&body, 0, "on_true") {
            Err(CompileError::BranchTargetOutOfRange { .. }) => Ok(()),
            other => Err(format!("expected BranchTargetOutOfRange, got {other:?}")),
        }
    }

    #[test]
    fn required_branch_target_rejects_negative() -> Result<(), String> {
        let body = yaml_node("on_true: -5")?;
        match required_branch_target(&body, 0, "on_true") {
            Err(CompileError::BranchTargetOutOfRange { .. }) => Ok(()),
            other => Err(format!("expected BranchTargetOutOfRange for negative, got {other:?}")),
        }
    }

    // -- required_choose_condition: both variants --

    #[test]
    fn required_choose_condition_literal_true() -> Result<(), String> {
        let body = yaml_node("condition: true")?;
        match required_choose_condition(&body, 0) {
            Ok(ChooseCondition::Literal(true)) => Ok(()),
            other => Err(format!("expected Literal(true), got {other:?}")),
        }
    }

    #[test]
    fn required_choose_condition_literal_false() -> Result<(), String> {
        let body = yaml_node("condition: false")?;
        match required_choose_condition(&body, 0) {
            Ok(ChooseCondition::Literal(false)) => Ok(()),
            other => Err(format!("expected Literal(false), got {other:?}")),
        }
    }

    #[test]
    fn required_choose_condition_slot_variant() -> Result<(), String> {
        let body = yaml_node("condition: 3")?;
        match required_choose_condition(&body, 0) {
            Ok(ChooseCondition::Slot(slot)) if slot.as_u16() == 3 => Ok(()),
            other => Err(format!("expected Slot(3), got {other:?}")),
        }
    }

    #[test]
    fn required_choose_condition_rejects_missing() -> Result<(), String> {
        let body = yaml_node("other: 1")?;
        match required_choose_condition(&body, 0) {
            Err(CompileError::MissingStepField { step: 0, field: "condition" }) => Ok(()),
            other => Err(format!("expected MissingStepField, got {other:?}")),
        }
    }

    #[test]
    fn required_choose_condition_rejects_string_condition() -> Result<(), String> {
        let body = yaml_node("condition: hello")?;
        match required_choose_condition(&body, 0) {
            Err(CompileError::StepFieldShape { .. }) => Ok(()),
            other => Err(format!("expected error for string condition, got {other:?}")),
        }
    }

    // -- checked_step_offset: additional cases --

    #[test]
    fn checked_step_offset_zero_offset() -> Result<(), String> {
        let result = checked_step_offset(StepIdx::new(100), 0, "for_each", "body")?;
        ensure(result.as_usize() == 100, "zero offset should be identity")
    }

    #[test]
    fn checked_step_offset_at_boundary() -> Result<(), String> {
        let result = checked_step_offset(StepIdx::new(65534), 1, "for_each", "body")?;
        ensure(result.as_usize() == 65535, "65534 + 1 should be 65535")
    }

    // -- source_ir_start: boundary cases --

    #[test]
    fn source_ir_start_empty_slice() -> Result<(), String> {
        let starts: &[StepIdx] = &[];
        match source_ir_start(starts, 0) {
            Err(CompileError::StepIndexOutOfRange { value: 0 }) => Ok(()),
            other => Err(format!("expected StepIndexOutOfRange for empty slice, got {other:?}")),
        }
    }

    #[test]
    fn source_ir_start_first_element() -> Result<(), String> {
        let starts = [StepIdx::new(42)];
        let result = source_ir_start(&starts, 0)?;
        ensure(result.as_usize() == 42, "should return first element")
    }

    // -- mapped_branch_target --

    #[test]
    fn mapped_branch_target_resolves_through_starts() -> Result<(), String> {
        let body = yaml_node("on_true: 2")?;
        let starts = [StepIdx::new(0), StepIdx::new(5), StepIdx::new(10)];
        let result = mapped_branch_target(&body, 0, "on_true", &starts)?;
        ensure(result.as_usize() == 10, "should resolve to starts[2]")
    }

    #[test]
    fn mapped_branch_target_propagates_missing_field() -> Result<(), String> {
        let body = yaml_node("other: 1")?;
        let starts = [StepIdx::new(0)];
        match mapped_branch_target(&body, 0, "on_true", &starts) {
            Err(CompileError::MissingStepField { .. }) => Ok(()),
            other => Err(format!("expected MissingStepField, got {other:?}")),
        }
    }

    #[test]
    fn mapped_branch_target_propagates_out_of_range_start() -> Result<(), String> {
        let body = yaml_node("on_true: 5")?;
        let starts = [StepIdx::new(0)];
        match mapped_branch_target(&body, 0, "on_true", &starts) {
            Err(CompileError::StepIndexOutOfRange { .. }) => Ok(()),
            other => Err(format!("expected StepIndexOutOfRange, got {other:?}")),
        }
    }

    // -- required_next_step --

    #[test]
    fn required_next_step_returns_some_value() -> Result<(), String> {
        let result = required_next_step(Some(StepIdx::new(3)), 0)?;
        ensure(result.as_usize() == 3, "should return the step index")
    }

    #[test]
    fn required_next_step_rejects_none() -> Result<(), String> {
        match required_next_step(None, 42) {
            Err(CompileError::StepIndexOutOfRange { value: 42 }) => Ok(()),
            other => Err(format!("expected StepIndexOutOfRange with value=42, got {other:?}")),
        }
    }

    // -- reject_last_non_finish: boundary cases --

    #[test]
    fn reject_last_non_finish_single_step() -> Result<(), String> {
        match reject_last_non_finish(0, 0) {
            Err(CompileError::LastStepMustFinish) => Ok(()),
            other => Err(format!("single step should be last, got {other:?}")),
        }
    }

    #[test]
    fn reject_last_non_finish_non_last_steps_pass() -> Result<(), String> {
        reject_last_non_finish(0, 5)?;
        reject_last_non_finish(1, 5)?;
        reject_last_non_finish(4, 5)
    }

    // -- reject_unknown_primitive_fields: edge cases --

    #[test]
    fn reject_unknown_primitive_fields_rejects_non_mapping() -> Result<(), String> {
        let body = yaml_node("just_a_string")?;
        match reject_unknown_primitive_fields(&body, 0, "do", &["action"]) {
            Err(CompileError::StepFieldShape { step: 0, field: "do", .. }) => Ok(()),
            other => Err(format!("expected StepFieldShape for scalar, got {other:?}")),
        }
    }

    #[test]
    fn reject_unknown_primitive_fields_empty_allowed_list() -> Result<(), String> {
        let body = yaml_node("action: 1")?;
        match reject_unknown_primitive_fields(&body, 0, "do", &[]) {
            Err(CompileError::UnknownStepPrimitiveField { step: 0, primitive: "do", .. }) => Ok(()),
            other => Err(format!("expected UnknownStepPrimitiveField, got {other:?}")),
        }
    }

    #[test]
    fn reject_unknown_primitive_fields_empty_mapping_passes() -> Result<(), String> {
        let body = yaml_node("{}")?;
        reject_unknown_primitive_fields(&body, 0, "do", &["action"])
    }

    #[test]
    fn reject_unknown_primitive_fields_non_string_key_produces_step_shape() -> Result<(), String> {
        // Build a YAML mapping with a non-string key via a sequence trick.
        // Using explicit integer key in YAML to trigger non-string key path.
        let source = "1: value";
        let docs = saphyr::LoadableYamlNode::load_from_str(source)
            .map_err(|e| format!("yaml: {e:?}"))?;
        let body = docs.first().ok_or("empty doc")?;
        match reject_unknown_primitive_fields(body, 0, "do", &["action"]) {
            Err(CompileError::StepShape { step: 0 }) => Ok(()),
            other => Err(format!("expected StepShape for non-string key, got {other:?}")),
        }
    }

    // -- reject_non_mapping_step_body --

    #[test]
    fn reject_non_mapping_step_body_passes_for_mapping() -> Result<(), String> {
        let body = yaml_node("key: value")?;
        reject_non_mapping_step_body(&body, 0, "do", "a mapping")
    }

    #[test]
    fn reject_non_mapping_step_body_rejects_scalar() -> Result<(), String> {
        let body = yaml_node("42")?;
        match reject_non_mapping_step_body(&body, 0, "do", "a mapping") {
            Err(CompileError::StepFieldShape { step: 0, field: "do", expected: "a mapping" }) => Ok(()),
            other => Err(format!("expected StepFieldShape, got {other:?}")),
        }
    }

    #[test]
    fn reject_non_mapping_step_body_rejects_sequence() -> Result<(), String> {
        let body = yaml_node("[1, 2]")?;
        match reject_non_mapping_step_body(&body, 0, "do", "a mapping") {
            Err(CompileError::StepFieldShape { step: 0, field: "do", .. }) => Ok(()),
            other => Err(format!("expected StepFieldShape for sequence, got {other:?}")),
        }
    }

    // -- reject_unsupported_for_each_fields: non-mapping body --

    #[test]
    fn reject_unsupported_for_each_non_mapping_passes() -> Result<(), String> {
        let body = yaml_node("42")?;
        reject_unsupported_for_each_fields(&body, 0)
    }

    // -- slot_value: additional edge cases --

    #[test]
    fn slot_value_parses_large_positive_integer() -> Result<(), String> {
        let node = yaml_node("9223372036854775807")?;
        let value = slot_value(&node, 0).map_err(|e| format!("slot_value: {e:?}"))?;
        ensure(value == ConstValue::I64(i64::MAX), "i64::MAX should map to I64")
    }

    #[test]
    fn slot_value_parses_min_negative_integer() -> Result<(), String> {
        // YAML may parse -9223372036854775808 as i64::MIN or overflow
        let node = yaml_node("-9223372036854775807")?;
        let value = slot_value(&node, 0).map_err(|e| format!("slot_value: {e:?}"))?;
        ensure(
            value == ConstValue::I64(i64::MIN + 1),
            "near-min negative should map correctly",
        )
    }

    #[test]
    fn slot_value_preserves_step_index_in_error() -> Result<(), String> {
        let node = yaml_node("hello")?;
        match slot_value(&node, 42) {
            Err(CompileError::UnsupportedConstantValue { step: 42 }) => Ok(()),
            other => Err(format!("expected UnsupportedConstantValue with step=42, got {other:?}")),
        }
    }

    #[test]
    fn slot_value_parses_zero_integer() -> Result<(), String> {
        let node = yaml_node("0")?;
        let value = slot_value(&node, 0).map_err(|e| format!("slot_value: {e:?}"))?;
        ensure(value == ConstValue::I64(0), "zero should map to I64(0)")
    }

    // -- alloc_workflow_slot: integration with WorkflowBuilder --

    #[test]
    fn alloc_workflow_slot_returns_sequential_slots() -> Result<(), String> {
        use super::super::compile_step::WorkflowBuilder;
        let mut builder = WorkflowBuilder::new();
        let slot_a = alloc_workflow_slot(&mut builder)?;
        let slot_b = alloc_workflow_slot(&mut builder)?;
        let slot_c = alloc_workflow_slot(&mut builder)?;
        ensure(slot_a.as_u16() == 0, "first allocated slot should be 0")?;
        ensure(slot_b.as_u16() == 1, "second allocated slot should be 1")?;
        ensure(slot_c.as_u16() == 2, "third allocated slot should be 2")
    }
}
