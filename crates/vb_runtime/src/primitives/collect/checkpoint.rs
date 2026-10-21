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

        CollectStatesCheckpointV1 { entries, lineages }
    }

    /// Restores checkpoint state only when keys are unique and every lineage is owned.
    pub fn from_checkpoint_v1(
        checkpoint: CollectStatesCheckpointV1,
    ) -> Result<Self, CollectStatesCheckpointV1Error> {
        let mut entries = super::Map::new();
        for state in checkpoint.entries {
            let key = (state.run_id, state.collector_slot);
            if entries.insert(key, state).is_some() {
                return Err(CollectStatesCheckpointV1Error::DuplicateEntry);
            }
        }

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
        Ok(Self { entries, lineages })
    }
}

#[cfg(test)]
mod tests {
    use super::{CollectStates, CollectStatesCheckpointV1, CollectStatesCheckpointV1Error};
    use crate::primitives::collect::CollectPaginationState;
    use vb_core::ids::{ListId, RunId, SlotIdx};

    fn state(run: RunId, collector_slot: SlotIdx) -> CollectPaginationState {
        CollectPaginationState {
            run_id: run,
            collector_slot,
            source: ListId::new(1),
            current_page: ListId::new(3),
            cursor: 4,
            page_size: 5,
            item_count: 10,
            limit: 20,
            time_limit_ms: Some(100),
            start_millis: 77,
        }
    }

    #[test]
    fn collect_checkpoint_roundtrips_canonical_state_and_lineage() -> Result<(), String> {
        let mut states = CollectStates::new();
        states
            .upsert(state(RunId::new(7), SlotIdx::new(2)))
            .map_err(|error| format!("first state setup failed: {error}"))?;
        states
            .upsert(state(RunId::new(3), SlotIdx::new(1)))
            .map_err(|error| format!("second state setup failed: {error}"))?;
        states
            .upsert(CollectPaginationState {
                current_page: ListId::new(8),
                ..state(RunId::new(7), SlotIdx::new(2))
            })
            .map_err(|error| format!("lineage setup failed: {error}"))?;
        states
            .upsert(CollectPaginationState {
                current_page: ListId::new(9),
                ..state(RunId::new(7), SlotIdx::new(2))
            })
            .map_err(|error| format!("lineage setup failed: {error}"))?;

        let checkpoint = states.checkpoint_v1();
        let bytes = postcard::to_allocvec(&checkpoint)
            .map_err(|error| format!("checkpoint encode failed: {error}"))?;
        let decoded = postcard::from_bytes(&bytes)
            .map_err(|error| format!("checkpoint decode failed: {error}"))?;
        let restored = CollectStates::from_checkpoint_v1(decoded)
            .map_err(|error| format!("checkpoint restore failed: {error:?}"))?;

        assert_eq!(restored.checkpoint_v1(), checkpoint);
        Ok(())
    }

    #[test]
    fn collect_checkpoint_rejects_orphan_lineage() {
        let checkpoint = CollectStatesCheckpointV1 {
            entries: Vec::new(),
            lineages: vec![super::CollectPageLineageCheckpointV1 {
                run_id: RunId::new(1),
                collector_slot: SlotIdx::new(0),
                previous_page: None,
                stale_pages: Vec::new(),
            }],
        };

        assert_eq!(
            CollectStates::from_checkpoint_v1(checkpoint),
            Err(CollectStatesCheckpointV1Error::OrphanLineage)
        );
    }
}
