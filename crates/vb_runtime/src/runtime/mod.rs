#![forbid(unsafe_code)]
//! Multi-shard runtime facade routing public commands to owning shards.
// canonical-path: crates/vb_runtime/src/runtime/mod.rs (submodules in
// crates/vb_runtime/src/runtime/{runtime_admission,runtime_ask,runtime_control,runtime_construction,runtime_metrics,runtime_recovery,runtime_sharding}.rs)
// Declared as `pub mod runtime;` in crates/vb_runtime/src/lib.rs.

mod runtime_construction;
mod runtime_admission;
mod runtime_ask;
mod runtime_control;
mod runtime_metrics;
mod runtime_recovery;
mod runtime_sharding;

pub use runtime_control::ActiveRunSummary;

use crate::shard::Shard;
use crate::journal::SharedRuntimeJournal;
use crate::RuntimeError;
use vb_core::ids::RunId;

/// Multi-shard runtime facade.
pub struct Runtime {
    pub(crate) shards: Vec<Shard>,
    shard_count: usize,
    pub(crate) journal: SharedRuntimeJournal,
}

impl Runtime {
    /// Computes the owning shard index for a run.
    #[must_use]
    pub fn shard_index(&self, run: RunId) -> usize {
        let Ok(count) = u64::try_from(self.shard_count) else {
            return 0;
        };
        let Some(remainder) = run.get().checked_rem(count) else {
            return 0;
        };
        usize::try_from(remainder).unwrap_or_default()
    }

    fn shard_for(&self, run: RunId) -> Result<&Shard, RuntimeError> {
        self.shards
            .get(self.shard_index(run))
            .ok_or(RuntimeError::RunNotFound)
    }
}
