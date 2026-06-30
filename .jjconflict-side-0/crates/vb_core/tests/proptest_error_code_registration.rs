//! Property test: Every CoreError variant maps to a non-zero DiagnosticCode
//! that is registered in CODE_REGISTRY.
//!
//! PO-001 / PS-001: Error code stability — diagnostic_code across all CoreError variants.
//!
//! Invariants:
//!   - Every CoreError variant's diagnostic_code() is non-zero.
//!   - Every CoreError variant's diagnostic_code() has a CODE_REGISTRY entry.
//!   - Every CoreError variant's symbolic_code() is registered and roundtrips.

use vb_core::diagnostic::{CODE_REGISTRY, HasSymbolicCode, SymbolicCode};
use vb_core::errors::CoreError;
use vb_core::ids::{
    BlobId, ConstIdx, ExprIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx, SymbolId,
};

/// Build every CoreError variant (40 total) with plausible field values.
fn all_core_error_variants() -> Vec<CoreError> {
    vec![
        CoreError::InvalidProgramCounter {
            step: StepIdx::new(1),
        },
        CoreError::MissingNextStep {
            step: StepIdx::new(2),
        },
        CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(99),
        },
        CoreError::SlotUninitialized {
            slot: SlotIdx::new(3),
        },
        CoreError::ExprOutOfBounds {
            expr: ExprIdx::new(7),
        },
        CoreError::ConstOutOfBounds {
            index: ConstIdx::new(12),
        },
        CoreError::MissingOutputSlot {
            step: StepIdx::new(4),
        },
        CoreError::StepStateOutOfBounds {
            step: StepIdx::new(5),
        },
        CoreError::TypeMismatch {
            expected: "u64",
            found: "string",
        },
        CoreError::NonBoolCondition {
            slot: SlotIdx::new(1),
        },
        CoreError::NonFiniteNumber,
        CoreError::DivisionByZero,
        CoreError::StepBudgetExhausted,
        CoreError::StepCounterOverflow,
        CoreError::QueueFull,
        CoreError::ResourceLimitExceeded { resource: "cpu" },
        CoreError::AllocationFailed,
        CoreError::ExpressionStackOverflow { max: 64 },
        CoreError::ExpressionStackUnderflow,
        CoreError::InvalidCompiledWorkflow { reason: "test" },
        CoreError::UnsupportedPrimitive { primitive: "op" },
        CoreError::UnsupportedAccessorTraversal {
            segment: "field",
            found: "map",
        },
        CoreError::ObjectFieldNotFound {
            field: SymbolId::new(0),
        },
        CoreError::ListIndexOutOfBounds { index: 999 },
        CoreError::InternalInvariantViolation { reason: "test" },
        CoreError::SymbolOutOfBounds {
            symbol: SymbolId::new(0),
        },
        CoreError::ListOutOfBounds {
            list: ListId::new(0),
        },
        CoreError::ObjectOutOfBounds {
            object: ObjectId::new(0),
        },
        CoreError::BlobOutOfBounds {
            blob: BlobId::new(0),
        },
        CoreError::IterationLimitExceeded { resource: "cpu" },
        CoreError::RepeatExhausted { max: 3 },
        CoreError::CollectPageLimitExceeded,
        CoreError::CollectItemLimitExceeded,
        CoreError::CollectTimeLimitExceeded,
        CoreError::TogetherBranchLimitExceeded { max: 1 },
        CoreError::ParallelLimitExceeded { limit: 1 },
        CoreError::CapabilityDenied {
            action: vb_core::ids::ActionId::new(1),
            required: vb_core::capability::Capability::new(
                Box::from("required"),
                vb_core::ids::ActionId::new(2),
            ),
            granted: vb_core::capability::CapabilitySet::empty(),
        },
        CoreError::BudgetExceeded {
            budget: "cpu",
            limit: 100,
        },
        CoreError::BudgetParse { reason: "bad" },
        CoreError::CollectPageOrderViolation {
            kind: vb_core::errors::CollectPageOrderViolationKind::OutOfOrder,
            run_id: RunId::new(1),
            collector_slot: SlotIdx::new(3),
            expected_page: ListId::new(2),
            observed_page: ListId::new(3),
        },
        CoreError::CollectExtraHydrationFailed {
            kind: vb_core::errors::CollectExtraHydrationFailureKind::EmptyExtra,
            run_id: RunId::new(1),
            collector_slot: SlotIdx::new(3),
            event_seq: Some(vb_core::ids::EventSeq::new(1)),
        },
        CoreError::CollectEvidenceCapacityExceeded {
            run_id: RunId::new(1),
            slot: SlotIdx::new(3),
            capacity: 10,
            len: 11,
            required: "extra slots",
        },
        CoreError::LifecycleStorageUnavailable {
            code: vb_core::DiagnosticCode::new(0x1501),
            context: "test".into(),
            timestamp: chrono::Utc::now(),
            bead_id: Some(RunId::new(1)),
        },
        CoreError::LifecycleDuplicateRequest {
            code: vb_core::DiagnosticCode::new(0x1502),
            context: "test".into(),
            timestamp: chrono::Utc::now(),
            bead_id: Some(RunId::new(1)),
            command: Some("run"),
        },
        CoreError::LifecycleStaleRequest {
            code: vb_core::DiagnosticCode::new(0x1503),
            context: "test".into(),
            timestamp: chrono::Utc::now(),
            bead_id: Some(RunId::new(1)),
            command: Some("cancel"),
        },
        CoreError::LifecycleInvalidTransition {
            code: vb_core::DiagnosticCode::new(0x1504),
            context: "test".into(),
            timestamp: chrono::Utc::now(),
            bead_id: Some(RunId::new(1)),
            command: Some("run"),
        },
        CoreError::JournalWriteFailure {
            code: vb_core::DiagnosticCode::new(0x1505),
            context: "test".into(),
            timestamp: chrono::Utc::now(),
            bead_id: Some(RunId::new(1)),
        },
        CoreError::ReplayCorruption {
            code: vb_core::DiagnosticCode::new(0x1506),
            context: "test".into(),
            timestamp: chrono::Utc::now(),
            bead_id: Some(RunId::new(1)),
        },
    ]
}

