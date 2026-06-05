#![forbid(unsafe_code)]
//! Runtime facade shard-control helpers.

use crate::runtime::Runtime;
use crate::shard::{ShardCommand, ShardDirective};
use crate::{RuntimeError, RuntimeResult};
use vb_core::ids::RunId;

/// Summary of an active run on a shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRunSummary {
    /// Run identifier.
    pub run_id: RunId,
    /// Compiled workflow digest.
    pub workflow: vb_core::WorkflowDigest,
    /// Number of workflow steps.
    pub step_count: u16,
    /// Count of steps in a terminal per-step state.
    pub steps_completed: u16,
}

impl Runtime {
    /// Processes one directive on a selected shard.
    pub fn tick_shard(&mut self, shard: u32, directive: ShardDirective) -> RuntimeResult<bool> {
        let source = self.checked_shard_index(shard)?;
        match directive {
            ShardDirective::Continue => self.tick_existing_shard(source),
            ShardDirective::Suspend | ShardDirective::Barrier => self.shard_alive(source),
            ShardDirective::Cancel => self.cancel_selected_shard(source),
            ShardDirective::Migrate { target } => self.migrate_selected_shard(source, target),
            ShardDirective::Shutdown => self.shutdown_selected_shard(source),
        }
    }

    /// Lists active run summaries across all shards, up to `limit` entries.
    pub fn list_active_runs(
        &self,
        limit: u32,
        workflow_filter: Option<vb_core::WorkflowDigest>,
    ) -> Vec<ActiveRunSummary> {
        let max = usize::try_from(limit).unwrap_or(usize::MAX);
        let mut summaries = Vec::new();
        for shard in &self.shards {
            collect_shard_summaries(shard, max, workflow_filter, &mut summaries);
            if summaries.len() >= max {
                break;
            }
        }
        summaries.sort_by_key(|summary| summary.run_id);
        summaries.truncate(max);
        summaries
    }

    fn checked_shard_index(&self, shard: u32) -> RuntimeResult<usize> {
        let index = usize::try_from(shard).map_err(|_| RuntimeError::ShardNotFound { shard })?;
        if index < self.shards.len() {
            Ok(index)
        } else {
            Err(RuntimeError::ShardNotFound { shard })
        }
    }

    fn tick_existing_shard(&mut self, source: usize) -> RuntimeResult<bool> {
        let Some(shard) = self.shards.get_mut(source) else {
            return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
        };
        shard.tick()
    }

    fn shard_alive(&self, source: usize) -> RuntimeResult<bool> {
        let Some(shard) = self.shards.get(source) else {
            return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
        };
        Ok(!shard.is_shutting_down())
    }

    fn cancel_selected_shard(&mut self, source: usize) -> RuntimeResult<bool> {
        let Some(shard) = self.shards.get_mut(source) else {
            return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
        };
        let runs: Vec<RunId> = shard.runs.keys().copied().collect();
        for run in runs {
            shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
        }
        shard.tick()
    }

    fn migrate_selected_shard(&mut self, source: usize, target: u32) -> RuntimeResult<bool> {
        let target_index = self.checked_shard_index(target)?;
        if source == target_index {
            return Err(RuntimeError::MigrateSelf);
        }
        let commands = self.drain_source_commands(source)?;
        self.enqueue_migrated_commands(target_index, commands)?;
        self.source_has_work(source)
    }

    fn drain_source_commands(&self, source: usize) -> RuntimeResult<Vec<ShardCommand>> {
        let Some(shard) = self.shards.get(source) else {
            return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
        };
        let limit = shard.command_queue_len();
        let mut commands = Vec::with_capacity(limit);
        let mut drained = 0usize;
        while drained < limit {
            let Some(command) = shard.command_queue.pop() else {
                break;
            };
            commands.push(command);
            drained = drained.saturating_add(1);
        }
        Ok(commands)
    }

    fn enqueue_migrated_commands(
        &self,
        target: usize,
        commands: Vec<ShardCommand>,
    ) -> RuntimeResult<()> {
        let Some(shard) = self.shards.get(target) else {
            return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
        };
        for command in commands {
            shard.enqueue(command)?;
        }
        Ok(())
    }

    fn source_has_work(&self, source: usize) -> RuntimeResult<bool> {
        let Some(shard) = self.shards.get(source) else {
            return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
        };
        Ok(shard.active_run_count() > 0 || shard.command_queue_len() > 0)
    }

    fn shutdown_selected_shard(&mut self, source: usize) -> RuntimeResult<bool> {
        let Some(shard) = self.shards.get_mut(source) else {
            return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
        };
        shard.enqueue(ShardCommand::Shutdown)?;
        shard.drain_for_shutdown()?;
        self.journal.drain_for_shutdown()?;
        Ok(false)
    }
}

fn collect_shard_summaries(
    shard: &crate::shard::Shard,
    max: usize,
    workflow_filter: Option<vb_core::WorkflowDigest>,
    summaries: &mut Vec<ActiveRunSummary>,
) {
    let mut index = 0usize;
    while index < shard.runs.len() && summaries.len() < max {
        collect_one_summary(shard, index, workflow_filter, summaries);
        index = index.saturating_add(1);
    }
}

fn collect_one_summary(
    shard: &crate::shard::Shard,
    index: usize,
    workflow_filter: Option<vb_core::WorkflowDigest>,
    summaries: &mut Vec<ActiveRunSummary>,
) {
    let Some((run_id, state)) = shard.runs.get_index(index) else {
        return;
    };
    let digest = state.workflow.digest();
    if workflow_filter.is_some_and(|filter| digest != filter) {
        return;
    }
    summaries.push(ActiveRunSummary {
        run_id: *run_id,
        workflow: digest,
        step_count: state.workflow.node_count(),
        steps_completed: completed_steps(state),
    });
}

fn completed_steps(state: &crate::shard::RunState) -> u16 {
    let mut completed = 0u16;
    let mut step_index = 0u16;
    while step_index < state.workflow.node_count() {
        let step = vb_core::ids::StepIdx::new(step_index);
        if matches!(
            state.frame.step_state(step),
            Ok(vb_core::frame::StepState::Succeeded)
                | Ok(vb_core::frame::StepState::Failed)
                | Ok(vb_core::frame::StepState::Skipped)
                | Ok(vb_core::frame::StepState::Cancelled)
        ) {
            completed = completed.saturating_add(1);
        }
        step_index = step_index.saturating_add(1);
    }
    completed
}
