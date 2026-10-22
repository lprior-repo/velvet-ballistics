//! Test-only RuntimeJournal wrapper that overrides only
//! append_policy_decision_strict to return a controlled failure.
//! All other methods delegate to the real underlying journal.

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use vb_runtime::journal::RuntimeJournal;
use vb_runtime::{RuntimeError, RuntimeResult};
use vb_storage::{FjallJournal, EventSeq};

/// Test wrapper that delegates all real methods but overrides
/// append_policy_decision_strict to return a specific test error.
pub struct FailingPolicyPortJournal {
    pub inner: Arc<FjallJournal>,
    pub catalog: vb_storage::executor::ExecutorCatalog,
    pub policy_port_called: Arc<AtomicBool>,
    pub captured_decision: Arc<Mutex<Option<vb_core::policy::DecisionRecord>>>,
}

impl FailingPolicyPortJournal {
    pub fn new(
        inner: Arc<FjallJournal>,
        catalog: vb_storage::executor::ExecutorCatalog,
    ) -> Self {
        Self {
            inner,
            catalog,
            policy_port_called: Arc::new(AtomicBool::new(false)),
            captured_decision: Arc::new(Mutex::new(None)),
        }
    }
}

// Delegate all methods except append_policy_decision_strict.

impl RuntimeJournal for FailingPolicyPortJournal {
    fn probe(&self) -> RuntimeResult<()> {
        self.inner.probe().map_err(RuntimeError::from)
    }

    fn begin_run(
        &self,
        run: vb_core::ids::RunId,
        workflow: &vb_core::workflow::CompiledWorkflow,
        admission: vb_runtime::admission::RunAdmission,
        inputs: &[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)],
    ) -> RuntimeResult<vb_storage::EventSeq> {
        self.inner
            .begin_run(run, workflow, admission, inputs)
            .map_err(RuntimeError::from)
    }

    fn step_event(
        &self,
        run: vb_core::ids::RunId,
        step: vb_core::ids::StepIdx,
        action: vb_core::ids::ActionId,
        inputs: &[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)],
        taints: &[vb_core::value::Taint],
    ) -> RuntimeResult<vb_storage::EventSeq> {
        self.inner
            .step_event(run, step, action, inputs, taints)
            .map_err(RuntimeError::from)
    }

    fn step_completed(
        &self,
        run: vb_core::ids::RunId,
        step: vb_core::ids::StepIdx,
        output: &[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)],
    ) -> RuntimeResult<vb_storage::EventSeq> {
        self.inner
            .step_completed(run, step, output)
            .map_err(RuntimeError::from)
    }

    fn step_failed(
        &self,
        run: vb_core::ids::RunId,
        step: vb_core::ids::StepIdx,
        error: &str,
    ) -> RuntimeResult<vb_storage::EventSeq> {
        self.inner
            .step_failed(run, step, error)
            .map_err(RuntimeError::from)
    }

    fn run_completed(&self, run: vb_core::ids::RunId) -> RuntimeResult<vb_storage::EventSeq> {
        self.inner
            .run_completed(run)
            .map_err(RuntimeError::from)
    }

    fn run_failed(
        &self,
        run: vb_core::ids::RunId,
        step: vb_core::ids::StepIdx,
        error: &str,
    ) -> RuntimeResult<vb_storage::EventSeq> {
        self.inner
            .run_failed(run, step, error)
            .map_err(RuntimeError::from)
    }

    fn run_suspended(
        &self,
        run: vb_core::ids::RunId,
        step: vb_core::ids::StepIdx,
        reason: &str,
    ) -> RuntimeResult<vb_storage::EventSeq> {
        self.inner
            .run_suspended(run, step, reason)
            .map_err(RuntimeError::from)
    }

    fn action_event(
        &self,
        run: vb_core::ids::RunId,
        step: vb_core::ids::StepIdx,
        action: vb_core::ids::ActionId,
        event: vb_core::action::ActionJournalEvent,
    ) -> RuntimeResult<vb_storage::EventSeq> {
        self.inner
            .action_event(run, step, action, event)
            .map_err(RuntimeError::from)
    }

    fn timer_event(
        &self,
        run: vb_core::ids::RunId,
        step: vb_core::ids::StepIdx,
        action: vb_core::ids::ActionId,
        event: vb_core::action::ActionJournalEvent,
    ) -> RuntimeResult<vb_storage::EventSeq> {
        self.inner
            .timer_event(run, step, action, event)
            .map_err(RuntimeError::from)
    }

    fn ask_event(
        &self,
        run: vb_core::ids::RunId,
        step: vb_core::ids::StepIdx,
        action: vb_core::ids::ActionId,
        event: vb_core::action::ActionJournalEvent,
    ) -> RuntimeResult<vb_storage::EventSeq> {
        self.inner
            .ask_event(run, step, action, event)
            .map_err(RuntimeError::from)
    }

    fn recovery_event(
        &self,
        run: vb_core::ids::RunId,
        step: vb_core::ids::StepIdx,
        action: vb_core::ids::ActionId,
        event: vb_core::recovery::RecoveryJournalEvent,
    ) -> RuntimeResult<vb_storage::EventSeq> {
        self.inner
            .recovery_event(run, step, action, event)
            .map_err(RuntimeError::from)
    }

    fn sync(&self) -> RuntimeResult<()> {
        self.inner.sync().map_err(RuntimeError::from)
    }

    fn close(&self) -> RuntimeResult<()> {
        self.inner.close().map_err(RuntimeError::from)
    }

    fn storage_journal(&self) -> Option<&FjallJournal> {
        Some(self.inner.as_ref())
    }

    fn executor_catalog(&self) -> Option<&vb_storage::executor::ExecutorCatalog> {
        Some(&self.catalog)
    }

    /// Override: return a specific test error and capture the decision.
    fn append_policy_decision_strict(
        &self,
        decision: vb_core::policy::DecisionRecord,
        _seq: EventSeq,
    ) -> RuntimeResult<()> {
        self.policy_port_called.store(true, Ordering::SeqCst);
        let mut captured = self.captured_decision.lock().map_err(|_| {
            RuntimeError::Core {
                source: Box::new(vb_core::errors::CoreError::InternalInvariantViolation {
                    reason: "captured_decision mutex poisoned",
                }),
            }
        })?;
        *captured = Some(decision);
        Err(RuntimeError::UnsupportedOperation {
            operation: "test_policy_decision_failure",
        })
    }
}