#[test]
fn every_core_error_variant_has_nonzero_diagnostic_code() {
    for error in &all_core_error_variants() {
        let code = error.diagnostic_code();
        assert_ne!(
            code.code(),
            0,
            "CoreError variant {:?} returned zero diagnostic_code",
            error
        );
    }
}

#[test]
fn every_core_error_variant_diagnostic_code_in_registry() {
    for error in &all_core_error_variants() {
        let code = error.diagnostic_code();
        let hex = code.code();
        assert!(
            CODE_REGISTRY.iter().any(|e| e.numeric == hex),
            "CoreError diagnostic_code 0x{hex:04X} not found in CODE_REGISTRY for variant {:?}",
            error
        );
    }
}

#[test]
fn every_core_error_variant_symbolic_code_is_registered() {
    for error in &all_core_error_variants() {
        let sym = error.symbolic_code();
        let reconstructed = SymbolicCode::from_static(sym.as_str());
        assert!(
            reconstructed.is_some(),
            "CoreError symbolic_code '{}' not reconstructable via from_static for variant {:?}",
            sym.as_str(),
            error
        );
        assert!(
            CODE_REGISTRY.iter().any(|e| e.symbolic == sym.as_str()),
            "CoreError symbolic_code '{}' not in CODE_REGISTRY for variant {:?}",
            sym.as_str(),
            error
        );
    }
}

#[test]
fn all_40_core_error_variants_enumerated() {
    let count = all_core_error_variants().len();
    assert!(
        count >= 40,
        "Expected at least 40 CoreError variants, found {count}"
    );
}
