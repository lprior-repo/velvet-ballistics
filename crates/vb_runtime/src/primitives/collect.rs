#![forbid(unsafe_code)]
//! Collect pagination primitive handlers.

#[cfg(kani)]
use std::collections::BTreeMap as Map;
#[cfg(not(kani))]
use std::collections::HashMap as Map;
use std::time::SystemTime;

use vb_core::time::{DeterministicTimeSource, TimeSource};

use serde::{Deserialize, Serialize};
use vb_core::errors::{
    CollectExtraHydrationFailureKind, CollectPageOrderViolationKind, EngineError,
};
use vb_core::frame::RunFrame;
use vb_core::ids::{EventSeq, ListId, RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_storage::JournalEvent;

use super::helpers::{expect_list, jump_to, jump_to_body, jump_to_next, require_output};

mod checkpoint;
mod checkpoint_restore;
pub use checkpoint::{CollectStatesCheckpointV1, CollectStatesCheckpointV1Error};

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
}

/// Side table replacing the global Mutex. Owns pagination state per (RunId, SlotIdx).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CollectStates {
    entries: Map<(RunId, SlotIdx), CollectPaginationState>,
    lineages: Map<(RunId, SlotIdx), CollectPageLineage>,
    total_items_consumed: u64,
    total_pages_delivered: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct CollectPageLineage {
    previous_page: Option<ListId>,
    stale_pages: Vec<ListId>,
}

include!("collect/states.rs");

include!("collect/hydration.rs");

include!("collect/execute.rs");

include!("collect/helpers.rs");

include!("collect/kani.rs");

#[cfg(test)]
#[path = "collect/tests.rs"]
mod tests;
