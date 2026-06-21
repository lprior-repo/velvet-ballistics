#![forbid(unsafe_code)]
//! Collect pagination state management.

#[cfg(kani)]
use std::collections::BTreeMap as Map;
#[cfg(not(kani))]
use std::collections::HashMap as Map;

use serde::{Deserialize, Serialize};
use vb_core::errors::{
    CollectExtraHydrationFailureKind, CollectPageOrderViolationKind, EngineError,
};
use vb_core::ids::{EventSeq, ListId, RunId, SlotIdx};
use vb_core::value::SlotValue;
use vb_storage::JournalEvent;

/// Per-run pagination state stored in a side table keyed by (RunId, SlotIdx).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectPaginationState {
    /// Run owning this pagination state.
    pub run_id: RunId,
    /// Collector slot holding the current page.
    pub collector_slot: SlotIdx,
    /// Source list being paginated.
    pub source: ListId,
    /// Current page list expected in the collector slot.
    pub current_page: ListId,
    /// Next source item cursor.
    pub cursor: usize,
    /// Maximum page size.
    pub page_size: usize,
    /// Source item count captured at start.
    pub item_count: usize,
    /// Collect item limit.
    pub limit: usize,
    /// Optional wall-clock collect time limit.
    pub time_limit_ms: Option<u64>,
    /// Start timestamp in milliseconds since UNIX epoch.
    pub start_millis: u64,
    /// Flag indicating whether this state was hydrated from journal replay.
    /// When true, the state was restored from journal events and `start_millis`
    /// contains the original wall-clock time from the first execution.
    /// `collect_start` will skip re-capturing wall-clock time in this case
    /// to preserve deterministic replay behavior.
    pub from_journal: bool,
}

/// Side table replacing the global Mutex. Owns pagination state per (RunId, SlotIdx).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CollectStates {
    pub(crate) entries: Map<(RunId, SlotIdx), CollectPaginationState>,
    lineages: Map<(RunId, SlotIdx), CollectPageLineage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct CollectPageLineage {
    previous_page: Option<ListId>,
    stale_pages: Vec<ListId>,
}

impl CollectStates {
    /// Create an empty state table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace the state for the given key.
    pub fn upsert(&mut self, state: CollectPaginationState) -> Result<(), EngineError> {
        let key = (state.run_id, state.collector_slot);
        self.record_lineage(key, state.current_page)?;
        self.entries.insert(key, state);
        Ok(())
    }

    fn record_lineage(
        &mut self,
        key: (RunId, SlotIdx),
        next_page: ListId,
    ) -> Result<(), EngineError> {
        let Some(current) = self.entries.get(&key).map(|state| state.current_page) else {
            self.lineages.entry(key).or_default();
            return Ok(());
        };
        if current == next_page {
            return Ok(());
        }
        let lineage = self.lineages.entry(key).or_default();
        if let Some(previous) = lineage.previous_page {
            lineage.stale_pages.try_reserve(1).map_err(|_| {
                EngineError::InternalInvariantViolation {
                    reason: "collect lineage allocation failed",
                }
            })?;
            lineage.stale_pages.push(previous);
        }
        lineage.previous_page = Some(current);
        Ok(())
    }

    /// Find the state matching (run_id, collector_slot, current_page).
    pub fn find(
        &self,
        run_id: RunId,
        collector_slot: SlotIdx,
        current_page: ListId,
    ) -> Option<CollectPaginationState> {
        self.entries
            .get(&(run_id, collector_slot))
            .filter(|s| s.current_page == current_page)
            .copied()
    }

    pub(crate) fn require_current_page(
        &self,
        run_id: RunId,
        collector_slot: SlotIdx,
        observed_page: ListId,
    ) -> Result<CollectPaginationState, EngineError> {
        let Some(state) = self.entries.get(&(run_id, collector_slot)).copied() else {
            return Err(EngineError::InvalidCompiledWorkflow {
                reason: "collect pagination state missing",
            });
        };
        if state.current_page == observed_page {
            return Ok(state);
        }
        let kind = self.classify_observed_page((run_id, collector_slot), observed_page);
        Err(EngineError::CollectPageOrderViolation {
            kind,
            run_id,
            collector_slot,
            expected_page: state.current_page,
            observed_page,
        })
    }

    /// Remove state for the given key.
    pub fn remove(&mut self, run_id: RunId, collector_slot: SlotIdx) {
        let key = (run_id, collector_slot);
        self.entries.remove(&key);
        self.lineages.remove(&key);
    }

    fn classify_observed_page(
        &self,
        key: (RunId, SlotIdx),
        observed_page: ListId,
    ) -> CollectPageOrderViolationKind {
        let Some(lineage) = self.lineages.get(&key) else {
            return CollectPageOrderViolationKind::OutOfOrder;
        };
        if lineage.previous_page == Some(observed_page) {
            return CollectPageOrderViolationKind::Duplicate;
        }
        if lineage.stale_pages.contains(&observed_page) {
            return CollectPageOrderViolationKind::Stale;
        }
        CollectPageOrderViolationKind::OutOfOrder
    }

    /// Serialize the active state for a collector slot as durable frame extra data.
    pub fn capture_extra(
        &self,
        run_id: RunId,
        collector_slot: SlotIdx,
    ) -> Result<Option<Vec<u8>>, EngineError> {
        self.entries
            .get(&(run_id, collector_slot))
            .map(postcard::to_allocvec)
            .transpose()
            .map_err(|_| EngineError::InvalidCompiledWorkflow {
                reason: "collect pagination state encode failed",
            })
    }

    /// Capture the active state for a collector slot.
    #[must_use]
    pub fn capture_state(
        &self,
        run_id: RunId,
        collector_slot: SlotIdx,
    ) -> Option<CollectPaginationState> {
        self.entries.get(&(run_id, collector_slot)).copied()
    }

