//! Property test: HasSymbolicCode determinism for CoreError.
//!
//! Compensates: BLOCKED PO-013 (kani_symbolic_code_determinism).
//! Addresses: F-BR-004 from test-plan.md.
//!
//! Invariant: Calling `symbolic_code()` twice on any CoreError value always returns
//!            the same `SymbolicCode`. No panics, pure function, no side effects.
//!
//! Cross-crate determinism tests for ValidationError, YamlError, RuntimeError,
//! JournalError, and CompileError live in workspace_tests.

use proptest::prelude::*;
use vb_core::diagnostic::{HasSymbolicCode, SymbolicCode};
use vb_core::errors::CoreError;
use vb_core::ids::{SlotIdx, StepIdx};

// ---------------------------------------------------------------------------
// Strategy: generate arbitrary CoreError instances.
//
// Note: Variants with `&'static str` fields use fixed string values in the
// strategy since the symbolic_code() result depends only on the variant, not
// the field values. The numeric fields are varied to exercise all paths.
// ---------------------------------------------------------------------------

fn arb_core_error() -> impl Strategy<Value = CoreError> {
    prop_oneof![
        // Unit-like variants
        Just(CoreError::DivisionByZero),
        Just(CoreError::NonFiniteNumber),
        Just(CoreError::StepBudgetExhausted),
        Just(CoreError::StepCounterOverflow),
        Just(CoreError::QueueFull),
        Just(CoreError::AllocationFailed),
        Just(CoreError::ExpressionStackUnderflow),
        Just(CoreError::CollectPageLimitExceeded),
        Just(CoreError::CollectItemLimitExceeded),
        Just(CoreError::CollectTimeLimitExceeded),
        // Numeric-field variants
        (0u16..100).prop_map(|s| CoreError::InvalidProgramCounter {
            step: StepIdx::new(s)
        }),
        (0u16..100).prop_map(|s| CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(s)
        }),
        (0u16..100).prop_map(|s| CoreError::SlotUninitialized {
            slot: SlotIdx::new(s)
        }),
        (0u16..100).prop_map(|s| CoreError::MissingNextStep {
            step: StepIdx::new(s)
        }),
        (0u16..100).prop_map(|s| CoreError::MissingOutputSlot {
            step: StepIdx::new(s)
        }),
        (0u16..100).prop_map(|s| CoreError::StepStateOutOfBounds {
            step: StepIdx::new(s)
        }),
        (0u16..100).prop_map(|s| CoreError::NonBoolCondition {
            slot: SlotIdx::new(s)
        }),
        (any::<u8>()).prop_map(|m| CoreError::ExpressionStackOverflow { max: m }),
        (any::<u16>()).prop_map(|m| CoreError::RepeatExhausted { max: m }),
        (any::<u16>()).prop_map(|m| CoreError::TogetherBranchLimitExceeded { max: m }),
        (any::<u16>()).prop_map(|l| CoreError::ParallelLimitExceeded { limit: l }),
        // &'static str variants — fixed strings, varying numeric params
        Just(CoreError::InvalidCompiledWorkflow { reason: "test" }),
        Just(CoreError::UnsupportedPrimitive { primitive: "test" }),
        Just(CoreError::ResourceLimitExceeded { resource: "test" }),
        Just(CoreError::IterationLimitExceeded { resource: "test" }),
        Just(CoreError::InternalInvariantViolation { reason: "test" }),
        Just(CoreError::BudgetParse { reason: "test" }),
        Just(CoreError::TypeMismatch {
            expected: "a",
            found: "b"
        }),
        Just(CoreError::UnsupportedAccessorTraversal {
            segment: "a",
            found: "b"
        }),
        (any::<u64>()).prop_map(|l| CoreError::BudgetExceeded {
            budget: "cpu",
            limit: l
        }),
        // Enumerate a wide range of variants
    ]
}

// ---------------------------------------------------------------------------
// Determinism property tests (proptest-generated cases)
// ---------------------------------------------------------------------------

proptest! {
    /// Verify that calling symbolic_code() twice on any CoreError returns
    /// the same SymbolicCode — determinism invariant.
    #[test]
    fn core_error_symbolic_code_determinism(error in arb_core_error()) {
        let code1 = error.symbolic_code();
        let code2 = error.symbolic_code();
        prop_assert_eq!(code1, code2,
            "CoreError::symbolic_code() must be deterministic: '{}' vs '{}'",
            code1.as_str(), code2.as_str()
        );
        // Verify the code is registered (non-vacuous assertion)
        let reconstructed = SymbolicCode::from_static(code1.as_str());
        prop_assert!(reconstructed.is_some(),
            "CoreError symbolic code '{}' must be registered in CODE_REGISTRY",
            code1.as_str()
        );
    }

    /// Verify that HasSymbolicCode::symbolic_code() is deterministic.
    #[test]
    fn core_error_has_symbolic_code_determinism(error in arb_core_error()) {
        let code1 = HasSymbolicCode::symbolic_code(&error);
        let code2 = HasSymbolicCode::symbolic_code(&error);
        prop_assert_eq!(code1, code2,
            "HasSymbolicCode::symbolic_code() must be deterministic for CoreError"
        );
    }

    /// Verify consistency: CoreError::symbolic_code() and
    /// HasSymbolicCode::symbolic_code() return the same value.
    #[test]
    fn core_error_code_and_trait_agree(error in arb_core_error()) {
        let code_direct = error.symbolic_code();
        let code_trait = HasSymbolicCode::symbolic_code(&error);
        prop_assert_eq!(code_direct, code_trait,
            "CoreError::symbolic_code() and HasSymbolicCode::symbolic_code() must agree"
        );
    }
}

