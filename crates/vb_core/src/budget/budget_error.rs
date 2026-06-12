#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

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
}
