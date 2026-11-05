#![forbid(unsafe_code)]
//! Checkpoint capture and restore for collect pagination state.

use vb_core::ids::{RunId, SlotIdx};

use super::checkpoint::{
    CollectPageLineageCheckpointV1, CollectStatesCheckpointV1, CollectStatesCheckpointV1Error,
};
use super::{CollectPageLineage, CollectPaginationState, CollectStates};

impl CollectStates {
    /// Copies pagination state in stable key order for deterministic encoding.
    #[must_use]
    pub fn checkpoint_v1(&self) -> CollectStatesCheckpointV1 {
        let mut entries: Vec<_> = self.entries.values().copied().collect();
        entries.sort_by_key(|state| (state.run_id, state.collector_slot));

        let mut lineages: Vec<_> = self
            .lineages
            .iter()
            .map(
                |((run_id, collector_slot), lineage)| CollectPageLineageCheckpointV1 {
                    run_id: *run_id,
                    collector_slot: *collector_slot,
                    previous_page: lineage.previous_page,
                    stale_pages: lineage.stale_pages.clone(),
                },
            )
            .collect();
        lineages.sort_by_key(|lineage| (lineage.run_id, lineage.collector_slot));

        CollectStatesCheckpointV1 {
            entries,
            lineages,
            total_items_consumed: self.total_items_consumed,
            total_pages_delivered: self.total_pages_delivered,
        }
    }

    /// Restores checkpoint state only when keys are unique and every lineage is owned.
    pub fn from_checkpoint_v1(
        checkpoint: CollectStatesCheckpointV1,
    ) -> Result<Self, CollectStatesCheckpointV1Error> {
        let entries = dedupe_entries(checkpoint.entries)?;

        let mut lineages = super::Map::new();
        for entry in checkpoint.lineages {
            let key = (entry.run_id, entry.collector_slot);
            if !entries.contains_key(&key) {
                return Err(CollectStatesCheckpointV1Error::OrphanLineage);
            }
            let lineage = CollectPageLineage {
                previous_page: entry.previous_page,
                stale_pages: entry.stale_pages,
            };
            if lineages.insert(key, lineage).is_some() {
                return Err(CollectStatesCheckpointV1Error::DuplicateLineage);
            }
        }
        Ok(Self {
            entries,
            lineages,
            total_items_consumed: checkpoint.total_items_consumed,
            total_pages_delivered: checkpoint.total_pages_delivered,
        })
    }
}

fn dedupe_entries(
    states: Vec<CollectPaginationState>,
) -> Result<super::Map<(RunId, SlotIdx), CollectPaginationState>, CollectStatesCheckpointV1Error> {
    let mut entries = super::Map::new();
    for state in states {
        let key = (state.run_id, state.collector_slot);
        if entries.insert(key, state).is_some() {
            return Err(CollectStatesCheckpointV1Error::DuplicateEntry);
        }
    }
    Ok(entries)
}
