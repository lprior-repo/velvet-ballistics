#![allow(unused_imports)]

use crate::mod_compile_errors::{CompileError, CompileErrors};
use std::collections::HashMap;
use vb_core::{SlotIdx, StepIdx};

pub(in crate::mod_compile_lowering) fn parse_i64_field(
    value: &str,
    step: usize,
    field: &'static str,
) -> Result<i64, CompileErrors> {
    value.parse::<i64>().map_err(|_| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step,
            field,
            expected: "integer string",
        }])
    })
}

pub(in crate::mod_compile_lowering) fn slot_from_text(
    text: &str,
    step: usize,
    field: &'static str,
) -> Result<SlotIdx, CompileErrors> {
    if text.is_empty() {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step,
            field,
            expected: "non-empty primitive field",
        }]));
    }
    let value = text.parse::<i64>().map_err(|_| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step,
            field,
            expected: "integer string",
        }])
    })?;
    let raw = u16::try_from(value)
        .map_err(|_| CompileErrors(vec![CompileError::SlotIndexOutOfRange { value }]))?;
    Ok(SlotIdx::new(raw))
}

pub(in crate::mod_compile_lowering) fn optional_slot_from_text(
    text: Option<&str>,
    step: usize,
    field: &'static str,
) -> Result<Option<SlotIdx>, CompileErrors> {
    match text {
        Some(value) => slot_from_text(value, step, field).map(Some),
        None => Ok(None),
    }
}

pub(in crate::mod_compile_lowering) trait StepIdxSlotExt {
    fn to_slot(self) -> SlotIdx;
}

impl StepIdxSlotExt for StepIdx {
    fn to_slot(self) -> SlotIdx {
        SlotIdx::new(self.get())
    }
}

pub(in crate::mod_compile_lowering) fn canonical_finish_slot(
    result: &crate::ScalarValue,
    outputs: &HashMap<String, SlotIdx>,
) -> Result<SlotIdx, CompileErrors> {
    match result {
        crate::ScalarValue::String(name) => {
            outputs.get(name.as_str()).copied().ok_or_else(|| {
                CompileErrors(vec![CompileError::UnknownOutputName {
                    name: name.clone().into_boxed_str(),
                }])
            })
        }
        crate::ScalarValue::Integer(value) => {
            let raw = u16::try_from(*value).map_err(|_| {
                CompileErrors(vec![CompileError::SlotIndexOutOfRange { value: *value }])
            })?;
            Ok(SlotIdx::new(raw))
        }
        _ => Err(CompileErrors(vec![
            CompileError::UnsupportedConstantValue { step: 0 },
        ])),
    }
}