    /// Hydrate durable frame extra data into the pagination side table.
    pub fn hydrate_extra(
        &mut self,
        run_id: RunId,
        collector_slot: SlotIdx,
        extra: &[u8],
    ) -> Result<(), EngineError> {
        self.hydrate_extra_with_context(run_id, collector_slot, None, None, extra)
    }

    fn hydrate_extra_with_context(
        &mut self,
        run_id: RunId,
        collector_slot: SlotIdx,
        event_seq: Option<EventSeq>,
        expected_page: Option<ListId>,
        extra: &[u8],
    ) -> Result<(), EngineError> {
        if extra.is_empty() {
            return Err(EngineError::CollectExtraHydrationFailed {
                kind: CollectExtraHydrationFailureKind::EmptyExtra,
                run_id,
                collector_slot,
                event_seq,
            });
        }
        let mut state: CollectPaginationState =
            postcard::from_bytes(extra).map_err(|_| EngineError::CollectExtraHydrationFailed {
                kind: CollectExtraHydrationFailureKind::DecodeFailed,
                run_id,
                collector_slot,
                event_seq,
            })?;
        // Mark state as hydrated from journal to preserve original wall-clock time
        // during replay and prevent collect_start from overwriting start_millis.
        state.from_journal = true;
        validate_hydrated_identity(&state, run_id, collector_slot, event_seq)?;
        if let Some(expected) = expected_page {
            validate_hydrated_page(&state, run_id, collector_slot, event_seq, expected)?;
        }
        self.upsert(state)
    }

    /// Hydrate durable pagination extras carried by slot-write journal events.
    pub fn hydrate_journal_events(&mut self, events: &[JournalEvent]) -> Result<(), EngineError> {
        events
            .iter()
            .try_for_each(|event| self.hydrate_journal_event(event))
    }

    fn hydrate_journal_event(&mut self, event: &JournalEvent) -> Result<(), EngineError> {
        match event {
            JournalEvent::SlotWrittenEvent {
                run,
                slot,
                seq,
                value,
                extra: Some(extra),
                ..
            } => self.hydrate_slot_written_extra(*run, *slot, *seq, value.as_deref(), extra),
            _ => Ok(()),
        }
    }

    fn hydrate_slot_written_extra(
        &mut self,
        run: RunId,
        slot: SlotIdx,
        seq: vb_storage::EventSeq,
        value: Option<&[u8]>,
        extra: &vb_storage::SlotWriteExtra,
    ) -> Result<(), EngineError> {
        match extra {
            vb_storage::SlotWriteExtra::Versioned(envelope) => {
                match envelope.frame_extra.as_deref() {
                    Some(frame_extra) => {
                        self.hydrate_frame_extra(run, slot, seq, value, frame_extra)
                    }
                    None => Ok(()),
                }
            }
            vb_storage::SlotWriteExtra::Legacy(frame_extra) => {
                self.hydrate_frame_extra(run, slot, seq, value, frame_extra.as_slice())
            }
            _ => Ok(()),
        }
    }

    fn hydrate_frame_extra(
        &mut self,
        run: RunId,
        slot: SlotIdx,
        seq: vb_storage::EventSeq,
        value: Option<&[u8]>,
        extra: &[u8],
    ) -> Result<(), EngineError> {
        match collect_page_from_event_value(run, slot, Some(core_event_seq(seq)), value)? {
            Some(expected_page) => self.hydrate_extra_with_context(
                run,
                slot,
                Some(core_event_seq(seq)),
                Some(expected_page),
                extra,
            ),
            None if value.is_none() => {
                self.hydrate_extra_with_context(run, slot, Some(core_event_seq(seq)), None, extra)
            }
            None => Ok(()),
        }
    }
}

fn core_event_seq(seq: vb_storage::EventSeq) -> EventSeq {
    EventSeq::new(seq.get())
}

fn validate_hydrated_identity(
    state: &CollectPaginationState,
    run_id: RunId,
    collector_slot: SlotIdx,
    event_seq: Option<EventSeq>,
) -> Result<(), EngineError> {
    if state.run_id != run_id {
        return Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::RunMismatch {
                expected: run_id,
                actual: state.run_id,
            },
            run_id,
            collector_slot,
            event_seq,
        });
    }
    if state.collector_slot != collector_slot {
        return Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::SlotMismatch {
                expected: collector_slot,
                actual: state.collector_slot,
            },
            run_id,
            collector_slot,
            event_seq,
        });
    }
    Ok(())
}

fn validate_hydrated_page(
    state: &CollectPaginationState,
    run_id: RunId,
    collector_slot: SlotIdx,
    event_seq: Option<EventSeq>,
    expected: ListId,
) -> Result<(), EngineError> {
    if state.current_page != expected {
        return Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::CurrentPageMismatch {
                expected,
                actual: state.current_page,
            },
            run_id,
            collector_slot,
            event_seq,
        });
    }
    Ok(())
}

pub(crate) fn collect_page_from_event_value(
    run_id: RunId,
    collector_slot: SlotIdx,
    event_seq: Option<EventSeq>,
    value: Option<&[u8]>,
) -> Result<Option<ListId>, EngineError> {
    match value {
        Some(bytes) => match postcard::from_bytes::<SlotValue>(bytes) {
            Ok(SlotValue::List(page)) => Ok(Some(page)),
            Ok(_) => Ok(None),
            Err(_) => Err(EngineError::CollectExtraHydrationFailed {
                kind: CollectExtraHydrationFailureKind::DecodeFailed,
                run_id,
                collector_slot,
                event_seq,
            }),
        },
        None => Ok(None),
    }
}
