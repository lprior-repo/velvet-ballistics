//! Test-only RuntimeJournal wrapper that overrides only
//! append_policy_decision_strict to return a controlled failure.
//! All other methods delegate to the real underlying journal.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use vb_runtime::journal::{RuntimeJournal, SharedRuntimeJournal, StorageRuntimeJournal};
use vb_runtime::{RuntimeError, RuntimeResult};
use vb_storage::{EventSeq, FjallJournal};

/// Test wrapper that delegates all real methods but overrides
/// append_policy_decision_strict to return a specific test error.
pub(crate) struct FailingPolicyPortJournal {
    inner: SharedRuntimeJournal,
    policy_port_called: Arc<AtomicBool>,
    captured_decision: Arc<Mutex<Option<vb_core::policy::DecisionRecord>>>,
}

impl FailingPolicyPortJournal {
    pub(crate) fn new(
        inner: Arc<FjallJournal>,
        catalog: vb_storage::executor::ExecutorCatalog,
    ) -> Self {
        Self {
            inner: StorageRuntimeJournal::shared_strict_with_catalog(inner, Arc::new(catalog)),
            policy_port_called: Arc::new(AtomicBool::new(false)),
            captured_decision: Arc::new(Mutex::new(None)),
        }
    }

    pub(crate) fn policy_port_called(&self) -> bool {
        self.policy_port_called.load(Ordering::SeqCst)
    }

    pub(crate) fn captured_decision(
        &self,
    ) -> Result<Option<vb_core::policy::DecisionRecord>, String> {
        self.captured_decision
            .lock()
            .map_err(|_| "captured_decision mutex poisoned".to_owned())
            .map(|guard| guard.clone())
    }
}

impl RuntimeJournal for FailingPolicyPortJournal {
    fn append(&self, event: vb_runtime::journal::RuntimeJournalEvent) -> RuntimeResult<()> {
        self.inner.append(event)
    }

    fn append_sequenced(
        &self,
        event: vb_runtime::journal::RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        self.inner.append_sequenced(event, seq)
    }

    fn append_sequenced_batch(
        &self,
        events: &[vb_runtime::journal::RuntimeJournalEvent],
        start_seq: EventSeq,
    ) -> RuntimeResult<()> {
        self.inner.append_sequenced_batch(events, start_seq)
    }

    fn probe(&self) -> RuntimeResult<()> {
        self.inner.probe()
    }

    fn storage_journal(&self) -> Option<Arc<FjallJournal>> {
        self.inner.storage_journal()
    }

    fn executor_catalog(&self) -> Option<Arc<vb_storage::executor::ExecutorCatalog>> {
        self.inner.executor_catalog()
    }

    fn append_policy_decision_strict(
        &self,
        decision: vb_core::policy::DecisionRecord,
        _seq: EventSeq,
    ) -> RuntimeResult<()> {
        self.policy_port_called.store(true, Ordering::SeqCst);
        let mut captured = self
            .captured_decision
            .lock()
            .map_err(|_| RuntimeError::Core {
                source: Box::new(vb_core::errors::CoreError::InternalInvariantViolation {
                    reason: "captured_decision mutex poisoned",
                }),
            })?;
        *captured = Some(decision);
        Err(RuntimeError::UnsupportedOperation {
            operation: "test_policy_decision_failure",
        })
    }
}
