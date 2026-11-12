use super::CarrierError;
use crate::EventSeq;
use vb_core::RunId;

/// Immutable explicit scalar identity; wire validity is not runtime authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContinuationCarrierHeader {
    run: RunId,
    own_seq: EventSeq,
    attempt: u16,
    prior_seq: EventSeq,
    digest: [u8; 32],
}

impl ContinuationCarrierHeader {
    /// Fixes both versions to one and checks all scalar invariants.
    pub fn new(
        run: RunId,
        own_seq: EventSeq,
        attempt: u16,
        prior_seq: EventSeq,
        component_digest: [u8; 32],
    ) -> Result<Self, CarrierError> {
        if run.get() == 0 || attempt == 0 || own_seq.get() == u64::MAX {
            return Err(CarrierError::Identity("reserved run, sequence or attempt"));
        }
        if prior_seq.get().checked_add(1) != Some(own_seq.get()) {
            return Err(CarrierError::Identity("prior sequence edge"));
        }
        Ok(Self {
            run,
            own_seq,
            attempt,
            prior_seq,
            digest: component_digest,
        })
    }

    /// Actual run identifier, unchanged.
    pub const fn run(self) -> RunId {
        self.run
    }

    /// This carrier's actual sequence.
    pub const fn own_seq(self) -> EventSeq {
        self.own_seq
    }

    /// Encoded actual attempt; never normalized.
    pub const fn actual_attempt(self) -> u16 {
        self.attempt
    }

    /// Claimed prior sequence; independently verified durable provenance is separate.
    pub const fn prior_seq(self) -> EventSeq {
        self.prior_seq
    }

    /// Complete component content digest, including all nested state bindings.
    pub const fn component_digest(self) -> [u8; 32] {
        self.digest
    }
}
