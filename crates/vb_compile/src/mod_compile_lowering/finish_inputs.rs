use super::canonical_finish_slot;
use super::resolve_slot_reference;
use super::*;

/// Resolve finish slot using both output names and step IDs.
pub(crate) fn canonical_finish_slot_with_step_outputs(
    result: &crate::ScalarValue,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
) -> Result<SlotIdx, CompileErrors> {
    match result {
        crate::ScalarValue::String(name)
            if name.starts_with('$') && !name.starts_with("$input.") =>
        {
            resolve_slot_reference(name, outputs, step_outputs).map_err(|e| CompileErrors(vec![e]))
        }
        crate::ScalarValue::String(name) if name.starts_with("$input.") => {
            resolve_finish_input_slot(name, outputs)
        }
        _ => canonical_finish_slot(result, outputs),
    }
}

fn resolve_finish_input_slot(
    name: &str,
    outputs: &HashMap<String, SlotIdx>,
) -> Result<SlotIdx, CompileErrors> {
    outputs.get(name).copied().ok_or_else(|| {
        CompileErrors(vec![CompileError::UnknownOutputName {
            name: name.to_owned().into(),
        }])
    })
}

pub(crate) fn prepare_finish_input_slots(
    source: &crate::WorkflowSource,
    result: &crate::ScalarValue,
    outputs: &mut HashMap<String, SlotIdx>,
    step_outputs: &mut HashMap<String, SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    if !matches!(result, crate::ScalarValue::String(name) if name.starts_with("$input.")) {
        let internal = canonical_finish_slot_with_step_outputs(result, outputs, step_outputs)?;
        builder.record_slot(internal);
    }

    allocate_input_slots(source.inputs(), builder).map_err(|e| CompileErrors(vec![e]))?;

    for (input, port) in source.inputs().iter().zip(builder.input_slots.iter()) {
        let name = format!("$input.{}", input.key);
        if outputs.contains_key(&name) {
            return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
                name: name.to_owned().into(),
            }]));
        }
        outputs.insert(name, port.slot);
    }

    Ok(())
}
