impl Runtime {
    /// Installs names available to secret declarations on each shard.
    pub fn set_available_secret_names(&mut self, names: Box<[Box<str>]>) {
        let names: std::sync::Arc<[Box<str>]> = names.into();
        for shard in &mut self.shards {
            shard.set_available_secret_names(names.clone());
        }
    }

    /// Creates a new runtime with the given number of shards, per-shard configuration, and
    /// explicit [`SharedRuntimeJournal`].
    ///
    /// The journal argument is **required**: this constructor will not silently pick a
    /// non-durable default. Master §18 requires Fjall-backed persistence for recovery,
    /// so production callers must wire a [`crate::journal::StorageRuntimeJournal`] (strict
    /// barrier) or [`crate::journal::QueuedStorageRuntimeJournal`] (group-committed).
    /// Tests and benchmarks that do not need durability should pass
    /// [`crate::journal::VolatileRuntimeJournal::shared`] or
    /// [`crate::journal::NoopRuntimeJournal::shared_for_tests_and_benchmarks`] so the
    /// call site documents the non-durable intent.
    #[must_use]
    pub fn new(
        shard_count: NonZeroUsize,
        config: ShardConfig,
        journal: SharedRuntimeJournal,
    ) -> Self {
        let count = shard_count.get();
        let mut shards = Vec::with_capacity(count);
        let mut index = 0usize;
        while index < count {
            shards.push(Shard::new_with_journal(config, journal.clone()));
            index = index.saturating_add(1);
        }
        Self {
            shards,
            shard_count: count,
            journal,
            policy: config.policy,
            live_policy: crate::policy::LivePolicy::new(),
        }
    }

    /// Creates a new runtime whose journal events survive only until process exit.
    ///
    /// This is the **test/benchmark-only** non-durable path. It is intentionally named
    /// long and explicit so every call site self-documents the trade-off. Production
    /// callers must use [`Runtime::new`] with a storage-backed
    /// [`SharedRuntimeJournal`].
    #[must_use]
    pub fn new_for_tests_and_benchmarks_only(
        shard_count: NonZeroUsize,
        config: ShardConfig,
    ) -> Self {
        Self::new(
            shard_count,
            config,
            crate::journal::VolatileRuntimeJournal::shared(),
        )
    }

    /// Configures the live policy with a verified artifact and rule.
    ///
    /// Updates actor-owned runtime state; does not persist to durable storage.
    pub fn configure_live_policy(
        &mut self,
        digest: vb_core::policy::PolicyDigest,
        rule: vb_core::policy::PolicyRule,
    ) -> RuntimeResult<()> {
        self.configure_live_policy_helper(digest, rule);
        Ok(())
    }

    /// Activates the emergency deny overlay with strict durability.
    ///
    /// Persists the emergency marker, verifies the returned state is Active,
    /// then mirrors into owned live policy state. Fails on corruption or
    /// unsupported marker versions.
    pub fn activate_emergency_deny(&mut self) -> RuntimeResult<()> {
        self.activate_emergency_deny_helper()
    }
}
