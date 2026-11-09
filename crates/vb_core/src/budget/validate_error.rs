#![forbid(unsafe_code)]
//! Budget validation failures: the budget error taxonomy and the
//! dimension classification backing admission diagnostics.

use thiserror::Error;

/// Budget validation failures.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BudgetError {
    #[error("total steps exceeded: {actual} > {limit}")]
    TotalStepsExceeded { actual: u64, limit: u64 },
    #[error("total slots exceeded: {actual} > {limit}")]
    TotalSlotsExceeded { actual: u64, limit: u64 },
    #[error("fanout exceeded: {actual} > {limit}")]
    FanoutExceeded { actual: u16, limit: u16 },
    #[error("nesting depth exceeded: {actual} > {limit}")]
    NestingDepthExceeded { actual: u16, limit: u16 },
    #[error("parallel exceeded: {actual} > {limit}")]
    ParallelExceeded { actual: u16, limit: u16 },
    #[error("action tickets exceeded: {actual} > {limit}")]
    ActionTicketsExceeded { actual: u32, limit: u32 },
    #[error("run time exceeded: {actual} > {limit}")]
    RunTimeExceeded { actual: u64, limit: u64 },
    #[error("result bytes exceeded: {actual} > {limit}")]
    ResultBytesExceeded { actual: u32, limit: u32 },
    #[error("steps executable exceeded: {actual} > {limit}")]
    StepsExecutableExceeded { actual: u32, limit: u32 },
    #[error("timer entries exceeded: {actual} > {limit}")]
    TimerEntriesExceeded { actual: u32, limit: u32 },
    #[error("trace events exceeded: {actual} > {limit}")]
    TraceEventsExceeded { actual: u64, limit: u64 },
    #[error("journal batch bytes exceeded: {actual} > {limit}")]
    JournalBatchBytesExceeded { actual: u32, limit: u32 },
    #[error("queue depth exceeded: {actual} > {limit}")]
    QueueDepthExceeded { actual: u32, limit: u32 },
    #[error("ipc payload bytes exceeded: {actual} > {limit}")]
    IpcPayloadBytesExceeded { actual: u32, limit: u32 },
    #[error("blob bytes exceeded: {actual} > {limit}")]
    BlobBytesExceeded { actual: u64, limit: u64 },
    #[error("input bytes exceeded: {actual} > {limit}")]
    InputBytesExceeded { actual: u32, limit: u32 },
    #[error("open gates exceeded: {actual} > {limit}")]
    OpenGatesExceeded { actual: u32, limit: u32 },
    #[error("signal subscriptions exceeded: {actual} > {limit}")]
    SignalSubscriptionsExceeded { actual: u32, limit: u32 },
    #[error("incident emissions exceeded: {actual} > {limit}")]
    IncidentEmissionsExceeded { actual: u64, limit: u64 },
    #[error("transitions exceeded: {actual} > {limit}")]
    TransitionsExceeded { actual: u64, limit: u64 },
    #[error("cumulative journal bytes exceeded: {actual} > {limit}")]
    JournalBytesTotalExceeded { actual: u64, limit: u64 },
    #[error("value-store entries exceeded: {actual} > {limit}")]
    ValueStoreEntriesExceeded { actual: u64, limit: u64 },
    #[error("cost units exceeded: {actual} > {limit}")]
    CostUnitsExceeded { actual: u64, limit: u64 },
}

