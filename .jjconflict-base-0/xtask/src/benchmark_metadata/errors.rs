use super::{
    EvidenceError, IpcBenchmarkError, RecoveryBenchmarkError, RuntimeBenchmarkError,
    StorageBenchmarkError, YamlBenchmarkError,
};

impl std::fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl std::error::Error for EvidenceError {}

impl std::fmt::Display for YamlBenchmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YamlBenchmarkError::ParseFailure(inner) => write!(f, "YAML parse failed: {inner}"),
            YamlBenchmarkError::ValidationFailure(inner) => {
                write!(f, "workflow validation failed: {inner}")
            }
        }
    }
}

impl std::error::Error for YamlBenchmarkError {}

impl std::fmt::Display for StorageBenchmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl std::error::Error for StorageBenchmarkError {}

impl std::fmt::Display for IpcBenchmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpcBenchmarkError::EncodeFailure(inner) => write!(f, "frame encode failed: {inner}"),
            IpcBenchmarkError::DecodeFailure(inner) => write!(f, "frame decode failed: {inner}"),
        }
    }
}

impl std::error::Error for IpcBenchmarkError {}

impl std::fmt::Display for RecoveryBenchmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecoveryBenchmarkError::HydrationFailure(inner) => {
                write!(f, "recovery hydration failed: {inner}")
            }
        }
    }
}

impl std::error::Error for RecoveryBenchmarkError {}

impl std::fmt::Display for RuntimeBenchmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeBenchmarkError::StepFailure(inner) => write!(f, "runtime step failed: {inner}"),
            RuntimeBenchmarkError::PrimitiveFailure(inner) => {
                write!(f, "runtime primitive failed: {inner}")
            }
        }
    }
}

impl std::error::Error for RuntimeBenchmarkError {}
