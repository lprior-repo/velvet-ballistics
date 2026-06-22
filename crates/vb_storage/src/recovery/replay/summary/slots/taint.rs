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
//! - `None` extra → legacy taint classification by value type. Bool(false)
//!   maps to `Taint::Clean` (false predicates do not leak secrets), Bool(true)
//!   and Null map to `Taint::DerivedFromSecret` (positive / absence predicates
//!   can derive from secrets), and I64/F64/Symbol/Object/List values map to
//!   `Taint::Secret` because they carry the data itself.

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

/// Derives the taint for a legacy slot write (no `SlotWriteExtra` envelope).
///
/// The mapping follows the qi37 contract: the taint reflects how much secret
/// information a value could leak. Bool(false) is the only clean case because
/// a "no" predicate does not reveal secret bits. Bool(true) and Null are
/// treated as derived from a secret because a "yes" predicate and an absence
/// predicate can both depend on secret inputs. I64/F64/Symbol values are
/// classified as Secret because they carry the data itself.
fn legacy_slot_taint(value: SlotValue) -> Taint {
    match value {
        SlotValue::Bool(false) => Taint::Clean,
        SlotValue::Bool(true) | SlotValue::Null => Taint::DerivedFromSecret,
        SlotValue::I64(_)
        | SlotValue::F64(_)
        | SlotValue::Symbol(_)
        | SlotValue::Object(_)
        | SlotValue::List(_)
        | SlotValue::Blob(_) => Taint::Secret,
        // SlotValue is #[non_exhaustive]; the conservative default for any
        // future variant is to over-classify as Secret (qi37 / vb-7ol6y
        // contract: prefer false positives to silent under-classification).
        _ => Taint::Secret,
    }
}
