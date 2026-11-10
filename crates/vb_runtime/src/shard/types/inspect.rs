/// Origin determines whether registered production effects may execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisteredExecutionMode {
    /// A newly admitted live run can execute with an exact issued grant.
    Live,
    /// Replayed runs may only reconcile existing receipts.
    ReconcileOnly,
}

/// Mutable run state owned directly by the shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunState {
    /// Replayed state never authorizes a new registered production effect.
    pub registered_execution: RegisteredExecutionMode,
    /// Active run frame.
    pub frame: RunFrame,
    /// Current durable run attempt. Attempt one is used only for newly admitted runs.
    pub run_attempt: NonZeroU16,
    /// Compiled workflow for this run.
    pub workflow: CompiledWorkflow,
    /// Cold value store for list, object, and blob handles.
    pub store: ValueStore,
    /// Per-Do-step attempt counters owned with the live frame.
    pub action_attempts: Box<[u16]>,
    /// Admission record for this run, if admission gating was performed.
    pub admission: Option<crate::admission::RunAdmission>,
    /// Wall-clock run start in milliseconds since the UNIX epoch, recorded at submit.
    /// None for states built before a start was recorded; the drive check records
    /// the current wall clock on first drive in that case.
    pub run_started_millis: Option<u64>,
    /// Per-run collect pagination state side table.
    pub collect_states: CollectStates,
    /// Cumulative journal bytes charged before append, carried in checkpoints.
    pub journal_bytes_written: u64,
    /// Validated action contracts used by Do execution.
    pub action_contracts: Box<[ActionContract]>,
}

/// Diagnostic snapshot returned by the Inspect command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectSnapshot {
    /// Run identifier.
    pub run: RunId,
    /// Caller correlation identifier.
    pub correlation: u64,
    /// Current program counter.
    pub pc: StepIdx,
    /// Number of executed transitions.
    pub executed: u64,
}

/// Bounded response produced by an inspect command.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InspectResponse {
    /// The run was active and a snapshot was captured.
    Found(InspectSnapshot),
    /// The run was not active on this shard.
    NotFound {
        /// Run identifier.
        run: RunId,
        /// Caller correlation identifier.
        correlation: u64,
    },
}

// ============================================================================
// RAII Introspection Registry
// ============================================================================

/// Outcome of an unregister operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnregisterOutcome {
    /// The handle was successfully unregistered.
    Unregistered,
    /// The handle was not found (no-op).
    Missing,
}

/// Outcome of a register operation when overlap is detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterOverlapOutcome {
    /// Registration was rejected due to conflict with existing registration.
    Conflict,
    /// Registration replaced the existing one with a new epoch.
    Replaced {
        /// The epoch of the replaced registration.
        old_epoch: u64,
        /// The epoch of the new registration.
        new_epoch: u64,
    },
}

/// Epoch-based handle for an introspection registration.
///
/// When dropped, the handle is automatically unregistered from the registry.
#[derive(Debug)]
pub struct InspectHandle {
    run: RunId,
    epoch: u64,
    registry: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<RunId, u64>>>,
}

impl InspectHandle {
    /// Returns the run identifier associated with this handle.
    #[must_use]
    pub fn run(&self) -> RunId {
        self.run
    }

    /// Returns the epoch of this handle.
    #[must_use]
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

impl Drop for InspectHandle {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.registry.lock() {
            // Only remove if the epoch matches (handles stale drops correctly)
            if let Some(current_epoch) = guard.get(&self.run)
                && *current_epoch == self.epoch
            {
                guard.remove(&self.run);
            }
        }
    }
}
