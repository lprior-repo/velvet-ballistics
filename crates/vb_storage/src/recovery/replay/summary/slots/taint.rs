#![forbid(unsafe_code)]
//! Slot taint extraction from event data.
//!
//! Provides:
//! - `RecoveredSlotTaint` — taint with unsafety flag
//! - `recovered_slot_taint` — slot taint from event data

use crate::events::SlotWriteExtra;
use crate::recovery::RecoveryResult;
use vb_core::{SlotIdx, SlotValue, Taint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecoveredSlotTaint {
    pub(crate) taint: Taint,
    pub(crate) unsupported: bool,
}

pub(crate) fn recovered_slot_taint(
    _slot: SlotIdx,
    value: SlotValue,
    extra: Option<&SlotWriteExtra>,
) -> RecoveryResult<RecoveredSlotTaint> {
    match extra {
        Some(SlotWriteExtra::Versioned(envelope)) => Ok(RecoveredSlotTaint {
            taint: envelope.taint,
            unsupported: false,
        }),
        Some(SlotWriteExtra::Legacy(_)) => Ok(legacy_frame_extra_recovered_slot_taint(value)),
        None => Ok(legacy_recovered_slot_taint(value)),
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

fn legacy_slot_taint(_value: SlotValue) -> Taint {
    Taint::Secret
}
