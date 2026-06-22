#![forbid(unsafe_code)]

//! Evidence collection types for the deterministic drive loop.
//!
//! Exports `EvidenceEvent` and `EvidenceCollector` for tracking step execution
//! events during `drive_deterministic_full`.

use vb_core::errors::EngineError;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};

use crate::primitives::collect::CollectPaginationState;

const REQUIRED_COLLECT_SLOT_EXTRA: &str = "collect SlotWritten extra";

/// Evidence event emitted by the deterministic drive loop for each step.
///
/// These events are collected during `drive_deterministic_full` and drained
/// by the shard to emit to the journal and trace ring. This satisfies
/// the Phase 40/44 evidence chain requirement that every deterministic step
/// emits `StepStarted` before `SlotWritten`, followed by `StepSucceeded`.
///
/// RS-004: `StepSucceeded` carries `attempt: u16` so the durable journal
/// records the per-step attempt counter consistently with `ActionFailed`.
/// The engine emits `attempt: 1` for the deterministic loop because
/// engine-level retries are tracked by the shard in `state.action_attempts`
/// rather than re-driven inside the engine. The shard's flush step
/// overrides this value with the actual live attempt before the journal
/// append so the durable record matches the same counter used by the
/// `ActionFailed` journal path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EvidenceEvent {
    /// Step began execution.
    StepStarted {
        /// Step index.
        step: StepIdx,
    },
    /// Step completed and optionally wrote an output slot.
    StepSucceeded {
        /// Step index.
        step: StepIdx,
        /// Output slot written, if any (Nop/Jump have no output).
        output: Option<SlotIdx>,
        /// Live per-step attempt counter at emission time. The engine
        /// emits `1`; the shard refines this from `state.action_attempts`
        /// at flush time before journaling.
        attempt: u16,
    },
    /// A slot was written during step execution.
    SlotWritten {
        /// Slot index.
        slot: SlotIdx,
        /// Value written to the slot.
        value: SlotValue,
        /// Taint written to the slot.
        taint: Taint,
        /// Optional frame extra data captured with the slot write.
        extra: Option<CollectPaginationState>,
    },
}

/// Default maximum number of evidence events before the collector drops
/// new events. Each step emits up to 3 events (Started + SlotWritten +
/// Succeeded), so 3 * step_budget provides a safe upper bound.
const DEFAULT_EVIDENCE_CAPACITY: usize = 3 * 1024;

/// Bounded collector for evidence events produced during a drive loop.
///
/// Collected and drained once per drive loop iteration by the shard
/// to emit StepStarted/StepSucceeded/SlotWritten events to the journal.
/// The collector enforces a capacity limit to prevent unbounded memory
/// growth from malicious or buggy workflows. When at capacity, new events
/// are silently dropped (the evidence chain becomes incomplete but the
/// system remains memory-safe).
#[derive(Debug, Clone)]
pub struct EvidenceCollector {
    events: Vec<EvidenceEvent>,
    capacity: usize,
    dropped: usize,
}

impl EvidenceCollector {
    /// Creates a new empty collector with a default capacity bound.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::with_capacity(DEFAULT_EVIDENCE_CAPACITY),
            capacity: DEFAULT_EVIDENCE_CAPACITY,
            dropped: 0,
        }
    }

    /// Creates a new collector with a specific capacity bound.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Vec::with_capacity(capacity),
            capacity,
            dropped: 0,
        }
    }

    /// Records a StepStarted event.
    /// Silently drops the event if the collector is at capacity.
    pub fn push_step_started(&mut self, step: StepIdx) {
        if self.events.len() < self.capacity {
            self.events.push(EvidenceEvent::StepStarted { step });
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
    }

    /// Records a StepSucceeded event.
    /// Silently drops the event if the collector is at capacity.
    ///
    /// `attempt` is the live per-step attempt counter. The deterministic
    /// drive loop passes `1` because engine-level retries do not exist;
    /// the shard's flush path overrides this with `state.action_attempts`
    /// before persisting to the journal so the durable record reflects
    /// the actual attempt count (RS-004).
    pub fn push_step_succeeded(&mut self, step: StepIdx, output: Option<SlotIdx>, attempt: u16) {
        if self.events.len() < self.capacity {
            self.events.push(EvidenceEvent::StepSucceeded {
                step,
                output,
                attempt,
            });
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
    }

    /// Records a SlotWritten event.
    /// Silently drops the event if the collector is at capacity.
    pub fn push_slot_written(&mut self, slot: SlotIdx, value: SlotValue) {
        self.push_slot_written_with_taint(slot, value, Taint::Clean);
    }

    /// Records a SlotWritten event with explicit taint.
    /// Silently drops the event if the collector is at capacity.
    pub fn push_slot_written_with_taint(&mut self, slot: SlotIdx, value: SlotValue, taint: Taint) {
        if self.events.len() < self.capacity {
            self.events.push(EvidenceEvent::SlotWritten {
                slot,
                value,
                taint,
                extra: None,
            });
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
    }

    /// Records a SlotWritten event with frame extra data.
    pub fn push_slot_written_with_extra(
        &mut self,
        slot: SlotIdx,
        value: SlotValue,
        taint: Taint,
        extra: Option<CollectPaginationState>,
    ) -> Result<(), EngineError> {
        if let Some(state) = extra
            && self.events.len() >= self.capacity
        {
            return Err(EngineError::CollectEvidenceCapacityExceeded {
                run_id: state.run_id,
                slot,
                capacity: self.capacity,
                len: self.events.len(),
                required: REQUIRED_COLLECT_SLOT_EXTRA,
            });
        }
        if self.events.len() < self.capacity {
            self.events.push(EvidenceEvent::SlotWritten {
                slot,
                value,
                taint,
                extra,
            });
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
        Ok(())
    }

    /// Drains all collected events, returning them for processing.
    pub fn drain(&mut self) -> Vec<EvidenceEvent> {
        self.dropped = 0;
        core::mem::take(&mut self.events)
    }

    /// Restores a single evidence event to the collector. Used by
    /// [`crate::shard::Shard::flush_evidence`] when a partial flush
    /// fails so the unprocessed suffix can be retried instead of
    /// dropped (RS-205).
    ///
    /// Capacity overflow is reported via the `dropped` counter rather
    /// than a panic; downstream callers can observe the loss.
    pub fn push_event(&mut self, event: EvidenceEvent) {
        match event {
            EvidenceEvent::StepStarted { step } => self.push_step_started(step),
            EvidenceEvent::StepSucceeded {
                step,
                output,
                attempt,
            } => {
                self.push_step_succeeded(step, output, attempt);
            }
            EvidenceEvent::SlotWritten {
                slot,
                value,
                taint,
                extra,
            } => {
                if extra.is_some() {
                    let _ = self.push_slot_written_with_extra(slot, value, taint, extra);
                } else if self.events.len() < self.capacity {
                    self.events.push(EvidenceEvent::SlotWritten {
                        slot,
                        value,
                        taint,
                        extra: None,
                    });
                } else {
                    self.dropped = self.dropped.saturating_add(1);
                }
            }
        }
    }

    /// Returns the number of collected events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns true if no events have been collected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Returns the number of events dropped due to capacity limits.
    #[must_use]
    pub fn dropped(&self) -> usize {
        self.dropped
    }

    /// Returns the configured capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Default for EvidenceCollector {
    fn default() -> Self {
        Self::new()
    }
}
