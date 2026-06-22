#![forbid(unsafe_code)]
//! Slot taint extraction from event data.
//!
//! Provides:
//! - `RecoveredSlotTaint` — taint with unsafety flag
//! - `recovered_slot_taint` — slot taint from event data
//!
//! Fail-closed envelope classification (vb-7ol6y):
//! - [`SlotWriteExtra::Versioned`] → use envelope taint directly.
//! - [`SlotWriteExtra::Legacy`] whose bytes begin with
//!   [`SLOT_WRITTEN_EXTRA_PREFIX`] but cannot decode as a
//!   `SlotWrittenExtraEnvelope` →
//!   [`RecoveryError::CorruptSlotTaint { slot }`]. Corrupt taint metadata
//!   must never be silently rewritten to a default.
//! - [`SlotWriteExtra::Legacy`] whose bytes are real legacy frame-extra
//!   payloads (no versioned prefix) → [`Taint::Clean`]. Legacy runtime
//!   used this slot for collect pagination state and other non-taint
//!   payloads; classifying those bytes as `Taint::Secret` would be a
//!   false positive.
//! - `None` extra → legacy default [`Taint::Secret`] (no envelope means
//!   the safe choice is over-classification).

use crate::constants::MAX_FRAME_EXTRA_BYTES;
use crate::events::SlotWriteExtra;
use crate::recovery::{RecoveryError, RecoveryResult};
use crate::slot_extra::{
    DecodedSlotWrittenExtra, SLOT_WRITTEN_EXTRA_PREFIX, decode_slot_written_extra,
};
use vb_core::{SlotIdx, SlotValue, Taint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecoveredSlotTaint {
    pub(crate) taint: Taint,
    pub(crate) unsupported: bool,
}

pub(crate) fn recovered_slot_taint(
    slot: SlotIdx,
    value: SlotValue,
    extra: Option<&SlotWriteExtra>,
) -> RecoveryResult<RecoveredSlotTaint> {
    match extra {
        Some(SlotWriteExtra::Versioned(envelope)) => Ok(RecoveredSlotTaint {
            taint: envelope.taint,
            unsupported: false,
        }),
        Some(SlotWriteExtra::Legacy(bytes)) => legacy_or_corrupt_taint(slot, bytes),
        None => Ok(legacy_recovered_slot_taint(value)),
    }
}

/// Classify a [`SlotWriteExtra::Legacy`] payload.
///
/// Returns the versioned envelope taint when the bytes are a valid
/// `SlotWrittenExtraEnvelope`, [`RecoveryError::CorruptSlotTaint`]
/// when the bytes carry the versioned prefix but cannot decode, and
/// [`Taint::Clean`] for plain legacy frame-extra payloads that were
/// never taint metadata.
fn legacy_or_corrupt_taint(slot: SlotIdx, bytes: &[u8]) -> RecoveryResult<RecoveredSlotTaint> {
    if bytes.starts_with(SLOT_WRITTEN_EXTRA_PREFIX) {
        let payload_len = bytes.len().saturating_sub(SLOT_WRITTEN_EXTRA_PREFIX.len());
        if payload_len > MAX_FRAME_EXTRA_BYTES {
            return Err(RecoveryError::CorruptSlotTaint { slot });
        }
        match decode_slot_written_extra(bytes) {
            Ok(DecodedSlotWrittenExtra::Envelope(envelope)) => Ok(RecoveredSlotTaint {
                taint: envelope.taint,
                unsupported: false,
            }),
            // strip_prefix matched but decode produced LegacyFrameExtra, which
            // is only reachable when the prefix is the entire payload.
            Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(_))
            | Err(crate::slot_extra::SlotWrittenExtraError::DecodeFailed)
            | Err(crate::slot_extra::SlotWrittenExtraError::Oversized { .. }) => {
                Err(RecoveryError::CorruptSlotTaint { slot })
            }
            Err(crate::slot_extra::SlotWrittenExtraError::EncodeFailed)
            | Err(crate::slot_extra::SlotWrittenExtraError::AllocationFailed) => {
                Err(RecoveryError::CorruptSlotTaint { slot })
            }
        }
    } else {
        // Legacy runtime used SlotWrittenEvent.extra for collect
        // pagination state and other non-taint payloads. Without the
        // versioned prefix this is not taint metadata; classifying it
        // as Secret would be a false positive (vb-7ol6y / Bug 2).
        Ok(RecoveredSlotTaint {
            taint: Taint::Clean,
            unsupported: false,
        })
    }
}

fn legacy_recovered_slot_taint(value: SlotValue) -> RecoveredSlotTaint {
    RecoveredSlotTaint {
        taint: legacy_slot_taint(value),
        unsupported: false,
    }
}

fn legacy_slot_taint(_value: SlotValue) -> Taint {
    Taint::Secret
}