// ---------------------------------------------------------------------------
// Strateless determinism: exhaustive variant coverage
// ---------------------------------------------------------------------------

#[test]
fn core_error_all_variants_produce_registered_codes() {
    use chrono::Utc;
    use vb_core::diagnostic::CODE_REGISTRY;
    use vb_core::ids::{ActionId, BlobId, ConstIdx, ExprIdx, ListId, ObjectId, SymbolId};

    let run_id = vb_core::ids::RunId::new(1);

    let errors: Vec<CoreError> = vec![
        CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        },
        CoreError::MissingNextStep {
            step: StepIdx::new(0),
        },
        CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(0),
        },
        CoreError::SlotUninitialized {
            slot: SlotIdx::new(0),
        },
        CoreError::ExprOutOfBounds {
            expr: ExprIdx::new(0),
        },
        CoreError::ConstOutOfBounds {
            index: ConstIdx::new(0),
        },
        CoreError::MissingOutputSlot {
            step: StepIdx::new(0),
        },
        CoreError::StepStateOutOfBounds {
            step: StepIdx::new(0),
        },
        CoreError::TypeMismatch {
            expected: "u64",
            found: "string",
        },
        CoreError::NonBoolCondition {
            slot: SlotIdx::new(0),
        },
        CoreError::DivisionByZero,
        CoreError::NonFiniteNumber,
        CoreError::StepBudgetExhausted,
        CoreError::StepCounterOverflow,
        CoreError::QueueFull,
        CoreError::ResourceLimitExceeded { resource: "cpu" },
        CoreError::AllocationFailed,
        CoreError::ExpressionStackOverflow { max: 64 },
        CoreError::ExpressionStackUnderflow,
        CoreError::InvalidCompiledWorkflow { reason: "test" },
        CoreError::UnsupportedPrimitive { primitive: "wait" },
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
        CoreError::BudgetExceeded {
            budget: "cpu",
            limit: 100,
        },
        CoreError::BudgetParse { reason: "invalid" },
        CoreError::CapabilityDenied {
            action: ActionId::new(0),
            required: vb_core::capability::Capability::new(
                Box::<str>::from("test_cap"),
                ActionId::new(0),
            ),
            granted: vb_core::capability::CapabilitySet::empty(),
        },
        CoreError::CollectPageOrderViolation {
            kind: vb_core::errors::CollectPageOrderViolationKind::OutOfOrder,
            run_id,
            collector_slot: SlotIdx::new(0),
            expected_page: ListId::new(0),
            observed_page: ListId::new(1),
        },
        CoreError::CollectExtraHydrationFailed {
            kind: vb_core::errors::CollectExtraHydrationFailureKind::EmptyExtra,
            run_id,
            collector_slot: SlotIdx::new(0),
            event_seq: None,
        },
        CoreError::CollectEvidenceCapacityExceeded {
            run_id,
            slot: SlotIdx::new(0),
            capacity: 10,
            len: 11,
            required: "extra_slots",
        },
        CoreError::LifecycleStorageUnavailable {
            code: vb_core::diagnostic::DiagnosticCode::new(0x1501),
            context: "test".into(),
            timestamp: Utc::now(),
            bead_id: None,
        },
        CoreError::LifecycleDuplicateRequest {
            code: vb_core::diagnostic::DiagnosticCode::new(0x1502),
            context: "test".into(),
            timestamp: Utc::now(),
            bead_id: None,
            command: None,
        },
        CoreError::LifecycleStaleRequest {
            code: vb_core::diagnostic::DiagnosticCode::new(0x1503),
            context: "test".into(),
            timestamp: Utc::now(),
            bead_id: None,
            command: None,
        },
        CoreError::LifecycleInvalidTransition {
            code: vb_core::diagnostic::DiagnosticCode::new(0x1504),
            context: "test".into(),
            timestamp: Utc::now(),
            bead_id: None,
            command: None,
        },
        CoreError::JournalWriteFailure {
            code: vb_core::diagnostic::DiagnosticCode::new(0x1505),
            context: "test".into(),
            timestamp: Utc::now(),
            bead_id: None,
        },
        CoreError::ReplayCorruption {
            code: vb_core::diagnostic::DiagnosticCode::new(0x1506),
            context: "test".into(),
            timestamp: Utc::now(),
            bead_id: None,
        },
    ];

    for error in &errors {
        let code1 = error.symbolic_code();
        let code2 = error.symbolic_code();
        assert_eq!(code1, code2, "symbolic_code must be deterministic");
        assert!(
            CODE_REGISTRY.iter().any(|e| e.symbolic == code1.as_str()),
            "CoreError '{code1}' must be in CODE_REGISTRY"
        );
    }
}
