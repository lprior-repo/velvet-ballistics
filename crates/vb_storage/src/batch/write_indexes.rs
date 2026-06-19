//! Index marker staging methods for [`super::JournalWriteBatch`].

use super::JournalWriteBatch;
use crate::{
    error::JournalError,
    keys::{index_action_key, index_status_key, index_workflow_key},
};

impl<'j> JournalWriteBatch<'j> {
    /// Inserts a status index marker into the batch.
    pub fn put_status_index(
        &mut self,
        state: crate::types::IndexStatusState,
        timestamp: u64,
        run: vb_core::RunId,
    ) -> Result<(), JournalError> {
        let key = index_status_key(state, timestamp, run)?;
        self.inner
            .insert(&self.journal.index_status, key, Vec::<u8>::new());
        Ok(())
    }

    /// Inserts a workflow index marker into the batch.
    pub fn put_workflow_index(
        &mut self,
        workflow: vb_core::WorkflowId,
        run: vb_core::RunId,
    ) -> Result<(), JournalError> {
        let key = index_workflow_key(workflow, run)?;
        self.inner
            .insert(&self.journal.index_workflow, key, Vec::<u8>::new());
        Ok(())
    }

    /// Inserts an action index marker into the batch.
    pub fn put_action_index(
        &mut self,
        action: vb_core::ActionId,
        run: vb_core::RunId,
        step: vb_core::StepIdx,
    ) -> Result<(), JournalError> {
        let key = index_action_key(action, run, step)?;
        self.inner
            .insert(&self.journal.index_action, key, Vec::<u8>::new());
        Ok(())
    }
}
