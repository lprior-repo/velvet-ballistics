//! Canonical finish lowering: last-step result binding and name padding.

use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use vb_core::StepIdx;

pub(in crate::mod_compile_lowering) fn extend_step_names_for_generated(
    names: &mut Vec<Box<str>>,
    step_id: &str,
    node_count: usize,
) {
    while names.len() < node_count {
        names.push(Box::from(step_id));
    }
}

pub(in crate::mod_compile_lowering) fn lower_canonical_finish_scoped(
    index: usize,
    last: usize,
    id: StepIdx,
    result: &crate::ScalarValue,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    if index != last {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "finish",
            expected: "the last step",
        }]));
    }
    let slot = match result {
        crate::ScalarValue::String(name) if name.starts_with('$') => {
            resolve_slot_reference_scoped(name, scope).map_err(|e| CompileErrors(vec![e]))?
        }
        crate::ScalarValue::String(name) => scope.get_output(name).ok_or_else(|| {
            CompileErrors(vec![CompileError::UnknownOutputName {
                name: name.clone().into_boxed_str(),
            }])
        })?,
        _ => super::canonical_finish_slot_with_step_outputs(
            result,
            scope.outputs,
            scope.step_outputs,
        )?,
    };
    let node = lower_finish(id, slot, builder);
    builder.push_node(node);
    Ok(())
}
