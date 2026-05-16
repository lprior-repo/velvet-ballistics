extern crate alloc;
use alloc::string::String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceError {
    MissingBaseline,
    MissingResult,
    MissingEnvironment,
    MissingCommand,
    MissingCommit,
    RegressionDetected {
        benchmark: String,
        delta: u64,
    },
    EmptyBudget,
}

impl core::fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EvidenceError::MissingBaseline => write!(f, "missing baseline measurement"),
            EvidenceError::MissingResult => write!(f, "missing result measurement"),
            EvidenceError::MissingEnvironment => write!(f, "missing environment"),
            EvidenceError::MissingCommand => write!(f, "missing command"),
            EvidenceError::MissingCommit => write!(f, "missing commit hash"),
            EvidenceError::RegressionDetected { benchmark, delta } => {
                write!(f, "regression detected: {benchmark} delta={delta}")
            }
            EvidenceError::EmptyBudget => write!(f, "budget not configured"),
        }
    }
}

impl core::error::Error for EvidenceError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YamlBenchmarkError {
    ParseFailure(String),
    ValidationFailure(String),
}

impl core::fmt::Display for YamlBenchmarkError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            YamlBenchmarkError::ParseFailure(inner) => write!(f, "YAML parse failed: {inner}"),
            YamlBenchmarkError::ValidationFailure(inner) => {
                write!(f, "workflow validation failed: {inner}")
            }
        }
    }
}

impl core::error::Error for YamlBenchmarkError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageBenchmarkError {
    JournalOpenFailure(String),
    AppendFailure(String),
}

impl core::fmt::Display for StorageBenchmarkError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StorageBenchmarkError::JournalOpenFailure(inner) => {
                write!(f, "journal open failed: {inner}")
            }
            StorageBenchmarkError::AppendFailure(inner) => {
                write!(f, "journal append failed: {inner}")
            }
        }
    }
}

impl core::error::Error for StorageBenchmarkError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcBenchmarkError {
    EncodeFailure(String),
    DecodeFailure(String),
}

impl core::fmt::Display for IpcBenchmarkError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IpcBenchmarkError::EncodeFailure(inner) => write!(f, "frame encode failed: {inner}"),
            IpcBenchmarkError::DecodeFailure(inner) => write!(f, "frame decode failed: {inner}"),
        }
    }
}

impl core::error::Error for IpcBenchmarkError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryBenchmarkError {
    HydrationFailure(String),
}

impl core::fmt::Display for RecoveryBenchmarkError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RecoveryBenchmarkError::HydrationFailure(inner) => {
                write!(f, "recovery hydration failed: {inner}")
            }
        }
    }
}

impl core::error::Error for RecoveryBenchmarkError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeBenchmarkError {
    StepFailure(String),
    PrimitiveFailure(String),
}

impl core::fmt::Display for RuntimeBenchmarkError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RuntimeBenchmarkError::StepFailure(inner) => write!(f, "runtime step failed: {inner}"),
            RuntimeBenchmarkError::PrimitiveFailure(inner) => {
                write!(f, "runtime primitive failed: {inner}")
            }
        }
    }
}

impl core::error::Error for RuntimeBenchmarkError {}
