#![forbid(unsafe_code)]
//! B-06: Section 17 runtime code reverse parity (PO-012).
//!
//! Verifies that every mapped Section 17 code name has at least one
//! runtime_code() source across CoreError, RuntimeError, and IpcError.
//! Unmapped codes are documented with their rationale.

use std::collections::BTreeSet;
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, ConstIdx, SlotIdx, StepIdx};

/// Golden set of all 33 Section 17 runtime code names per velvet-ballistics-MASTER.md.
const SECTION_17_MAPPED: &[&str] = &[
    "INPUT_TYPE_MISMATCH",
    "COLLECT_LIMIT_REACHED",
    "REPEAT_LIMIT_REACHED",
    "QUEUE_FULL",
    "CONST_OUT_OF_BOUNDS",
    "MISSING_OUTPUT_SLOT",
    "STEP_STATE_OUT_OF_BOUNDS",
    "EXPRESSION_STACK_OVERFLOW",
    "EXPRESSION_STACK_UNDERFLOW",
    "INVALID_COMPILED_WORKFLOW",
    "INTERNAL_INVARIANT_VIOLATION",
    "UNSUPPORTED_PRIMITIVE",
    "ACTION_FAILED",
    "IPC_FRAME_INVALID",
    "IPC_PAYLOAD_TOO_LARGE",
    "STORAGE_ERROR",
    "ADMISSION_DURABILITY_ERROR",
    "BUDGET_EXCEEDED",
    "CAPABILITY_DENIED",
    "WAIT_TIMEOUT",
    "ASK_TIMEOUT",
    "FOR_EACH_ITEM_FAILED",
    "TOGETHER_BRANCH_FAILED",
    "COLLECT_PAGE_FAILED",
    "REDUCE_ITEM_FAILED",
    "INPUT_MAPPING_FAILED",
];

const SECTION_17_UNMAPPED: &[&str] = &[
    "REFERENCE_MISSING",
    "STEP_SKIPPED_REFERENCE",
    "RETRY_EXHAUSTED",
    "RESULT_REFERENCE_MISSING",
    "PAYLOAD_TOO_LARGE", // runtime-specific (diff from Section 16 PAYLOAD_TOO_LARGE)
    "REPLAY_DIVERGED",
    "SECRET_UNAVAILABLE",
];

/// Collect all unique runtime_code values returned by CoreError variants.
fn core_error_runtime_codes() -> BTreeSet<String> {
    let mut codes = BTreeSet::new();

    // Mapped variants
    let variants: Vec<vb_core::errors::CoreError> = vec![
        vb_core::errors::CoreError::ConstOutOfBounds {
            index: ConstIdx::new(0),
        },
        vb_core::errors::CoreError::TypeMismatch {
            expected: "n",
            found: "b",
        },
        vb_core::errors::CoreError::NonBoolCondition {
            slot: SlotIdx::new(0),
        },
        vb_core::errors::CoreError::MissingOutputSlot {
            step: StepIdx::new(0),
        },
        vb_core::errors::CoreError::StepStateOutOfBounds {
            step: StepIdx::new(0),
        },
        vb_core::errors::CoreError::ExpressionStackOverflow { max: 1 },
        vb_core::errors::CoreError::ExpressionStackUnderflow,
        vb_core::errors::CoreError::InvalidCompiledWorkflow { reason: "x" },
        vb_core::errors::CoreError::InternalInvariantViolation { reason: "x" },
        vb_core::errors::CoreError::UnsupportedPrimitive { primitive: "x" },
        vb_core::errors::CoreError::QueueFull,
        vb_core::errors::CoreError::RepeatExhausted { max: 1 },
        vb_core::errors::CoreError::CollectPageLimitExceeded,
        vb_core::errors::CoreError::CollectItemLimitExceeded,
        vb_core::errors::CoreError::CollectTimeLimitExceeded,
        vb_core::errors::CoreError::BudgetExceeded {
            budget: "x",
            limit: 1,
        },
        vb_core::errors::CoreError::CapabilityDenied {
            action: ActionId::new(1),
            required: Capability::new("test".into(), ActionId::new(1)),
            granted: CapabilitySet::empty(),
        },
    ];

    for v in &variants {
        if let Some(code) = v.runtime_code() {
            codes.insert(code.to_owned());
        }
    }

    codes
}

