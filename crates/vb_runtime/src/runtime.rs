#![forbid(unsafe_code)]
//! Multi-shard runtime routing commands to correct shards.

#[path = "runtime_actions.rs"]
mod actions;
#[path = "runtime_metrics.rs"]
mod metrics;
#[path = "runtime_routing.rs"]
mod routing;
#[path = "runtime_scheduling.rs"]
mod scheduling;

#[cfg(kani)]
pub use actions::{AskTicketDerivation, kani_derive_ask_ticket_from_parts};

use std::num::NonZeroUsize;
use vb_core::action::ActionContract;
use vb_core::capability::CapabilitySet;
use vb_core::ids::{RunId, SlotIdx};
use vb_core::value::SlotValue;
use vb_core::workflow::CompiledWorkflow;

use crate::RuntimeResult;
use crate::journal::SharedRuntimeJournal;
use crate::shard::{Shard, ShardCommand, ShardConfig};

/// Summary of an active run on a shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRunSummary {
    /// Run identifier.
    pub run_id: RunId,
    /// Compiled workflow digest.
    pub workflow: vb_core::WorkflowDigest,
    /// Number of steps in the workflow.
    pub step_count: u16,
    /// Steps that reached a terminal state (Succeeded, Failed, Skipped, or Cancelled).
    pub steps_completed: u16,
}

/// Multi-shard runtime.
pub struct Runtime {
    shards: Vec<Shard>,
    shard_count: usize,
    journal: SharedRuntimeJournal,
}

impl Runtime {
    /// Creates a new runtime with the given number of shards and per-shard configuration.
    #[must_use]
    pub fn new(shard_count: NonZeroUsize, config: ShardConfig) -> Self {
        Self::new_with_journal(
            shard_count,
            config,
            crate::journal::VolatileRuntimeJournal::shared(),
        )
    }

    /// Creates a new runtime with an explicit runtime journal sink.
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn new_with_journal(
        shard_count: NonZeroUsize,
        config: ShardConfig,
        journal: SharedRuntimeJournal,
    ) -> Self {
        let count = shard_count.get();
        let shards = (0..count)
            .map(|_| Shard::new_with_journal(config, journal.clone()))
            .collect();
        Self {
            shards,
            shard_count: count,
            journal,
        }
    }

    /// Submits a run using a compiled workflow.
    pub fn submit_direct(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
        self.submit_direct_with_grants(run, workflow, CapabilitySet::empty())
    }

    /// Submits a run using a compiled workflow and explicit caller grants.
    pub fn submit_direct_with_grants(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.validate_submit_admission(run, workflow.digest(), caps.clone())?;
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps,
        })
    }

    /// Submits a run with explicit caller grants and validated action contracts.
    pub fn submit_direct_with_grants_and_contracts(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
        action_contracts: Box<[ActionContract]>,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.validate_submit_admission(run, workflow.digest(), caps.clone())?;
        shard.enqueue(ShardCommand::SubmitWithContracts {
            run,
            workflow,
            caps,
            action_contracts,
        })
    }

    /// Submits a run with pre-mapped input slots, explicit caller grants, and validated action contracts.
    pub fn submit_direct_with_inputs_grants_and_contracts(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: Box<[(SlotIdx, SlotValue)]>,
        caps: CapabilitySet,
        action_contracts: Box<[ActionContract]>,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.validate_submit_admission(run, workflow.digest(), caps.clone())?;
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            inputs,
            caps,
            action_contracts,
        })
    }

    /// Submits a run with inline workflow (same as submit_direct for now).
    pub fn submit_compiled(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
        self.submit_direct(run, workflow)
    }

    /// Submits a compiled run with explicit caller grants.
    pub fn submit_compiled_with_grants(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.submit_direct_with_grants(run, workflow, caps)
    }

    /// Submits a run with pre-mapped runtime input slots.
    pub fn submit_compiled_with_inputs(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: Box<[(SlotIdx, SlotValue)]>,
    ) -> RuntimeResult<()> {
        self.submit_compiled_with_inputs_and_grants(run, workflow, inputs, CapabilitySet::empty())
    }

    /// Submits a run with pre-mapped runtime input slots and explicit caller grants.
    pub fn submit_compiled_with_inputs_and_grants(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: Box<[(SlotIdx, SlotValue)]>,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.validate_submit_admission(run, workflow.digest(), caps.clone())?;
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run,
            workflow,
            inputs,
            caps,
        })
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
