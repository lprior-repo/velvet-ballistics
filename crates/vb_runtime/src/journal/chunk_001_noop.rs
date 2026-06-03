/// Journal implementation that intentionally drops all events.
#[derive(Debug, Default)]
pub struct NoopRuntimeJournal;

impl NoopRuntimeJournal {
    /// Creates a shared noop journal for explicitly non-durable tests or benchmarks.
    #[must_use]
    pub fn shared_for_tests_and_benchmarks() -> SharedRuntimeJournal {
        Arc::new(Self)
    }

    /// Creates a shared noop journal for callers that explicitly select no durability.
    #[must_use]
    pub fn shared() -> SharedRuntimeJournal {
        Self::shared_for_tests_and_benchmarks()
    }
}

impl RuntimeJournal for NoopRuntimeJournal {
    fn append(&self, _event: RuntimeJournalEvent) -> RuntimeResult<()> {
        Ok(())
    }
    fn probe(&self) -> RuntimeResult<()> {
        Ok(())
    }
}
