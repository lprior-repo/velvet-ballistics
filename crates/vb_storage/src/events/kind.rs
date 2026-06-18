#![forbid(unsafe_code)]
//! MRWE5 proof-seam kind classification for journal event payloads.

use super::variant::JournalEvent;
use crate::RecordKind;
use crate::mrwe5_contract::{Mrwe5PayloadClass, mrwe5_canonical_kind_id};

/// Verus-friendly class for the two MRWE5 payloads whose record kinds must stay
/// separated. `Other` deliberately carries no compatibility privilege.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum JournalEventKindClass {
    /// `JournalEvent::StepSucceeded` payloads use `RecordKind::StepSucceeded`.
    StepSucceeded = 1,
    /// `JournalEvent::SlotWrittenEvent` payloads use `RecordKind::SlotWritten`.
    SlotWrittenEvent = 2,
    /// Any journal payload outside the MRWE5 separation pair.
    Other = 3,
}

impl JournalEventKindClass {
    /// Canonical record kind for the named MRWE5 payload classes.
    #[must_use]
    pub const fn canonical_record_kind(self) -> Option<RecordKind> {
        match self {
            Self::StepSucceeded => Some(RecordKind::StepSucceeded),
            Self::SlotWrittenEvent => Some(RecordKind::SlotWritten),
            Self::Other => None,
        }
    }

    /// Canonical record-kind id for the named MRWE5 payload classes.
    #[must_use]
    pub const fn canonical_record_kind_id(self) -> Option<u16> {
        mrwe5_canonical_kind_id(self.mrwe5_payload_class())
    }

    /// Primitive production-bound class used by the shared MRWE5 contract kernel.
    #[must_use]
    pub const fn mrwe5_payload_class(self) -> Mrwe5PayloadClass {
        match self {
            Self::StepSucceeded => Mrwe5PayloadClass::StepSucceeded,
            Self::SlotWrittenEvent => Mrwe5PayloadClass::SlotWrittenEvent,
            Self::Other => Mrwe5PayloadClass::Other,
        }
    }
}

impl JournalEvent {
    /// MRWE5 proof seam exposing the payload class used for kind separation.
    #[must_use]
    pub const fn kind_class(&self) -> JournalEventKindClass {
        match self {
            Self::StepSucceeded { .. } => JournalEventKindClass::StepSucceeded,
            Self::SlotWrittenEvent { .. } => JournalEventKindClass::SlotWrittenEvent,
            _ => JournalEventKindClass::Other,
        }
    }
}
