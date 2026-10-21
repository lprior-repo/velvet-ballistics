use crate::JournalError;
use vb_core::{SlotIdx, SlotValue, WorkflowDigest};

/// Historical receipts did not bind input values. New receipts always bind a digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RunInputBinding {
    /// Explicit compatibility state for historical admission records only.
    LegacyUnbound,
    /// BLAKE3 of canonical slot-sorted postcard typed input pairs.
    Digest(WorkflowDigest),
}

/// Computes the order-independent identity of a unique, bounded typed input mapping.
pub fn compute_input_digest(
    inputs: &[(SlotIdx, SlotValue)],
) -> Result<WorkflowDigest, JournalError> {
    if inputs.len() > usize::from(u16::MAX) {
        return Err(JournalError::InputSchemaMismatch);
    }
    let mut canonical = Vec::new();
    canonical
        .try_reserve_exact(inputs.len())
        .map_err(JournalError::AdmissionAllocationFailed)?;
    canonical.extend_from_slice(inputs);
    canonical.sort_unstable_by_key(|(slot, _)| slot.get());
    if canonical.windows(2).any(|pair| {
        pair.first()
            .zip(pair.last())
            .is_some_and(|(a, b)| a.0 == b.0)
    }) {
        return Err(JournalError::InputSchemaMismatch);
    }
    let bytes = postcard::to_allocvec(&canonical).map_err(JournalError::PostcardEncodeFailed)?;
    Ok(WorkflowDigest::from_bytes(blake3::hash(&bytes).into()))
}
