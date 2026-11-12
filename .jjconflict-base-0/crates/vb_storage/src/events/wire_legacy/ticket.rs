#![forbid(unsafe_code)]

//! Action ticket in the shape written before the fencing token existed.

use serde::{Deserialize, Serialize};
use vb_core::{ActionId, ActionTicket, RunId, StepIdx};

/// Action ticket as written before the fencing token existed.
///
/// A record in this shape decodes with [`ActionTicket::fencing_token`] zero.
/// Fencing reads zero as "unknown", not "stale": the durable ticket identity
/// still rejects wrong attempts, and refusing every pre-token completion would
/// strand runs across an upgrade.
#[derive(Clone, Copy, Serialize, Deserialize)]
pub(crate) struct LegacyActionTicket {
    pub(crate) run: RunId,
    pub(crate) step: StepIdx,
    pub(crate) seq: vb_core::SeqNo,
    pub(crate) action: ActionId,
    pub(crate) run_attempt: u16,
    pub(crate) attempt: u16,
    pub(crate) idempotency_key: u128,
    pub(crate) capacity: u16,
}

impl LegacyActionTicket {
    pub(super) const fn into_current(self) -> ActionTicket {
        ActionTicket {
            run: self.run,
            step: self.step,
            seq: self.seq,
            action: self.action,
            run_attempt: self.run_attempt,
            attempt: self.attempt,
            idempotency_key: self.idempotency_key,
            capacity: self.capacity,
            fencing_token: 0,
        }
    }
}
