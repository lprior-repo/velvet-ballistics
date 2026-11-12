use super::canonical_finish_slot;
use super::resolve_slot_reference;
use super::*;

/// Finish uses the same declaration and prior-output environment as expressions.
pub(crate) fn canonical_finish_slot_with_step_outputs(
    result: &crate::ScalarValue,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
) -> Result<SlotIdx, CompileErrors> {
    match result {
        crate::ScalarValue::String(name) if name.starts_with('$') => {
            resolve_slot_reference(name, outputs, step_outputs).map_err(|e| CompileErrors(vec![e]))
        }
        _ => canonical_finish_slot(result, outputs),
    }
}
