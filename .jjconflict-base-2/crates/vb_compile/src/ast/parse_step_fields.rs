#![forbid(unsafe_code)]
use super::parse_values::parse_value;
use super::types::{AstMapEntry, AstValue};
use crate::CompileError;
use saphyr::Yaml;
use vb_core::{SlotIdx, StepIdx};

pub(super) fn parse_value_fields(
    body: &Yaml<'_>,
) -> Result<Vec<AstMapEntry<AstValue>>, CompileError> {
    let mapping = body.as_mapping().ok_or(CompileError::StepFieldShape {
        step: 0,
        field: "save",
        expected: "an object",
    })?;
    let mut fields = Vec::with_capacity(mapping.len());
    for (key, value) in mapping {
        fields.push(value_field(key, value)?);
    }
    Ok(fields)
}

pub(super) fn value_field(
    key: &Yaml<'_>,
    value: &Yaml<'_>,
) -> Result<AstMapEntry<AstValue>, CompileError> {
    let name = key.as_str().ok_or_else(crate::non_string_key_error)?;
    Ok(AstMapEntry {
        name: name.into(),
        value: parse_value(value)?,
        mark: None,
    })
}

pub(super) fn step_str<'a>(
    step: &'a Yaml<'a>,
    index: usize,
    field: &'static str,
) -> Result<&'a str, CompileError> {
    step.as_mapping_get(field)
        .ok_or(CompileError::MissingStepField { step: index, field })?
        .as_str()
        .ok_or(CompileError::StepFieldShape {
            step: index,
            field,
            expected: "a string",
        })
}

pub(super) fn step_field<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    field: &'static str,
) -> Result<&'a Yaml<'a>, CompileError> {
    body.as_mapping_get(field)
        .ok_or(CompileError::MissingStepField { step, field })
}

pub(super) fn parse_step_idx(node: &Yaml<'_>) -> Result<StepIdx, CompileError> {
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step: 0,
        field: "branch target",
        expected: "an integer step index",
    })?;
    let raw = u16::try_from(value).map_err(|_| CompileError::BranchTargetOutOfRange { value })?;
    Ok(StepIdx::new(raw))
}

pub(super) fn parse_slot_idx(
    node: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<SlotIdx, CompileError> {
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "an integer slot index",
    })?;
    let raw = u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
    Ok(SlotIdx::new(raw))
}

pub(super) fn parse_step_idx_sequence(
    node: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<Vec<StepIdx>, CompileError> {
    let sequence = node.as_sequence().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "a sequence of integer step indexes",
    })?;
    let mut targets = Vec::with_capacity(sequence.len());
    for item in sequence {
        targets.push(parse_step_idx(item)?);
    }
    Ok(targets)
}

pub(super) fn parse_u32_field(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<u32, CompileError> {
    let value = step_field(body, step, field)?.as_integer().ok_or({
        CompileError::StepFieldShape {
            step,
            field,
            expected: "a non-negative u32 integer",
        }
    })?;
    u32::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive: field,
        field,
        value: integer_error_value(value),
        limit: usize::try_from(u32::MAX).map_or(usize::MAX, |limit| limit),
    })
}

pub(super) fn parse_u16_field(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<u16, CompileError> {
    let value = step_field(body, step, field)?.as_integer().ok_or({
        CompileError::StepFieldShape {
            step,
            field,
            expected: "a non-negative u16 integer",
        }
    })?;
    u16::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive: field,
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

pub(super) fn parse_action_idx(
    node: &Yaml<'_>,
    step: usize,
) -> Result<vb_core::ActionId, CompileError> {
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field: "action",
        expected: "an integer action id",
    })?;
    let raw = u16::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive: "run",
        field: "action",
        value: integer_error_value(value),
        limit: usize::from(u16::MAX),
    })?;
    Ok(vb_core::ActionId::new(raw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CompileError;
    use saphyr::LoadableYamlNode;

    fn parse_yaml(s: &str) -> Yaml<'static> {
        Yaml::load_from_str(s)
            .expect("yaml parse")
            .into_iter()
            .next()
            .expect("single document")
    }

    #[test]
    fn parse_u16_field_accepts_boundary_values() {
        let zero_node = parse_yaml("max_attempts: 0");
        match parse_u16_field(&zero_node, 0, "max_attempts") {
            Ok(0) => {}
            v => panic!("expected Ok(0), got {v:?}"),
        }

        let max_node = parse_yaml("max_attempts: 65535");
        match parse_u16_field(&max_node, 0, "max_attempts") {
            Ok(u16::MAX) => {}
            v => panic!("expected Ok(u16::MAX), got {v:?}"),
        }
    }

    #[test]
    fn parse_u16_field_rejects_over_max() {
        let over_node = parse_yaml("max_attempts: 65536");
        match parse_u16_field(&over_node, 0, "max_attempts") {
            Err(CompileError::PrimitiveLoweringLimitExceeded { .. }) => {}
            v => panic!("expected PrimitiveLoweringLimitExceeded, got {v:?}"),
        }
    }

    #[test]
    fn parse_u16_field_rejects_negative_integer() {
        let neg_node = parse_yaml("max_attempts: -1");
        match parse_u16_field(&neg_node, 0, "max_attempts") {
            Err(CompileError::PrimitiveLoweringLimitExceeded { .. }) => {}
            v => panic!("expected PrimitiveLoweringLimitExceeded, got {v:?}"),
        }
    }

    #[test]
    fn parse_u16_field_rejects_non_integer() {
        let str_node = parse_yaml("max_attempts: hello");
        match parse_u16_field(&str_node, 0, "max_attempts") {
            Err(CompileError::StepFieldShape { .. }) => {}
            v => panic!("expected StepFieldShape, got {v:?}"),
        }
    }

    #[test]
    fn integer_error_value_returns_value_when_in_range() {
        assert_eq!(integer_error_value(42), 42);
        assert_eq!(integer_error_value(0), 0);
        // i64::MAX fits in usize on 64-bit, returns i64::MAX
        assert_eq!(integer_error_value(i64::MAX), i64::MAX as usize);
    }

    #[test]
    fn integer_error_value_returns_max_for_out_of_range() {
        assert_eq!(integer_error_value(-1), usize::MAX);
        assert_eq!(integer_error_value(i64::MIN), usize::MAX);
    }
}
