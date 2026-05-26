//! Property test: CoreError, RuntimeError, JournalError symbolic code registration.
//!
//! Compensates: BLOCKED PO-015.
//! Invariant: Every sampled variant maps to a SymbolicCode registered in CODE_REGISTRY.

use vb_core::diagnostic::{CODE_REGISTRY, HasSymbolicCode, SymbolicCode};
use vb_core::errors::CoreError;
use vb_core::ids::{SlotIdx, StepIdx};

#[test]
fn core_error_symbolic_codes_are_registered() {
    let errors: Vec<CoreError> = vec![
        CoreError::DivisionByZero,
        CoreError::NonFiniteNumber,
        CoreError::StepBudgetExhausted,
        CoreError::StepCounterOverflow,
        CoreError::QueueFull,
        CoreError::AllocationFailed,
        CoreError::ExpressionStackUnderflow,
        CoreError::CollectPageLimitExceeded,
        CoreError::CollectItemLimitExceeded,
        CoreError::CollectTimeLimitExceeded,
        CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        },
        CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(0),
        },
        CoreError::TypeMismatch {
            expected: "u64",
            found: "string",
        },
        CoreError::NonBoolCondition {
            slot: SlotIdx::new(0),
        },
        CoreError::ResourceLimitExceeded { resource: "cpu" },
        CoreError::ExpressionStackOverflow { max: 64 },
        CoreError::MissingOutputSlot {
            step: StepIdx::new(0),
        },
        CoreError::StepStateOutOfBounds {
            step: StepIdx::new(0),
        },
        CoreError::UnsupportedPrimitive { primitive: "wait" },
        CoreError::InternalInvariantViolation { reason: "test" },
        CoreError::UnsupportedAccessorTraversal {
            segment: "field",
            found: "map",
        },
        CoreError::IterationLimitExceeded { resource: "cpu" },
        CoreError::RepeatExhausted { max: 3 },
        CoreError::TogetherBranchLimitExceeded { max: 1 },
        CoreError::BudgetExceeded {
            budget: "cpu",
            limit: 100,
        },
        CoreError::BudgetParse { reason: "invalid" },
        CoreError::ParallelLimitExceeded { limit: 1 },
        CoreError::InvalidCompiledWorkflow { reason: "test" },
        CoreError::ObjectFieldNotFound {
            field: vb_core::ids::SymbolId::new(0),
        },
        CoreError::ListIndexOutOfBounds { index: 999 },
        CoreError::SymbolOutOfBounds {
            symbol: vb_core::ids::SymbolId::new(0),
        },
        CoreError::ListOutOfBounds {
            list: vb_core::ids::ListId::new(0),
        },
        CoreError::ObjectOutOfBounds {
            object: vb_core::ids::ObjectId::new(0),
        },
        CoreError::BlobOutOfBounds {
            blob: vb_core::ids::BlobId::new(0),
        },
    ];

    for error in &errors {
        let code = error.symbolic_code();
        let reconstructed = SymbolicCode::from_static(code.as_str());
        assert!(
            reconstructed.is_some(),
            "CoreError symbolic code '{}' must be registered",
            code.as_str()
        );
        assert!(
            CODE_REGISTRY.iter().any(|e| e.symbolic == code.as_str()),
            "CoreError code '{}' must have a CODE_REGISTRY entry",
            code.as_str()
        );
    }
}

#[test]
fn runtime_error_symbolic_codes_are_registered() {
    use vb_runtime::RuntimeError;

    let errors: Vec<RuntimeError> = vec![
        RuntimeError::QueueFull,
        RuntimeError::RunNotFound,
        RuntimeError::RunAlreadyExists,
        RuntimeError::UnsupportedOperation { operation: "test" },
        RuntimeError::ShutdownInProgress,
        RuntimeError::JournalPoisoned,
        RuntimeError::UnsupportedAsyncStrictAck,
        RuntimeError::FramePoolUnavailable,
        RuntimeError::InvalidActionCompletion,
        RuntimeError::InvalidTimerFire,
        RuntimeError::UnsupportedFullRecoveryHydration,
        RuntimeError::InvalidRecoveryHydration,
        RuntimeError::ActiveRunCapacityZero,
        RuntimeError::EncodeFailed,
        RuntimeError::SecretResultNotAllowed,
        RuntimeError::MigrateSelf,
    ];

    for error in &errors {
        let code = error.symbolic_code();
        let reconstructed = SymbolicCode::from_static(code.as_str());
        assert!(
            reconstructed.is_some(),
            "RuntimeError symbolic code '{}' must be registered",
            code.as_str()
        );
        assert!(
            CODE_REGISTRY.iter().any(|e| e.symbolic == code.as_str()),
            "RuntimeError code '{}' must have a CODE_REGISTRY entry",
            code.as_str()
        );
    }
}

#[test]
fn journal_error_symbolic_codes_are_registered() {
    use vb_storage::JournalError;

    let errors: Vec<JournalError> = vec![
        JournalError::KeyCapacity,
        JournalError::QueueFull,
        JournalError::WriteLockPoisoned,
        JournalError::UnexpectedEof,
        JournalError::PostcardDecodeFailed,
        JournalError::QueueShutdown,
        JournalError::ArtifactMalformed,
        JournalError::ArtifactChecksumMismatch,
        JournalError::InvalidEvent,
        JournalError::AdmissionRequired,
        JournalError::InputSchemaMismatch,
        JournalError::CapabilityDenied,
        JournalError::SecretUnavailable,
        JournalError::RunAlreadyExists,
        JournalError::ActiveRunCapacityExceeded,
        JournalError::FrameAllocationFailed,
        JournalError::AdmissionJournalFailed,
        JournalError::StrictDurabilityFailed,
        JournalError::ClockUnavailable,
    ];

    for error in &errors {
        let code = error.symbolic_code();
        let reconstructed = SymbolicCode::from_static(code.as_str());
        assert!(
            reconstructed.is_some(),
            "JournalError symbolic code '{}' must be registered",
            code.as_str()
        );
        assert!(
            CODE_REGISTRY.iter().any(|e| e.symbolic == code.as_str()),
            "JournalError code '{}' must have a CODE_REGISTRY entry",
            code.as_str()
        );
    }
}
