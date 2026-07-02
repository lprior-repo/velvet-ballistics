#![forbid(unsafe_code)]

//! Public type definitions for the fault injection engine.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use vb_core::ids::{ActionId, RunId, StepIdx};

// ---------------------------------------------------------------------------
// Boundary surface
// ---------------------------------------------------------------------------

/// Slot relative to an action invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundarySlot {
    Before,
    After,
}

/// Crash severity applied at a `NamedBoundary`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrashSeverity {
    SoftPanic,
    HardKill,
}

/// Typed failure code attached to action failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    Permission,
    Network,
    SchemaMismatch,
    Internal,
}

/// Monotonic checkpoint sequence used by the restart boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct CheckpointSeq(pub u32);

/// A named runtime/journal boundary where faults may be injected.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NamedBoundary {
    RuntimeBeforeAppend {
        run: RunId,
        step: StepIdx,
    },
    RuntimeAfterAppend {
        run: RunId,
        step: StepIdx,
    },
    StorageAppendStart {
        partition: u8,
    },
    StorageAppendMid {
        partition: u8,
    },
    StorageAppendCommit {
        partition: u8,
    },
    ActionAction {
        action: ActionId,
        slot: BoundarySlot,
    },
    ActionFail {
        action: ActionId,
        slot: BoundarySlot,
    },
    AskTimeout {
        step: StepIdx,
    },
    TimerFire {
        run: RunId,
        generation: u32,
    },
    Restart {
        checkpoint: CheckpointSeq,
    },
}

impl NamedBoundary {
    /// Short, stable, human-readable label used in journal outcomes.
    ///
    /// Two boundaries that compare equal produce identical labels.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::RuntimeBeforeAppend { run, step } => {
                format!("runtime_before_append/run{}/step{}", run.get(), step.get())
            }
            Self::RuntimeAfterAppend { run, step } => {
                format!("runtime_after_append/run{}/step{}", run.get(), step.get())
            }
            Self::StorageAppendStart { partition } => {
                format!("storage_append_start/part{partition}")
            }
            Self::StorageAppendMid { partition } => {
                format!("storage_append_mid/part{partition}")
            }
            Self::StorageAppendCommit { partition } => {
                format!("storage_append_commit/part{partition}")
            }
            Self::ActionAction { action, slot } => {
                let slot = match slot {
                    BoundarySlot::Before => "before",
                    BoundarySlot::After => "after",
                };
                format!("action/action{action}/{slot}", action = action.get())
            }
            Self::ActionFail { action, slot } => {
                let slot = match slot {
                    BoundarySlot::Before => "before",
                    BoundarySlot::After => "after",
                };
                format!("action_fail/action{action}/{slot}", action = action.get())
            }
            Self::AskTimeout { step } => format!("ask_timeout/step{}", step.get()),
            Self::TimerFire { run, generation } => {
                format!("timer_fire/run{}/gen{generation}", run.get())
            }
            Self::Restart { checkpoint } => format!("restart/checkpoint{}", checkpoint.0),
        }
    }

    /// Return the boundary implied by a `FaultEvent` (the boundary the
    /// event targets), or `None` for fault events that do not target a
    /// single named boundary.
    #[must_use]
    pub fn for_fault(event: &FaultEvent) -> Option<Self> {
        match event {
            FaultEvent::Crash { boundary, .. }
            | FaultEvent::AppendFailure { boundary, .. }
            | FaultEvent::LockContention { boundary, .. } => Some(boundary.clone()),
            FaultEvent::ActionFailure { .. } | FaultEvent::Timeout { .. } => None,
            FaultEvent::Restart { checkpoint } => Some(Self::Restart {
                checkpoint: *checkpoint,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Fault events
// ---------------------------------------------------------------------------

/// A single fault event injected at a named boundary (or globally).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FaultEvent {
    Crash {
        boundary: NamedBoundary,
        severity: CrashSeverity,
    },
    AppendFailure {
        boundary: NamedBoundary,
        transient: bool,
    },
    LockContention {
        boundary: NamedBoundary,
        retry_count: u8,
    },
    ActionFailure {
        action: ActionId,
        code: FailureCode,
    },
    Timeout {
        step: StepIdx,
        delay_ticks: u32,
    },
    Restart {
        checkpoint: CheckpointSeq,
    },
}

impl FaultEvent {
    /// Whether this event targets a single boundary passage that produces a
    /// journal entry.
    #[must_use]
    pub fn writes_journal(&self) -> bool {
        matches!(
            self,
            FaultEvent::Crash { .. }
                | FaultEvent::AppendFailure { .. }
                | FaultEvent::LockContention { .. }
        )
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for a fault injection run.
///
/// `boundaries` declares the set of named boundary passages the runtime
/// walks through during the simulated run. `fault_schedule` is the ordered
/// list of faults to overlay onto those passages. `max_faults` and
/// `max_runtime_steps` are hard upper bounds the engine refuses to exceed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FaultConfig {
    pub seed: u64,
    pub boundaries: Vec<NamedBoundary>,
    pub fault_schedule: Vec<FaultEvent>,
    pub max_faults: u32,
    pub max_runtime_steps: u32,
}

impl FaultConfig {
    /// Construct a new `FaultConfig` with sensible defaults for the
    /// runtime/engine budget fields. The caller still supplies the seed,
    /// boundaries, and schedule.
    #[must_use]
    pub fn new(seed: u64, boundaries: Vec<NamedBoundary>, fault_schedule: Vec<FaultEvent>) -> Self {
        Self {
            seed,
            boundaries,
            fault_schedule,
            max_faults: 1024,
            max_runtime_steps: 4096,
        }
    }

    /// Override `max_faults`. Builder-style helper.
    #[must_use]
    pub fn with_max_faults(mut self, max_faults: u32) -> Self {
        self.max_faults = max_faults;
        self
    }

    /// Override `max_runtime_steps`. Builder-style helper.
    #[must_use]
    pub fn with_max_runtime_steps(mut self, max_runtime_steps: u32) -> Self {
        self.max_runtime_steps = max_runtime_steps;
        self
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Resource budget that was exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetKind {
    Faults,
    RuntimeSteps,
}

/// Errors returned by [`super::run_fault_injection`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FaultError {
    #[error("budget exceeded for {budget_kind:?}: observed {observed}, limit {limit}")]
    BudgetExceeded {
        budget_kind: BudgetKind,
        observed: u32,
        limit: u32,
    },
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}
