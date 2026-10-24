use super::payload::Body;
use super::{ContinuationAuthority, RunContinuationError};
use vb_core::value_store::ValueStoreCheckpointV1;

pub(super) fn validate(
    body: &Body,
    authority: &ContinuationAuthority,
) -> Result<(), RunContinuationError> {
    let original = authority.original.checkpoint();
    if !prefix(original, &body.value_store) {
        return Err(RunContinuationError::OwnedPrefix);
    }
    for (slot, value, taint) in authority.original.roots() {
        let index = usize::from(slot.get());
        if body.frame.slots.get(index) != Some(&Some(*value))
            || body.frame.taint.get(index) != Some(taint)
        {
            return Err(RunContinuationError::OwnedRoot);
        }
    }
    Ok(())
}

fn prefix(original: &ValueStoreCheckpointV1, current: &ValueStoreCheckpointV1) -> bool {
    current.max_arena_entries == original.max_arena_entries
        && current.symbols.get(..original.symbols.len()) == Some(original.symbols.as_slice())
        && current.lists.get(..original.lists.len()) == Some(original.lists.as_slice())
        && current.list_taints.get(..original.list_taints.len())
            == Some(original.list_taints.as_slice())
        && current.objects.get(..original.objects.len()) == Some(original.objects.as_slice())
        && current.blobs.get(..original.blobs.len()) == Some(original.blobs.as_slice())
}
