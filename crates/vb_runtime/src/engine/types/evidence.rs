/// Evidence event emitted by the deterministic drive loop for each step.
///
/// These events are collected during `drive_deterministic_full` and drained
/// by the shard to emit to the journal and trace ring. This satisfies
/// the Phase 40/44 evidence chain requirement that every deterministic step
/// emits `StepStarted` before `SlotWritten`, followed by `StepSucceeded`.
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
    },
    /// Step failed before entering its error handler.
    StepFailed {
        /// Failed step.
        step: StepIdx,
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
#[derive(Debug, Clone)]
pub struct EvidenceCollector {
    events: Vec<EvidenceEvent>,
    capacity: usize,
}

impl EvidenceCollector {
    /// Creates a new empty collector with a default capacity bound.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            capacity: DEFAULT_EVIDENCE_CAPACITY,
        }
    }

    /// Creates a new collector with a specific capacity bound.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Vec::new(),
            capacity,
        }
    }

    /// Records a StepStarted event.
    ///
    /// Returns `EngineError::EvidenceCapacityExceeded` if the collector is at
    /// capacity; the event is NOT pushed in that case. Callers must propagate
    /// the error — silent drops are no longer possible on this path.
    pub fn push_step_started(&mut self, step: StepIdx) -> Result<(), EngineError> {
        if self.events.len() >= self.capacity {
            return Err(EngineError::EvidenceCapacityExceeded {
                step,
                slot: SlotIdx::ZERO,
                capacity: self.capacity,
                len: self.events.len(),
                required: REQUIRED_STEP_STARTED,
            });
        }
        self.events.push(EvidenceEvent::StepStarted { step });
        Ok(())
    }

    /// Records a StepSucceeded event.
    ///
    /// Returns `EngineError::EvidenceCapacityExceeded` if the collector is at
    /// capacity; the event is NOT pushed in that case.
    pub fn push_step_succeeded(
        &mut self,
        step: StepIdx,
        output: Option<SlotIdx>,
    ) -> Result<(), EngineError> {
        if self.events.len() >= self.capacity {
            return Err(EngineError::EvidenceCapacityExceeded {
                step,
                slot: SlotIdx::ZERO,
                capacity: self.capacity,
                len: self.events.len(),
                required: REQUIRED_STEP_SUCCEEDED,
            });
        }
        self.events
            .push(EvidenceEvent::StepSucceeded { step, output });
        Ok(())
    }

    /// Records a failed deterministic step before handler dispatch.
    pub fn push_step_failed(&mut self, step: StepIdx) -> Result<(), EngineError> {
        if self.events.len() >= self.capacity {
            return Err(EngineError::EvidenceCapacityExceeded {
                step,
                slot: SlotIdx::ZERO,
                capacity: self.capacity,
                len: self.events.len(),
                required: "step_failed",
            });
        }
        self.events.push(EvidenceEvent::StepFailed { step });
        Ok(())
    }

    /// Records a SlotWritten event.
    ///
    /// Returns `EngineError::EvidenceCapacityExceeded` if the collector is at
    /// capacity; the event is NOT pushed in that case.
    pub fn push_slot_written(
        &mut self,
        slot: SlotIdx,
        value: SlotValue,
    ) -> Result<(), EngineError> {
        self.push_slot_written_with_taint(slot, value, Taint::Clean)
    }

    /// Records a SlotWritten event with explicit taint.
    ///
    /// Returns `EngineError::EvidenceCapacityExceeded` if the collector is at
    /// capacity; the event is NOT pushed in that case.
    pub fn push_slot_written_with_taint(
        &mut self,
        slot: SlotIdx,
        value: SlotValue,
        taint: Taint,
    ) -> Result<(), EngineError> {
        if self.events.len() >= self.capacity {
            return Err(EngineError::EvidenceCapacityExceeded {
                step: StepIdx::ZERO,
                slot,
                capacity: self.capacity,
                len: self.events.len(),
                required: REQUIRED_SLOT_WRITTEN,
            });
        }
        self.events.push(EvidenceEvent::SlotWritten {
            slot,
            value,
            taint,
            extra: None,
        });
        Ok(())
    }

    /// Records a SlotWritten event with frame extra data.
    ///
    /// Returns `EngineError::CollectEvidenceCapacityExceeded` when the
    /// collector is at capacity and `extra` is `Some` (collect-extra must
    /// be preserved). Returns `EngineError::EvidenceCapacityExceeded` for
    /// the non-extra overflow path. Either way, the event is NOT pushed.
    pub fn push_slot_written_with_extra(
        &mut self,
        slot: SlotIdx,
        value: SlotValue,
        taint: Taint,
        extra: Option<CollectPaginationState>,
    ) -> Result<(), EngineError> {
        if self.events.len() >= self.capacity {
            return Err(match extra {
                Some(state) => EngineError::CollectEvidenceCapacityExceeded {
                    run_id: state.run_id,
                    slot,
                    capacity: self.capacity,
                    len: self.events.len(),
                    required: REQUIRED_COLLECT_SLOT_EXTRA,
                },
                None => EngineError::EvidenceCapacityExceeded {
                    step: StepIdx::ZERO,
                    slot,
                    capacity: self.capacity,
                    len: self.events.len(),
                    required: REQUIRED_SLOT_WRITTEN,
                },
            });
        }
        self.events.push(EvidenceEvent::SlotWritten {
            slot,
            value,
            taint,
            extra,
        });
        Ok(())
    }

    /// Drains all collected events, returning them for processing.
    pub fn drain(&mut self) -> Vec<EvidenceEvent> {
        core::mem::take(&mut self.events)
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
