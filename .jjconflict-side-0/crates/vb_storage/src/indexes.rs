#![forbid(unsafe_code)]
//! Index storage operations.
//!
//! Provides storage of status, workflow, and action index markers.

use crate::{
    error::JournalError,
    keys::{index_action_key, index_status_key, index_workflow_key},
};

use crate::journal::FjallJournal;

impl FjallJournal {
    /// Inserts minimal status index marker bytes.
    pub fn put_status_index(
        &self,
        state: crate::types::IndexStatusState,
        timestamp: u64,
        run: vb_core::RunId,
    ) -> Result<(), JournalError> {
        let key = index_status_key(state, timestamp, run)?;
        self.index_status.insert(key.to_vec(), Vec::<u8>::new())?;
        Ok(())
    }

    /// Inserts minimal workflow index marker bytes.
    pub fn put_workflow_index(
        &self,
        workflow: vb_core::WorkflowId,
        run: vb_core::RunId,
    ) -> Result<(), JournalError> {
        let key = index_workflow_key(workflow, run)?;
        self.index_workflow.insert(key.to_vec(), Vec::<u8>::new())?;
        Ok(())
    }

    /// Inserts minimal pending action index marker bytes.
    pub fn put_action_index(
        &self,
        action: vb_core::ActionId,
        run: vb_core::RunId,
        step: vb_core::StepIdx,
    ) -> Result<(), JournalError> {
        let key = index_action_key(action, run, step)?;
        self.index_action.insert(key.to_vec(), Vec::<u8>::new())?;
        Ok(())
    }

    /// Removes the pending action index marker for the given triple.
    ///
    /// Idempotent: removing a non-existent key is a no-op (Fjall returns
    /// `Ok(())` for absent keys). Used to clear the index when an action
    /// reaches a terminal state (completed, failed, abandoned) so the
    /// runtime journal path leaves the `index_action` keyspace
    /// consistent with the durable event log.
    pub fn delete_action_index(
        &self,
        action: vb_core::ActionId,
        run: vb_core::RunId,
        step: vb_core::StepIdx,
    ) -> Result<(), JournalError> {
        let key = index_action_key(action, run, step)?;
        self.index_action.remove(key.as_slice())?;
        Ok(())
    }
}