/// Collect all unique runtime_code values returned by RuntimeError variants.
fn runtime_error_runtime_codes() -> BTreeSet<String> {
    let mut codes = BTreeSet::new();

    let dig = vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]);

    let variants: Vec<vb_runtime::RuntimeError> = vec![
        vb_runtime::RuntimeError::QueueFull,
        vb_runtime::RuntimeError::ActiveRunCapacityExceeded { capacity: 1 },
        vb_runtime::RuntimeError::JournalFull { capacity: 1 },
        vb_runtime::RuntimeError::JournalPoisoned,
        vb_runtime::RuntimeError::UnsupportedAsyncStrictAck,
        vb_runtime::RuntimeError::StorageJournalAppend {
            source: std::sync::Arc::new(vb_storage::JournalError::QueueFull),
        },
        vb_runtime::RuntimeError::AdmissionHeaderPersistenceFailed {
            source: std::sync::Arc::new(vb_storage::JournalError::QueueFull),
        },
        vb_runtime::RuntimeError::AdmissionArtifactDigestMismatch {
            requested: dig,
            found: dig,
        },
        vb_runtime::RuntimeError::AdmissionArtifactStale { digest: dig },
        vb_runtime::RuntimeError::AdmissionDigestMismatch {
            requested: dig,
            record: dig,
            envelope: dig,
        },
        vb_runtime::RuntimeError::InvalidActionCompletion,
        vb_runtime::RuntimeError::EngineDriveFailed {
            run: vb_core::ids::RunId::new(1),
            source: Box::new(vb_core::errors::CoreError::QueueFull),
        },
        vb_runtime::RuntimeError::AskTimeout {
            step: vb_core::ids::StepIdx::new(0),
            ask_id: vb_core::ids::StepIdx::new(0),
        },
        vb_runtime::RuntimeError::WaitTimeout {
            step: vb_core::ids::StepIdx::new(0),
        },
        vb_runtime::RuntimeError::CollectPageFailed {
            step: vb_core::ids::StepIdx::new(0),
            expected_page: vb_core::ids::ListId::new(0),
            found_page: vb_core::ids::ListId::new(0),
        },
        vb_runtime::RuntimeError::ReduceItemFailed {
            step: vb_core::ids::StepIdx::new(0),
            item_index: 0,
            source: Box::new(vb_core::errors::CoreError::QueueFull),
        },
        vb_runtime::RuntimeError::TogetherBranchFailed {
            step: vb_core::ids::StepIdx::new(0),
            branch_index: 0,
            source: Box::new(vb_core::errors::CoreError::QueueFull),
        },
        vb_runtime::RuntimeError::ForEachItemFailed {
            step: vb_core::ids::StepIdx::new(0),
            item_index: 0,
            source: Box::new(vb_core::errors::CoreError::QueueFull),
        },
        vb_runtime::RuntimeError::InputMappingFailed {
            kind: vb_runtime::InputMappingFailureKind::MalformedPostcard,
            source: Box::new(vb_core::errors::CoreError::QueueFull),
        },
    ];

    for v in &variants {
        if let Some(code) = v.runtime_code() {
            codes.insert(code.to_owned());
        }
    }

    codes
}

/// Collect all unique runtime_code values returned by IpcError variants.
fn ipc_error_runtime_codes() -> BTreeSet<String> {
    let mut codes = BTreeSet::new();

    let variants: Vec<vb_ipc::IpcError> = vec![
        vb_ipc::IpcError::Full,
        vb_ipc::IpcError::PayloadTooLarge {
            actual: 1,
            limit: 1,
        },
        vb_ipc::IpcError::InvalidMagic { actual: 0 },
        vb_ipc::IpcError::UnsupportedVersion { actual: 0 },
        vb_ipc::IpcError::UnknownCommand(0),
        vb_ipc::IpcError::ReservedNonZero { actual: 0 },
        vb_ipc::IpcError::PayloadLengthMismatch {
            header: 0,
            actual: 0,
        },
        vb_ipc::IpcError::HeaderDecodeFailed,
        vb_ipc::IpcError::PayloadLengthOutOfRange { actual: 0 },
        vb_ipc::IpcError::PayloadDecodeFailed,
        vb_ipc::IpcError::ResponseDecodeFailed,
    ];

    for v in &variants {
        if let Some(code) = v.runtime_code() {
            codes.insert(code.to_owned());
        }
    }

    codes
}

#[test]
fn section17_reverse_parity_mapped_codes_have_sources() {
    // Collect all unique runtime codes from all three error types.
    let mut all_codes = BTreeSet::new();
    for code in core_error_runtime_codes() {
        all_codes.insert(code);
    }
    for code in runtime_error_runtime_codes() {
        all_codes.insert(code);
    }
    for code in ipc_error_runtime_codes() {
        all_codes.insert(code);
    }

    // Every mapped Section 17 code must have at least one source.
    let mut missing: Vec<&str> = Vec::new();
    for name in SECTION_17_MAPPED {
        if !all_codes.contains(*name) {
            missing.push(*name);
        }
    }

    assert_eq!(
        missing,
        Vec::<&str>::new(),
        "Section 17 mapped codes must have runtime_code() sources. Found codes: {all_codes:?}"
    );
}

#[test]
fn section17_reverse_parity_unmapped_codes_have_no_sources() {
    let mut all_codes = BTreeSet::new();
    for code in core_error_runtime_codes() {
        all_codes.insert(code);
    }
    for code in runtime_error_runtime_codes() {
        all_codes.insert(code);
    }
    for code in ipc_error_runtime_codes() {
        all_codes.insert(code);
    }

    let mut unexpectedly_mapped: Vec<&str> = Vec::new();
    for name in SECTION_17_UNMAPPED {
        if all_codes.contains(*name) {
            unexpectedly_mapped.push(*name);
        }
    }

    assert_eq!(
        unexpectedly_mapped,
        Vec::<&str>::new(),
        "Section 17 unmapped codes must not have runtime_code() sources before vb-wstlsl01 lands"
    );
}
