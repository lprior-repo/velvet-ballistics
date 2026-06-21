#![forbid(unsafe_code)]
//! FSM routing for `RuntimeEvent` mutations on `Shard`.
//!
//! This module provides `Shard::apply`, the single routing method for all
//! runtime state mutations, replacing direct `insert`/`swap_remove` call sites.

use vb_core::ids::RunId;

use crate::shard::types::{RuntimeEvent, RuntimeState, Shard};
use crate::{RuntimeError, RuntimeResult};

impl Shard {
    /// Applies a RuntimeEvent to mutate runtime_states.
    ///
    /// This is the single routing method for all runtime_states mutations,
    /// replacing direct insert/swap_remove call sites.
    ///
    /// # Arguments
    /// * `run` - The run identifier
    /// * `event` - The runtime event variant
    ///
    /// # Returns
    /// `Err(RuntimeError::NotResumable)` if `event == Resume` and the prior
    /// runtime state is not `Resumable`. Other events do not require prior
    /// state validation and always succeed.
    ///
    /// # State Transitions
    /// * `Submit` → `runtime_states.insert(run, RuntimeState::Initial)`
    /// * `Resume` → `runtime_states.insert(run, RuntimeState::Resuming)` (requires prior state == Resumable)
    /// * `ResumeRollback` → `runtime_states.insert(run, RuntimeState::Resumable)` (journal failure)
    /// * `DriveContinue` → `runtime_states.insert(run, RuntimeState::Running)`
    /// * `AwaitAction` → `runtime_states.insert(run, RuntimeState::Resumable)`
    /// * `AwaitTimer` → `runtime_states.insert(run, RuntimeState::Resumable)`
    /// * `Fail` → `runtime_states.insert(run, RuntimeState::Failed)`
    /// * `TerminalRemove` → `runtime_states.swap_remove(&run)`
    /// * `DriveFinished` → `runtime_states.swap_remove(&run)`
    ///
    /// # Flux refinement (PO-vb282my-RS-FLUX-001):
    /// RuntimeState FSM contract:
    /// - `Resume` transition requires `runtime_states[run] == Resumable`
    /// - `ResumeRollback` transition ensures `runtime_states[run] == Resumable`
    /// - `Running` state rejects repeated `Resume` transitions
    ///
    /// Flux signature (requires flux-rs toolchain):
    /// ```flux
    /// #[flux_rs::sig(fn(&mut Shard, run: RunId, event: RuntimeEvent)
    ///     requires event == Resume => runtime_states[run] == Resumable,
    ///     ensures event == ResumeRollback => runtime_states[run] == Resumable
    /// )]
    /// ```
    pub(crate) fn apply(&mut self, run: RunId, event: RuntimeEvent) -> RuntimeResult<()> {
        match event {
            RuntimeEvent::Submit => {
                self.runtime_state_insert(run, RuntimeState::Initial);
            }
            RuntimeEvent::Resume => {
                let prior = self.runtime_state_get(run).unwrap_or(RuntimeState::Initial);
                if prior != RuntimeState::Resumable {
                    return Err(RuntimeError::NotResumable {
                        run,
                        current_state: prior,
                    });
                }
                self.runtime_state_insert(run, RuntimeState::Resuming);
            }
            RuntimeEvent::ResumeRollback => {
                // Journal append failed during resume, revert to Resumable
                self.runtime_state_insert(run, RuntimeState::Resumable);
            }
            RuntimeEvent::DriveContinue => {
                self.runtime_state_insert(run, RuntimeState::Running);
            }
            RuntimeEvent::AwaitAction | RuntimeEvent::AwaitTimer => {
                self.runtime_state_insert(run, RuntimeState::Resumable);
            }
            RuntimeEvent::Fail => {
                self.runtime_state_insert(run, RuntimeState::Failed);
            }
            RuntimeEvent::TerminalRemove | RuntimeEvent::DriveFinished => {
                self.runtime_states.swap_remove(&run);
            }
        }
        Ok(())
    }
}
