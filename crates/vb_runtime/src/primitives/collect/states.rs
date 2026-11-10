impl CollectStates {
    /// Cold transport may borrow an empty terminal table without cloning it.
    pub(crate) fn transport_tables_empty(&self) -> bool {
        self.entries.is_empty() && self.lineages.is_empty()
    }

    /// Create an empty state table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Items consumed by every collect page delivered so far, including
    /// collects whose pagination state was already removed at completion.
    #[must_use]
    pub fn total_items_consumed(&self) -> u64 {
        self.total_items_consumed
    }

    /// Pages delivered by every collect so far, including collects whose
    /// pagination state was already removed at completion.
    #[must_use]
    pub fn total_pages_delivered(&self) -> u64 {
        self.total_pages_delivered
    }

    /// Insert or replace the state for the given key.
    pub fn upsert(&mut self, state: CollectPaginationState) -> Result<(), EngineError> {
        let key = (state.run_id, state.collector_slot);
        let previous = self.entries.get(&key).map_or(0, |prior| prior.cursor);
        let delivered = state.cursor.saturating_sub(previous);
        let total = self
            .total_items_consumed
            .checked_add(u64::try_from(delivered).map_err(|_| {
                EngineError::InternalInvariantViolation {
                    reason: "collect item counter overflow",
                }
            })?)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "collect item counter overflow",
            })?;
        let pages_delivered = match self.entries.get(&key) {
            None => 1,
            Some(prior) => u64::from(prior.current_page != state.current_page),
        };
        let pages_total = self
            .total_pages_delivered
            .checked_add(pages_delivered)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "collect page counter overflow",
            })?;
        self.record_lineage(key, state.current_page)?;
        self.entries.insert(key, state);
        self.total_items_consumed = total;
        self.total_pages_delivered = pages_total;
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

    fn require_current_page(
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
        let state: CollectPaginationState =
            postcard::from_bytes(extra).map_err(|_| EngineError::CollectExtraHydrationFailed {
                kind: CollectExtraHydrationFailureKind::DecodeFailed,
                run_id,
                collector_slot,
                event_seq,
            })?;
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
        extra: &[u8],
    ) -> Result<(), EngineError> {
        match vb_storage::decode_slot_written_extra(extra) {
            Ok(vb_storage::DecodedSlotWrittenExtra::Envelope(envelope)) => {
                match envelope.frame_extra {
                    Some(frame_extra) => {
                        self.hydrate_frame_extra(run, slot, seq, value, &frame_extra)
                    }
                    None => Ok(()),
                }
            }
            Ok(vb_storage::DecodedSlotWrittenExtra::LegacyFrameExtra(frame_extra)) => {
                self.hydrate_frame_extra(run, slot, seq, value, frame_extra)
            }
            Err(_) => Err(EngineError::CollectExtraHydrationFailed {
                kind: CollectExtraHydrationFailureKind::DecodeFailed,
                run_id: run,
                collector_slot: slot,
                event_seq: Some(core_event_seq(seq)),
            }),
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
