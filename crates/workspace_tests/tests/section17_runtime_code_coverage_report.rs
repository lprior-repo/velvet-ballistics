#![forbid(unsafe_code)]
//! B-07: Section 17 runtime code coverage report (PO-012b).
//!
//! Documentation test that categorizes all 33 Section 17 runtime code names
//! as MAPPED, UNMAPPED, or PARTIALLY_MAPPED, with rationale for gaps.
//! This is a living specification test that alerts when unmapped codes
//! gain accidental coverage or mapped codes lose their source.

use std::collections::BTreeSet;
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, ConstIdx, SlotIdx, StepIdx};

// ---------------------------------------------------------------------------
// Helper: collect runtime_code strings from all error types
// ---------------------------------------------------------------------------

fn collect_all_runtime_codes() -> BTreeSet<String> {
    let mut all = BTreeSet::new();

    // CoreError mapped runtime codes
    for v in [
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
    ]
    .iter()
    {
        if let Some(code) = v.runtime_code() {
            all.insert(code.to_owned());
        }
    }

    let dig = vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]);

    // RuntimeError mapped runtime codes
    for v in [
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
    ]
    .iter()
    {
        if let Some(code) = v.runtime_code() {
            all.insert(code.to_owned());
        }
    }

    // IpcError mapped runtime codes
    for v in [
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
    ]
    .iter()
    {
        if let Some(code) = v.runtime_code() {
            all.insert(code.to_owned());
        }
    }

    all
}

// ---------------------------------------------------------------------------
// Golden data: all 33 Section 17 runtime codes with their classification
// ---------------------------------------------------------------------------

const MAPPED_CODES: &[&str] = &[
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

const UNMAPPED_CODES_WITH_RATIONALE: &[(&str, &str)] = &[
    (
        "REFERENCE_MISSING",
        "Future: unresolved runtime reference failures not yet implemented",
    ),
    (
        "STEP_SKIPPED_REFERENCE",
        "Future: skip-reference validation lives in compile phase",
    ),
    (
        "RETRY_EXHAUSTED",
        "Future: runtime retry exhaustion not yet surfaced as typed error",
    ),
    (
        "RESULT_REFERENCE_MISSING",
        "Future: result-reference resolution in runtime not yet implemented",
    ),
    (
        "PAYLOAD_TOO_LARGE",
        "Note: PAYLOAD_TOO_LARGE is a Section 16 validation code; Section 17 runtime variant not yet implemented",
    ),
    (
        "REPLAY_DIVERGED",
        "Future: deterministic replay divergence not yet surfaced as typed error",
    ),
];

const PARTIALLY_MAPPED_CODES: &[(&str, &str)] = &[(
    "SECRET_UNAVAILABLE",
    "Partially mapped: JournalError::SecretUnavailable exists at storage layer but has no runtime_code() source",
)];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn section17_coverage_report_mapped_codes_match_runtime() {
    let runtime_codes = collect_all_runtime_codes();

    assert_eq!(MAPPED_CODES.len(), 26, "golden mapped count must be 26");

    for code in MAPPED_CODES {
        assert!(
            runtime_codes.contains(*code),
            "MAPPED code '{code}' not found in runtime_code() output. Available codes: {runtime_codes:?}"
        );
    }
}

#[test]
fn section17_coverage_report_unmapped_codes_stay_unmapped() {
    let runtime_codes = collect_all_runtime_codes();

    for (code, _rationale) in UNMAPPED_CODES_WITH_RATIONALE {
        assert!(
            !runtime_codes.contains(*code),
            "UNMAPPED code '{code}' unexpectedly found in runtime_code() output"
        );
    }
}

#[test]
fn section17_coverage_report_counts_are_correct() {
    let mapped_count = MAPPED_CODES.len();
    let unmapped_count = UNMAPPED_CODES_WITH_RATIONALE.len();
    let partial_count = PARTIALLY_MAPPED_CODES.len();
    let total = mapped_count + unmapped_count + partial_count;

    assert_eq!(mapped_count, 26, "expected 26 mapped Section 17 codes");
    assert_eq!(unmapped_count, 6, "expected 6 unmapped Section 17 codes");
    assert_eq!(
        partial_count, 1,
        "expected 1 partially mapped Section 17 code"
    );
    assert_eq!(
        total, 33,
        "expected 33 unique Section 17 codes (26 mapped + 6 unmapped + 1 partially mapped)"
    );

    // Verify the mapped count against actual production runtime_codes
    let runtime_codes = collect_all_runtime_codes();
    let present_count = MAPPED_CODES
        .iter()
        .filter(|c| runtime_codes.contains(**c))
        .count();
    assert_eq!(
        present_count, mapped_count,
        "all {mapped_count} mapped codes must be present in runtime, found {present_count}"
    );
}
