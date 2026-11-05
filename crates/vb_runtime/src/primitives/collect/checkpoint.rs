#![forbid(unsafe_code)]
//! Versioned, deterministic checkpoint representation for collect state.

use serde::{Deserialize, Serialize};
use vb_core::ids::{ListId, RunId, SlotIdx};

use super::{CollectPageLineage, CollectPaginationState, CollectStates};

/// Complete v1 checkpoint of pagination and stale-page lineage metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectStatesCheckpointV1 {
    /// Active pagination states in canonical `(run, collector_slot)` order.
    pub entries: Vec<CollectPaginationState>,
    /// Previous and stale page lineage in canonical key order.
    pub lineages: Vec<CollectPageLineageCheckpointV1>,
    /// Items consumed by every collect page delivered so far, including
    /// collects whose pagination state was already removed at completion.
    /// Checkpoints written before this field existed do not decode;
    /// that failure is loud, never a silent reinterpretation.
    pub total_items_consumed: u64,
    /// Pages delivered by every collect so far, including collects whose
    /// pagination state was already removed at completion.
    /// Checkpoints written before this field existed do not decode;
    /// that failure is loud, never a silent reinterpretation.
    pub total_pages_delivered: u64,
}

/// V1 checkpoint entry for one collector's page lineage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectPageLineageCheckpointV1 {
    /// Run owning the collector.
    pub run_id: RunId,
    /// Collector slot key.
    pub collector_slot: SlotIdx,
    /// Immediately preceding current page, if present.
    pub previous_page: Option<ListId>,
    /// Pages older than `previous_page`, in observation order.
    pub stale_pages: Vec<ListId>,
}

/// Validation failures while restoring a collect checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectStatesCheckpointV1Error {
    /// An active pagination key is duplicated.
    DuplicateEntry,
    /// A page-lineage key is duplicated.
    DuplicateLineage,
    /// Page lineage has no corresponding active pagination state.
    OrphanLineage,
}

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

#[cfg(test)]
#[path = "checkpoint_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "checkpoint_total_tests.rs"]
mod total_tests;
