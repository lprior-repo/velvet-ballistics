//! Canonical finish lowering: last-step result binding and name padding.

use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use std::collections::HashMap;
use vb_core::{SlotIdx, StepIdx};

pub(in crate::mod_compile_lowering) fn extend_step_names_for_generated(
    names: &mut Vec<Box<str>>,
    step_id: &str,
    node_count: usize,
) {
    while names.len() < node_count {
        names.push(Box::from(step_id));
    }
}

pub(in crate::mod_compile_lowering) fn lower_canonical_finish(
    index: usize,
    last: usize,
    id: StepIdx,
    result: &crate::ScalarValue,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    if index != last {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "finish",
            expected: "the last step",
        }]));
    }
    let slot = super::canonical_finish_slot_with_step_outputs(result, outputs, step_outputs)?;
    let node = lower_finish(id, slot, builder);
    builder.push_node(node);
    Ok(())
}
