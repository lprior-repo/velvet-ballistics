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

const UNMAPPED_CODES_WITH_RATIONALE: &[(&str, &str)] = &[
    (
        "REFERENCE_MISSING",
        "runtime reference failures not yet implemented",
    ),
    (
        "STEP_SKIPPED_REFERENCE",
        "skip-reference validation is not runtime surfaced",
    ),
    (
        "RETRY_EXHAUSTED",
        "engine retry exhaustion is not in this report surface",
    ),
    (
        "RESULT_REFERENCE_MISSING",
        "runtime result-reference resolution not implemented",
    ),
    (
        "PAYLOAD_TOO_LARGE",
        "action payload code is outside this runtime report surface",
    ),
    (
        "REPLAY_DIVERGED",
        "replay divergence is not yet surfaced as typed runtime error",
    ),
];

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

const PARTIALLY_MAPPED_CODES: &[(&str, &str)] = &[(
    "SECRET_UNAVAILABLE",
    "Partially mapped: JournalError::SecretUnavailable exists at storage layer but has no runtime_code() source",
)];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn section17_coverage_report_required_codes_match_runtime() {
    let runtime_codes = collect_all_runtime_codes();

    assert_eq!(
        MAPPED_CODES.len(),
        26,
        "golden mapped count must be 26 (the 26 codes documented as mapped)"
    );

    // The test fails loudly if any of the 26 mapped Section 17 codes is
    // missing from production. This is the "fail loudly" behavior the
    // master §17 contract requires — no self-laundering.
    let mut missing: Vec<&str> = Vec::new();
    for code in MAPPED_CODES {
        if !runtime_codes.contains(*code) {
            missing.push(*code);
        }
    }

    assert_eq!(
        missing,
        Vec::<&str>::new(),
        "Section 17 mapped codes must have runtime_code() sources. Available codes: {runtime_codes:?}"
    );
}

#[test]
fn section17_coverage_report_unmapped_codes_stay_unmapped() {
    let runtime_codes = collect_all_runtime_codes();
    let mut unexpectedly_mapped: Vec<&str> = Vec::new();
    for (code, _rationale) in UNMAPPED_CODES_WITH_RATIONALE {
        if runtime_codes.contains(*code) {
            unexpectedly_mapped.push(*code);
        }
    }
    assert_eq!(unexpectedly_mapped, Vec::<&str>::new());
}

#[test]
fn section17_coverage_report_counts_are_correct() {
    let mapped_count = MAPPED_CODES.len();
    let unmapped_count = UNMAPPED_CODES_WITH_RATIONALE.len();
    let partial_count = PARTIALLY_MAPPED_CODES.len();
    let runtime_codes = collect_all_runtime_codes();
    let present_count = MAPPED_CODES
        .iter()
        .filter(|code| runtime_codes.contains(**code))
        .count();

    assert_eq!(mapped_count, 26, "expected 26 mapped Section 17 codes");
    assert_eq!(unmapped_count, 6, "expected 6 unmapped Section 17 codes");
    assert_eq!(
        partial_count, 1,
        "expected 1 partially mapped Section 17 code"
    );
    assert_eq!(mapped_count + unmapped_count + partial_count, 33);
    assert_eq!(present_count, mapped_count);
}
