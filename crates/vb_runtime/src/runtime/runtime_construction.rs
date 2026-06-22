#![forbid(unsafe_code)]
//! Runtime construction and lifecycle methods.

use std::num::NonZeroUsize;

use crate::Runtime;
use crate::journal::SharedRuntimeJournal;
use crate::shard::Shard;
use crate::shard::ShardConfig;

impl Runtime {
    /// Creates a runtime with a noop journal sink.
    pub fn new(shard_count: NonZeroUsize, config: ShardConfig) -> crate::RuntimeResult<Self> {
        Self::new_with_journal(
            shard_count,
            config,
            crate::journal::NoopRuntimeJournal::shared(),
        )
    }

    /// Creates a runtime with an explicit runtime journal sink.
    pub fn new_with_journal(
        shard_count: NonZeroUsize,
        config: ShardConfig,
        journal: SharedRuntimeJournal,
    ) -> crate::RuntimeResult<Self> {
        config.validate()?;
        let count = shard_count.get();
        let mut shards = Vec::with_capacity(count);
        let mut index = 0usize;
        while index < count {
            shards.push(Shard::new_with_journal(config, journal.clone())?);
            index = index.saturating_add(1);
        }
        Ok(Self {
            shards,
            shard_count: count,
            journal,
        })
    }

    /// Consumes the runtime and returns the underlying shared runtime journal.
    ///
    /// This is a terminal operation — the runtime and its shards must no
    /// longer be used after this call. Callers typically use the returned
    /// journal to flush pending writes (e.g. by calling `close()` on the
    /// inner `FjallJournal`) before the process exits.
    #[must_use]
    pub fn journal(self) -> SharedRuntimeJournal {
        self.journal
    }

    /// Creates a runtime with an explicit artifact store.
    ///
    /// Test-support constructor used by admission tests to wire a
    /// `AlwaysPresentArtifactStore` into strict-mode admission so the
    /// step-budget gate can be evaluated in isolation.
    #[cfg(any(test, feature = "test-util"))]
    pub fn new_with_artifact_store(
        shard_count: NonZeroUsize,
        config: ShardConfig,
        artifact_store: crate::admission::SharedAcceptedArtifactStore,
    ) -> crate::RuntimeResult<Self> {
        config.validate()?;
        let journal = crate::journal::NoopRuntimeJournal::shared();
        let count = shard_count.get();
        let mut shards = Vec::with_capacity(count);
        let mut index = 0usize;
        while index < count {
            shards.push(Shard::new_with_journal_and_artifact_store(
                config,
                journal.clone(),
                crate::admission::SharedAcceptedArtifactStore::clone(&artifact_store),
            )?);
            index = index.saturating_add(1);
        }
        Ok(Self {
            shards,
            shard_count: count,
            journal,
        })
    }
}
