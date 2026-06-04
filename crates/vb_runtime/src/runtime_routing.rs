#![forbid(unsafe_code)]

use vb_core::ids::RunId;

use crate::runtime::Runtime;
use crate::shard::Shard;
use crate::{RuntimeError, RuntimeResult};

impl Runtime {
    pub(crate) fn shard_index(&self, run: RunId) -> usize {
        let hash = run.get();
        let Ok(count) = u64::try_from(self.shard_count) else {
            return 0;
        };
        let Some(remainder) = hash.checked_rem(count) else {
            return 0;
        };
        let Ok(index) = usize::try_from(remainder) else {
            return 0;
        };
        index
    }

    pub(crate) fn shard_for(&self, run: RunId) -> RuntimeResult<&Shard> {
        let index = self.shard_index(run);
        self.shards.get(index).ok_or(RuntimeError::RunNotFound)
    }

    pub(crate) fn shard_for_mut(&mut self, run: RunId) -> RuntimeResult<&mut Shard> {
        let index = self.shard_index(run);
        self.shards.get_mut(index).ok_or(RuntimeError::RunNotFound)
    }
}