impl BudgetError {
    /// Classifies which budget dimension failed its policy bound.
    ///
    /// The mapping is exhaustive: adding a variant here fails compilation
    /// until its dimension is classified.
    #[must_use]
    pub const fn dimension(&self) -> BudgetDimension {
        match self {
            Self::TotalStepsExceeded { .. } => BudgetDimension::TotalSteps,
            Self::TotalSlotsExceeded { .. } => BudgetDimension::TotalSlots,
            Self::FanoutExceeded { .. } => BudgetDimension::Fanout,
            Self::NestingDepthExceeded { .. } => BudgetDimension::NestingDepth,
            Self::ParallelExceeded { .. } => BudgetDimension::Parallel,
            Self::ActionTicketsExceeded { .. } => BudgetDimension::ActionTickets,
            Self::RunTimeExceeded { .. } => BudgetDimension::RunTime,
            Self::ResultBytesExceeded { .. } => BudgetDimension::ResultBytes,
            Self::StepsExecutableExceeded { .. } => BudgetDimension::StepsExecutable,
            Self::TimerEntriesExceeded { .. } => BudgetDimension::TimerEntries,
            Self::TraceEventsExceeded { .. } => BudgetDimension::TraceEvents,
            Self::JournalBatchBytesExceeded { .. } => BudgetDimension::JournalBatchBytes,
            Self::QueueDepthExceeded { .. } => BudgetDimension::QueueDepth,
            Self::IpcPayloadBytesExceeded { .. } => BudgetDimension::IpcPayloadBytes,
            Self::BlobBytesExceeded { .. } => BudgetDimension::BlobBytes,
            Self::InputBytesExceeded { .. } => BudgetDimension::InputBytes,
            Self::OpenGatesExceeded { .. } => BudgetDimension::OpenGates,
            Self::SignalSubscriptionsExceeded { .. } => BudgetDimension::SignalSubscriptions,
            Self::IncidentEmissionsExceeded { .. } => BudgetDimension::IncidentEmissions,
            Self::TransitionsExceeded { .. } => BudgetDimension::Transitions,
            Self::JournalBytesTotalExceeded { .. } => BudgetDimension::JournalBytesTotal,
            Self::ValueStoreEntriesExceeded { .. } => BudgetDimension::ValueStoreEntries,
            Self::CostUnitsExceeded { .. } => BudgetDimension::CostUnits,
        }
    }
}

/// Budget dimension that exceeded its policy bound.
///
/// Canonical classification backing `WorkflowError::BudgetPolicyExceeded`.
/// The canonical names are frozen: diagnostics and admission mapping
/// match on them, so renaming a string is a behavior change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetDimension {
    /// Total step count bound.
    TotalSteps,
    /// Total slot count bound.
    TotalSlots,
    /// Branch fanout bound.
    Fanout,
    /// Structural nesting bound.
    NestingDepth,
    /// Parallel in-flight bound.
    Parallel,
    /// Action ticket bound.
    ActionTickets,
    /// Run time bound.
    RunTime,
    /// Result byte bound.
    ResultBytes,
    /// Executable step bound.
    StepsExecutable,
    /// Timer entry bound.
    TimerEntries,
    /// Trace event bound.
    TraceEvents,
    /// Journal batch byte bound.
    JournalBatchBytes,
    /// Queue depth bound.
    QueueDepth,
    /// IPC payload byte bound.
    IpcPayloadBytes,
    /// Blob byte bound.
    BlobBytes,
    /// Input byte bound.
    InputBytes,
    /// Open gate bound.
    OpenGates,
    /// Signal subscription bound.
    SignalSubscriptions,
    /// Incident emission bound.
    IncidentEmissions,
    /// Transition count bound.
    Transitions,
    /// Cumulative journal byte bound.
    JournalBytesTotal,
    /// Value-store entry bound.
    ValueStoreEntries,
    /// Cost unit bound.
    CostUnits,
}

impl BudgetDimension {
    /// Canonical dimension name. Frozen: diagnostics match on these strings.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TotalSteps => "max_total_steps",
            Self::TotalSlots => "max_total_slots",
            Self::Fanout => "max_fanout",
            Self::NestingDepth => "max_nesting_depth",
            Self::Parallel => "max_parallel_in_flight",
            Self::ActionTickets => "max_action_tickets",
            Self::RunTime => "max_run_time_seconds",
            Self::ResultBytes => "max_result_bytes",
            Self::StepsExecutable => "max_steps_executable",
            Self::TimerEntries => "max_timer_entries",
            Self::TraceEvents => "max_trace_events",
            Self::JournalBatchBytes => "max_journal_batch_bytes",
            Self::QueueDepth => "max_queue_depth",
            Self::IpcPayloadBytes => "max_ipc_payload_bytes",
            Self::BlobBytes => "max_blob_bytes",
            Self::InputBytes => "max_input_bytes",
            Self::OpenGates => "max_open_gates",
            Self::SignalSubscriptions => "max_signal_subscriptions",
            Self::IncidentEmissions => "max_incident_emissions",
            Self::Transitions => "max_transitions",
            Self::JournalBytesTotal => "max_journal_bytes_total",
            Self::ValueStoreEntries => "max_value_store_entries",
            Self::CostUnits => "max_cost_units",
        }
    }
}

impl core::fmt::Display for BudgetDimension {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}
