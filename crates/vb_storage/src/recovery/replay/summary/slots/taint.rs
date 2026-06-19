#![forbid(unsafe_code)]
//! Slot taint extraction from event data.
//!
//! Provides:
//! - `RecoveredSlotTaint` — taint with unsafety flag
//! - `recovered_slot_taint` — slot taint from event data

use crate::recovery::{RecoveryError, RecoveryResult};
use crate::slot_extra::{DecodedSlotWrittenExtra, decode_slot_written_extra};
use vb_core::{SlotIdx, SlotValue, Taint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecoveredSlotTaint {
    pub(crate) taint: Taint,
    pub(crate) unsupported: bool,
}

pub(crate) fn recovered_slot_taint(
    slot: SlotIdx,
    value: SlotValue,
    extra: &Option<Vec<u8>>,
) -> RecoveryResult<RecoveredSlotTaint> {
    match extra {
        Some(bytes) => decoded_slot_taint(slot, value, bytes),
        None => Ok(legacy_recovered_slot_taint(value)),
    }
}

fn decoded_slot_taint(
    slot: SlotIdx,
    value: SlotValue,
    bytes: &[u8],
) -> RecoveryResult<RecoveredSlotTaint> {
    match decode_slot_written_extra(bytes) {
        Ok(DecodedSlotWrittenExtra::Envelope(envelope)) => Ok(RecoveredSlotTaint {
            taint: envelope.taint,
            unsupported: false,
        }),
        Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(_)) => {
            Ok(legacy_frame_extra_recovered_slot_taint(value))
        }
        Err(_) => Err(RecoveryError::CorruptSlotTaint { slot }),
    }
}

fn legacy_recovered_slot_taint(value: SlotValue) -> RecoveredSlotTaint {
    RecoveredSlotTaint {
        taint: legacy_slot_taint(value),
        unsupported: false,
    }
}

fn legacy_frame_extra_recovered_slot_taint(value: SlotValue) -> RecoveredSlotTaint {
    RecoveredSlotTaint {
        taint: legacy_slot_taint(value),
        unsupported: true,
    }
}

fn legacy_slot_taint(value: SlotValue) -> Taint {
    match value {
        SlotValue::Bool(false) => Taint::Clean,
        SlotValue::Bool(true) | SlotValue::Null => Taint::DerivedFromSecret,
        _ => Taint::Secret,
    }
}
