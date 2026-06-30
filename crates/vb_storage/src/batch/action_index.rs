#![forbid(unsafe_code)]
//! Atomic staging of pending action index maintenance for journal appends.
//!
//! vb-3wn7x: the runtime journal path drives the lifecycle of pending
//! actions in three groups:
//!
//! 1. `ActionScheduled` / `ActionScheduledTicket` — emits the pending
//!    action to the external boundary; a marker must be inserted into
//!    the `index_action` keyspace so recovery, inspection, and lookup
//!    traffic see the action as still in flight.
//! 2. `ActionCompletedEvent` / `ActionCompletedEnvelope` — terminal
//!    success; the marker must be removed.
//! 3. `ActionFailedEvent` / `ActionAbandoned` — terminal failure or
//!    cancellation-driven abandonment; the marker must be removed.
//!
//! All other event kinds (runs, steps, waits, asks, slot writes, …) leave
//! the index untouched.
//!
//! [`stage_pending_action_index_op`] is the single staging entry point:
//! it stages either a value insert or a tombstone on the supplied
//! `fjall::OwnedWriteBatch`. Callers MUST commit the same batch that
//! holds the journal event so index and journal writes succeed or fail
//! together; partial writes would leave the index inconsistent with the
//! event log.

use crate::{
    error::JournalError, events::JournalEvent, journal::FjallJournal, keys::index_action_key,
};
use vb_core::{ActionId, RunId, StepIdx};

/// Discriminated pending-action-index mutation.
///
/// Internal-only: the variant layout is purely a refactoring of the
/// match arms in [`stage_pending_action_index_op`] so the encode paths
/// are not duplicated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingActionIndexOp {
    /// Insert an empty marker at `(action, run, step)`.
    Insert {
        action: ActionId,
        run: RunId,
        step: StepIdx,
    },
    /// Tombstone the marker at `(action, run, step)`.
    Remove {
        action: ActionId,
        run: RunId,
        step: StepIdx,
    },
}

/// Classifies a journal event into the pending-action-index mutation it
/// implies (if any). Returns `None` for events that leave the index
/// untouched.
fn pending_action_index_op_for(event: &JournalEvent) -> Option<PendingActionIndexOp> {
    match event {
        // Insert-side — pending actions enter the index.
        JournalEvent::ActionScheduled {
            action, run, step, ..
        } => Some(PendingActionIndexOp::Insert {
            action: *action,
            run: *run,
            step: *step,
        }),
        JournalEvent::ActionScheduledTicket { ticket, .. } => Some(PendingActionIndexOp::Insert {
            action: ticket.action,
            run: ticket.run,
            step: ticket.step,
        }),
        // Remove-side — terminal actions leave the index.
        JournalEvent::ActionCompletedEvent {
            action, run, step, ..
        }
        | JournalEvent::ActionFailedEvent {
            action, run, step, ..
        } => Some(PendingActionIndexOp::Remove {
            action: *action,
            run: *run,
            step: *step,
        }),
        JournalEvent::ActionCompletedEnvelope { ticket, .. }
        | JournalEvent::ActionAbandoned { ticket, .. } => Some(PendingActionIndexOp::Remove {
            action: ticket.action,
            run: ticket.run,
            step: ticket.step,
        }),
        // Everything else is a no-op for the index.
        _ => None,
    }
}

impl FjallJournal {
    /// Stages the pending-action-index mutation implied by `event` onto
    /// the supplied `fjall::OwnedWriteBatch`.
    ///
    /// Callers MUST commit the same batch that stages the corresponding
    /// `JournalEvent` write so the index update is atomic with the event.
    /// Events that imply no index change are silently accepted (the
    /// helper returns `Ok(())` without staging any batch operation).
    ///
    /// # Errors
    ///
    /// Returns `JournalError::KeyCapacity` if the encoded
    /// `index_action` key would overflow the 13-byte fixed-length
    /// contract.
    pub(crate) fn stage_pending_action_index_op(
        &self,
        batch: &mut fjall::OwnedWriteBatch,
        event: &JournalEvent,
    ) -> Result<(), JournalError> {
        let Some(op) = pending_action_index_op_for(event) else {
            return Ok(());
        };
        match op {
            PendingActionIndexOp::Insert { action, run, step } => {
                let key = index_action_key(action, run, step)?;
                batch.insert(&self.index_action, key, Vec::<u8>::new());
                Ok(())
            }
            PendingActionIndexOp::Remove { action, run, step } => {
                let key = index_action_key(action, run, step)?;
                batch.remove(&self.index_action, key);
                Ok(())
            }
        }
    }
}
