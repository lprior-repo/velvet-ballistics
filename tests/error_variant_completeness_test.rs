use std::io;

use vb_codegen::CodegenError;
use vb_compile::{CompileError, SourceMark};
use vb_core::{
    ActionError, ActionId, BlobId, ConstIdx, CoreError, DiagnosticCode, DiagnosticCodeParseError,
    ExprIdx, ListId, ObjectId, SlotIdx, StepIdx, SymbolId, WorkflowDigest, WorkflowError,
};
use vb_expr::ExprError;
use vb_ipc::IpcError;
use vb_ipc::client::IpcClientError;
use vb_ipc::server::{IpcServerError, WorkflowResolutionError};
use vb_runtime::RuntimeError;
use vb_runtime::engine::RuntimeEngineError;
use vb_storage::recovery::RecoveryError;
use vb_storage::{EventSeq, JournalError};
use vb_validate::ValidationError;
use vb_yaml::YamlError;

#[test]
fn validation_errors_map_every_public_variant_to_an_exact_code() {
    let validation = validation_error_codes();
    assert_unique_codes(&validation);
    assert_eq!(validation.len(), 36);
}

#[test]
fn core_errors_map_every_public_variant_to_an_exact_code() {
    let core = core_error_codes();
    assert_unique_codes(&core);
    assert_eq!(core.len(), 33);
}

#[test]
fn runtime_errors_map_every_public_variant_to_an_exact_code() {
    let runtime = runtime_error_codes();
    assert_unique_codes(&runtime);
    assert_eq!(runtime.len(), 14);
}

#[test]
fn ipc_errors_map_every_public_variant_to_an_exact_code() {
    let ipc = ipc_error_codes();
    assert_unique_codes(&ipc);
    assert_eq!(ipc.len(), 14);
}

#[test]
fn journal_errors_map_every_public_variant_to_an_exact_code() {
    let journal = journal_error_codes();
    assert_unique_codes(&journal);
    assert_eq!(journal.len(), 22);
}

#[test]
fn yaml_public_constructible_errors_have_exhaustive_variant_audits() {
    assert_eq!(yaml_variant_count(), 19);
}

#[test]
fn expr_public_constructible_errors_have_exhaustive_variant_audits() {
    assert_eq!(expr_variant_count(), 20);
}

#[test]
fn action_public_constructible_errors_have_exhaustive_variant_audits() {
    assert_eq!(action_variant_count(), 9);
}

#[test]
fn workflow_public_constructible_errors_have_exhaustive_variant_audits() {
    assert_eq!(workflow_variant_count(), 10);
}

#[test]
fn diagnostic_parse_public_constructible_errors_have_exhaustive_variant_audits() {
    assert_eq!(diagnostic_parse_variant_count(), 2);
}

#[test]
fn compile_public_constructible_errors_have_exhaustive_variant_audits() {
    assert_eq!(compile_constructible_variant_count(), 69);
}

#[test]
fn codegen_public_constructible_errors_have_exhaustive_variant_audits() {
    assert_eq!(codegen_constructible_variant_count(), 6);
}

#[test]
fn runtime_engine_public_constructible_errors_have_exhaustive_variant_audits() {
    assert_eq!(runtime_engine_variant_count(), 4);
}

#[test]
fn workflow_resolution_public_constructible_errors_have_exhaustive_variant_audits() {
    assert_eq!(workflow_resolution_variant_count(), 3);
}

#[test]
fn ipc_client_public_constructible_errors_have_exhaustive_variant_audits() {
    assert_eq!(ipc_client_constructible_variant_count(), 4);
}

#[test]
fn ipc_server_public_constructible_errors_have_exhaustive_variant_audits() {
    assert_eq!(ipc_server_constructible_variant_count(), 9);
}

#[test]
fn recovery_public_constructible_errors_have_exhaustive_variant_audits() {
    assert_eq!(recovery_constructible_variant_count(), 11);
}

#[test]
fn external_wrapper_variant_exceptions_are_documented_and_still_matched() {
    assert_eq!(EXTERNAL_WRAPPER_EXCEPTIONS.len(), 6);
    assert_eq!(compile_error_variant_name(&compile_utf8_error()), "Utf8");
    assert_eq!(
        journal_error_variant_name(&JournalError::KeyCapacity),
        "KeyCapacity"
    );
    assert_eq!(codegen_error_variant_name(&codegen_io_error()), "Io");
}

const EXTERNAL_WRAPPER_EXCEPTIONS: [&str; 6] = [
    "CompileError::Parse wraps saphyr::ScanError and is covered by exhaustive matching, not synthetic construction.",
    "JournalError::Fjall wraps fjall::Error and is covered by exhaustive matching, not synthetic construction.",
    "JournalError::Encode wraps postcard::Error and is covered by exhaustive matching, not synthetic construction.",
    "RecoveryError::Journal wraps JournalError and is covered through the constructible JournalError::KeyCapacity sample.",
    "RuntimeEngineError::Core wraps CoreError and is covered through the constructible CoreError::QueueFull sample.",
    "RuntimeEngineError::Action wraps ActionError and is covered through the constructible ActionError::QueueFull sample.",
];

fn assert_unique_codes(codes: &[(DiagnosticCode, &'static str)]) {
    let duplicate = codes.iter().enumerate().find_map(|(left_index, left)| {
        codes
            .iter()
            .skip(left_index.saturating_add(1))
            .find(|right| left.0 == right.0)
            .map(|right| (left, right))
    });

    assert_eq!(duplicate, None, "duplicate diagnostic code found");
}

fn validation_error_codes() -> Vec<(DiagnosticCode, &'static str)> {
    let samples = [
        ValidationError::DuplicateKey,
        ValidationError::ForbiddenYamlFeature,
        ValidationError::UnknownTopLevelField,
        ValidationError::UnknownStepField,
        ValidationError::MissingRequiredField { field: s("field") },
        ValidationError::InvalidVersion { version: s("v") },
        ValidationError::InvalidId { id: s("id") },
        ValidationError::ReservedId { id: s("id") },
        ValidationError::DuplicateId { id: s("id") },
        ValidationError::MultipleStepPrimitives,
        ValidationError::MissingStepPrimitive,
        ValidationError::UnknownReference {
            reference: s("ref"),
        },
        ValidationError::FutureReference {
            reference: s("ref"),
        },
        ValidationError::SecretNotDeclared {
            secret: s("secret"),
        },
        ValidationError::DirectRuntimeReference,
        ValidationError::InvalidThenTarget,
        ValidationError::ControlFlowCycle,
        ValidationError::UnreachableStep { step: s("step") },
        ValidationError::InvalidChoose,
        ValidationError::InvalidForEach,
        ValidationError::InvalidTogether,
        ValidationError::InvalidCollect,
        ValidationError::InvalidReduce,
        ValidationError::InvalidRepeat,
        ValidationError::InvalidWait,
        ValidationError::InvalidAsk,
        ValidationError::InvalidFinish,
        ValidationError::InvalidRetry,
        ValidationError::InvalidOnError,
        ValidationError::SecretResultLeak,
        ValidationError::TypeMismatch {
            expected: s("a"),
            found: s("b"),
        },
        ValidationError::PayloadTooLarge,
        ValidationError::LimitRequired { resource: s("r") },
        ValidationError::LimitExceeded { resource: s("r") },
        ValidationError::UnsupportedTrigger { trigger: s("t") },
        ValidationError::HttpTriggerOutOfCore,
    ];
    samples
        .iter()
        .map(|error| {
            (
                vb_validate::diagnostic::error_code(error),
                validation_error_variant_name(error),
            )
        })
        .collect()
}

fn validation_error_variant_name(error: &ValidationError) -> &'static str {
    match error {
        ValidationError::DuplicateKey => "DuplicateKey",
        ValidationError::ForbiddenYamlFeature => "ForbiddenYamlFeature",
        ValidationError::UnknownTopLevelField => "UnknownTopLevelField",
        ValidationError::UnknownStepField => "UnknownStepField",
        ValidationError::MissingRequiredField { .. } => "MissingRequiredField",
        ValidationError::InvalidVersion { .. } => "InvalidVersion",
        ValidationError::InvalidId { .. } => "InvalidId",
        ValidationError::ReservedId { .. } => "ReservedId",
        ValidationError::DuplicateId { .. } => "DuplicateId",
        ValidationError::MultipleStepPrimitives => "MultipleStepPrimitives",
        ValidationError::MissingStepPrimitive => "MissingStepPrimitive",
        ValidationError::UnknownReference { .. } => "UnknownReference",
        ValidationError::FutureReference { .. } => "FutureReference",
        ValidationError::SecretNotDeclared { .. } => "SecretNotDeclared",
        ValidationError::DirectRuntimeReference => "DirectRuntimeReference",
        ValidationError::InvalidThenTarget => "InvalidThenTarget",
        ValidationError::ControlFlowCycle => "ControlFlowCycle",
        ValidationError::UnreachableStep { .. } => "UnreachableStep",
        ValidationError::InvalidChoose => "InvalidChoose",
        ValidationError::InvalidForEach => "InvalidForEach",
        ValidationError::InvalidTogether => "InvalidTogether",
        ValidationError::InvalidCollect => "InvalidCollect",
        ValidationError::InvalidReduce => "InvalidReduce",
        ValidationError::InvalidRepeat => "InvalidRepeat",
        ValidationError::InvalidWait => "InvalidWait",
        ValidationError::InvalidAsk => "InvalidAsk",
        ValidationError::InvalidFinish => "InvalidFinish",
        ValidationError::InvalidRetry => "InvalidRetry",
        ValidationError::InvalidOnError => "InvalidOnError",
        ValidationError::SecretResultLeak => "SecretResultLeak",
        ValidationError::TypeMismatch { .. } => "TypeMismatch",
        ValidationError::PayloadTooLarge => "PayloadTooLarge",
        ValidationError::LimitRequired { .. } => "LimitRequired",
        ValidationError::LimitExceeded { .. } => "LimitExceeded",
        ValidationError::UnsupportedTrigger { .. } => "UnsupportedTrigger",
        ValidationError::HttpTriggerOutOfCore => "HttpTriggerOutOfCore",
    }
}

fn core_error_codes() -> Vec<(DiagnosticCode, &'static str)> {
    let samples = [
        CoreError::InvalidProgramCounter { step: step() },
        CoreError::MissingNextStep { step: step() },
        CoreError::SlotOutOfBounds { slot: slot() },
        CoreError::ExprOutOfBounds {
            expr: ExprIdx::new(1),
        },
        CoreError::ConstOutOfBounds {
            index: ConstIdx::new(1),
        },
        CoreError::MissingOutputSlot { step: step() },
        CoreError::StepStateOutOfBounds { step: step() },
        CoreError::TypeMismatch {
            expected: "a",
            found: "b",
        },
        CoreError::NonBoolCondition { slot: slot() },
        CoreError::DivisionByZero,
        CoreError::NonFiniteNumber,
        CoreError::StepBudgetExhausted,
        CoreError::StepCounterOverflow,
        CoreError::QueueFull,
        CoreError::ResourceLimitExceeded { resource: "r" },
        CoreError::AllocationFailed,
        CoreError::ExpressionStackOverflow { max: 1 },
        CoreError::ExpressionStackUnderflow,
        CoreError::InvalidCompiledWorkflow { reason: "r" },
        CoreError::UnsupportedPrimitive { primitive: "p" },
        CoreError::UnsupportedAccessorTraversal {
            segment: "s",
            found: "f",
        },
        CoreError::ObjectFieldNotFound {
            field: SymbolId::new(1),
        },
        CoreError::ListIndexOutOfBounds { index: 1 },
        CoreError::InternalInvariantViolation { reason: "r" },
        CoreError::SymbolOutOfBounds {
            symbol: SymbolId::new(1),
        },
        CoreError::ListOutOfBounds {
            list: ListId::new(1),
        },
        CoreError::ObjectOutOfBounds {
            object: ObjectId::new(1),
        },
        CoreError::BlobOutOfBounds {
            blob: BlobId::new(1),
        },
        CoreError::IterationLimitExceeded { resource: "r" },
        CoreError::RepeatExhausted { max: 1 },
        CoreError::CollectPageLimitExceeded,
        CoreError::CollectItemLimitExceeded,
    ];
    samples
        .iter()
        .map(|error| (error.diagnostic_code(), core_error_variant_name(error)))
        .chain(std::iter::once((
            CoreError::TogetherBranchLimitExceeded { max: 1 }.diagnostic_code(),
            "TogetherBranchLimitExceeded",
        )))
        .collect()
}

fn core_error_variant_name(error: &CoreError) -> &'static str {
    match error {
        CoreError::InvalidProgramCounter { .. } => "InvalidProgramCounter",
        CoreError::MissingNextStep { .. } => "MissingNextStep",
        CoreError::SlotOutOfBounds { .. } => "SlotOutOfBounds",
        CoreError::ExprOutOfBounds { .. } => "ExprOutOfBounds",
        CoreError::ConstOutOfBounds { .. } => "ConstOutOfBounds",
        CoreError::MissingOutputSlot { .. } => "MissingOutputSlot",
        CoreError::StepStateOutOfBounds { .. } => "StepStateOutOfBounds",
        CoreError::TypeMismatch { .. } => "TypeMismatch",
        CoreError::NonBoolCondition { .. } => "NonBoolCondition",
        CoreError::DivisionByZero => "DivisionByZero",
        CoreError::NonFiniteNumber => "NonFiniteNumber",
        CoreError::StepBudgetExhausted => "StepBudgetExhausted",
        CoreError::StepCounterOverflow => "StepCounterOverflow",
        CoreError::QueueFull => "QueueFull",
        CoreError::ResourceLimitExceeded { .. } => "ResourceLimitExceeded",
        CoreError::AllocationFailed => "AllocationFailed",
        CoreError::ExpressionStackOverflow { .. } => "ExpressionStackOverflow",
        CoreError::ExpressionStackUnderflow => "ExpressionStackUnderflow",
        CoreError::InvalidCompiledWorkflow { .. } => "InvalidCompiledWorkflow",
        CoreError::UnsupportedPrimitive { .. } => "UnsupportedPrimitive",
        CoreError::UnsupportedAccessorTraversal { .. } => "UnsupportedAccessorTraversal",
        CoreError::ObjectFieldNotFound { .. } => "ObjectFieldNotFound",
        CoreError::ListIndexOutOfBounds { .. } => "ListIndexOutOfBounds",
        CoreError::InternalInvariantViolation { .. } => "InternalInvariantViolation",
        CoreError::SymbolOutOfBounds { .. } => "SymbolOutOfBounds",
        CoreError::ListOutOfBounds { .. } => "ListOutOfBounds",
        CoreError::ObjectOutOfBounds { .. } => "ObjectOutOfBounds",
        CoreError::BlobOutOfBounds { .. } => "BlobOutOfBounds",
        CoreError::IterationLimitExceeded { .. } => "IterationLimitExceeded",
        CoreError::RepeatExhausted { .. } => "RepeatExhausted",
        CoreError::CollectPageLimitExceeded => "CollectPageLimitExceeded",
        CoreError::CollectItemLimitExceeded => "CollectItemLimitExceeded",
        CoreError::TogetherBranchLimitExceeded { .. } => "TogetherBranchLimitExceeded",
    }
}

fn runtime_error_codes() -> Vec<(DiagnosticCode, &'static str)> {
    let samples = [
        RuntimeError::QueueFull,
        RuntimeError::RunNotFound,
        RuntimeError::ActiveRunCapacityExceeded { capacity: 1 },
        RuntimeError::RunAlreadyExists,
        RuntimeError::UnsupportedOperation { operation: "op" },
        RuntimeError::ShutdownInProgress,
        RuntimeError::JournalPoisoned,
        RuntimeError::StorageJournalAppendFailed,
        RuntimeError::UnsupportedAsyncStrictAck,
        RuntimeError::FramePoolUnavailable,
        RuntimeError::InvalidActionCompletion,
        RuntimeError::InvalidTimerFire,
        RuntimeError::UnsupportedFullRecoveryHydration,
        RuntimeError::InvalidRecoveryHydration,
    ];
    samples
        .iter()
        .map(|error| (error.diagnostic_code(), runtime_error_variant_name(error)))
        .collect()
}

fn runtime_error_variant_name(error: &RuntimeError) -> &'static str {
    match error {
        RuntimeError::QueueFull => "QueueFull",
        RuntimeError::RunNotFound => "RunNotFound",
        RuntimeError::ActiveRunCapacityExceeded { .. } => "ActiveRunCapacityExceeded",
        RuntimeError::RunAlreadyExists => "RunAlreadyExists",
        RuntimeError::UnsupportedOperation { .. } => "UnsupportedOperation",
        RuntimeError::ShutdownInProgress => "ShutdownInProgress",
        RuntimeError::JournalPoisoned => "JournalPoisoned",
        RuntimeError::StorageJournalAppendFailed => "StorageJournalAppendFailed",
        RuntimeError::UnsupportedAsyncStrictAck => "UnsupportedAsyncStrictAck",
        RuntimeError::FramePoolUnavailable => "FramePoolUnavailable",
        RuntimeError::InvalidActionCompletion => "InvalidActionCompletion",
        RuntimeError::InvalidTimerFire => "InvalidTimerFire",
        RuntimeError::UnsupportedFullRecoveryHydration => "UnsupportedFullRecoveryHydration",
        RuntimeError::InvalidRecoveryHydration => "InvalidRecoveryHydration",
    }
}

fn ipc_error_codes() -> Vec<(DiagnosticCode, &'static str)> {
    let samples = [
        IpcError::Full,
        IpcError::Disconnected,
        IpcError::PayloadTooLarge {
            actual: 2,
            limit: 1,
        },
        IpcError::InvalidMagic { actual: 0 },
        IpcError::UnsupportedVersion { actual: 0 },
        IpcError::UnknownCommand(0),
        IpcError::ReservedNonZero { actual: 1 },
        IpcError::PayloadLengthMismatch {
            header: 2,
            actual: 1,
        },
        IpcError::HeaderEncodeFailed,
        IpcError::HeaderDecodeFailed,
        IpcError::PayloadLengthOutOfRange { actual: 1 },
        IpcError::PayloadEncodeFailed,
        IpcError::PayloadDecodeFailed,
        IpcError::ResponseDecodeFailed,
    ];
    samples
        .iter()
        .map(|error| (error.diagnostic_code(), ipc_error_variant_name(error)))
        .collect()
}

fn ipc_error_variant_name(error: &IpcError) -> &'static str {
    match error {
        IpcError::Full => "Full",
        IpcError::Disconnected => "Disconnected",
        IpcError::PayloadTooLarge { .. } => "PayloadTooLarge",
        IpcError::InvalidMagic { .. } => "InvalidMagic",
        IpcError::UnsupportedVersion { .. } => "UnsupportedVersion",
        IpcError::UnknownCommand(_) => "UnknownCommand",
        IpcError::ReservedNonZero { .. } => "ReservedNonZero",
        IpcError::PayloadLengthMismatch { .. } => "PayloadLengthMismatch",
        IpcError::HeaderEncodeFailed => "HeaderEncodeFailed",
        IpcError::HeaderDecodeFailed => "HeaderDecodeFailed",
        IpcError::PayloadLengthOutOfRange { .. } => "PayloadLengthOutOfRange",
        IpcError::PayloadEncodeFailed => "PayloadEncodeFailed",
        IpcError::PayloadDecodeFailed => "PayloadDecodeFailed",
        IpcError::ResponseDecodeFailed => "ResponseDecodeFailed",
    }
}

fn journal_error_codes() -> Vec<(DiagnosticCode, &'static str)> {
    let samples = [
        JournalError::KeyCapacity,
        JournalError::DuplicateEvent {
            run: run(),
            seq: seq(),
        },
        JournalError::WriteLockPoisoned,
        JournalError::QueueCapacity,
        JournalError::QueueFull,
        JournalError::WrongRun {
            expected: run(),
            actual: run(),
        },
        JournalError::SequenceGap {
            expected: seq(),
            actual: seq(),
        },
        JournalError::SequenceOverflow,
        JournalError::BadMagic { found: 0 },
        JournalError::UnsupportedSchemaVersion { version: 0 },
        JournalError::MigrationRequired { from: 0, to: 1 },
        JournalError::UnknownRecordKind { kind: 0 },
        JournalError::RecordKindFamilyMismatch { magic: 0, kind: 0 },
        JournalError::HeaderLengthMismatch { found: 0 },
        JournalError::PayloadTooLarge { len: 2, max: 1 },
        JournalError::HeaderChecksumMismatch,
        JournalError::PayloadDigestMismatch,
        JournalError::UnexpectedEof,
        JournalError::PostcardDecodeFailed,
        JournalError::QueueShutdown,
    ];
    std::iter::once((JournalError::FJALL_CODE, "Fjall"))
        .chain(std::iter::once((JournalError::ENCODE_CODE, "Encode")))
        .chain(
            samples
                .iter()
                .map(|error| (error.diagnostic_code(), journal_error_variant_name(error))),
        )
        .collect()
}

fn journal_error_variant_name(error: &JournalError) -> &'static str {
    match error {
        JournalError::Fjall(_) => "Fjall",
        JournalError::Encode(_) => "Encode",
        JournalError::KeyCapacity => "KeyCapacity",
        JournalError::DuplicateEvent { .. } => "DuplicateEvent",
        JournalError::WriteLockPoisoned => "WriteLockPoisoned",
        JournalError::QueueCapacity => "QueueCapacity",
        JournalError::QueueFull => "QueueFull",
        JournalError::WrongRun { .. } => "WrongRun",
        JournalError::SequenceGap { .. } => "SequenceGap",
        JournalError::SequenceOverflow => "SequenceOverflow",
        JournalError::BadMagic { .. } => "BadMagic",
        JournalError::UnsupportedSchemaVersion { .. } => "UnsupportedSchemaVersion",
        JournalError::MigrationRequired { .. } => "MigrationRequired",
        JournalError::UnknownRecordKind { .. } => "UnknownRecordKind",
        JournalError::RecordKindFamilyMismatch { .. } => "RecordKindFamilyMismatch",
        JournalError::HeaderLengthMismatch { .. } => "HeaderLengthMismatch",
        JournalError::PayloadTooLarge { .. } => "PayloadTooLarge",
        JournalError::HeaderChecksumMismatch => "HeaderChecksumMismatch",
        JournalError::PayloadDigestMismatch => "PayloadDigestMismatch",
        JournalError::UnexpectedEof => "UnexpectedEof",
        JournalError::PostcardDecodeFailed => "PostcardDecodeFailed",
        JournalError::QueueShutdown => "QueueShutdown",
    }
}

fn yaml_variant_count() -> usize {
    let samples = [
        YamlError::UnsupportedFeature { feature: "f" },
        YamlError::DuplicateKey { key: b("k") },
        YamlError::AnchorAliasMerge,
        YamlError::CustomTag { tag: b("t") },
        YamlError::BinaryScalar,
        YamlError::MultipleDocuments { count: 2 },
        YamlError::AmbiguousScalar { scalar: b("on") },
        YamlError::SourceTooLarge { size: 2, max: 1 },
        YamlError::NestingTooDeep { depth: 2, max: 1 },
        YamlError::NodeLimitExceeded { count: 2, max: 1 },
        YamlError::ScalarTooLong { len: 2, max: 1 },
        YamlError::SequenceTooLong { len: 2, max: 1 },
        YamlError::MappingTooLarge { count: 2, max: 1 },
        YamlError::UnknownField { field: b("f") },
        YamlError::EmptySource,
        YamlError::MissingField { field: "f" },
        YamlError::FieldShape {
            field: "f",
            expected: "map",
        },
        YamlError::ParseError {
            line: 1,
            reason: b("r"),
        },
        YamlError::ForbiddenFeature { detail: "d" },
    ];
    samples.iter().map(yaml_error_variant_name).count()
}

fn yaml_error_variant_name(error: &YamlError) -> &'static str {
    match error {
        YamlError::UnsupportedFeature { .. } => "UnsupportedFeature",
        YamlError::DuplicateKey { .. } => "DuplicateKey",
        YamlError::AnchorAliasMerge => "AnchorAliasMerge",
        YamlError::CustomTag { .. } => "CustomTag",
        YamlError::BinaryScalar => "BinaryScalar",
        YamlError::MultipleDocuments { .. } => "MultipleDocuments",
        YamlError::AmbiguousScalar { .. } => "AmbiguousScalar",
        YamlError::SourceTooLarge { .. } => "SourceTooLarge",
        YamlError::NestingTooDeep { .. } => "NestingTooDeep",
        YamlError::NodeLimitExceeded { .. } => "NodeLimitExceeded",
        YamlError::ScalarTooLong { .. } => "ScalarTooLong",
        YamlError::SequenceTooLong { .. } => "SequenceTooLong",
        YamlError::MappingTooLarge { .. } => "MappingTooLarge",
        YamlError::UnknownField { .. } => "UnknownField",
        YamlError::EmptySource => "EmptySource",
        YamlError::MissingField { .. } => "MissingField",
        YamlError::FieldShape { .. } => "FieldShape",
        YamlError::ParseError { .. } => "ParseError",
        YamlError::ForbiddenFeature { .. } => "ForbiddenFeature",
    }
}

fn expr_variant_count() -> usize {
    let samples = [
        ExprError::UnexpectedToken { token: s("t") },
        ExprError::UnexpectedEof,
        ExprError::UnknownOperator { op: s("+") },
        ExprError::UnknownHelper { helper: s("h") },
        ExprError::StackOverflow { max: 1 },
        ExprError::StackUnderflow,
        ExprError::TypeMismatch {
            expected: s("a"),
            found: s("b"),
        },
        ExprError::DivisionByZero,
        ExprError::IntegerOverflow,
        ExprError::InvalidReference { reference: s("r") },
        ExprError::ExpressionTooLong { len: 2, max: 1 },
        ExprError::UnterminatedString,
        ExprError::IntegerOutOfRange,
        ExprError::UnexpectedChar { ch: '?' },
        ExprError::ParseDepthExceeded { max: 1 },
        ExprError::TooManyHelperArgs { len: 2, max: 1 },
        ExprError::HelperArityMismatch {
            helper: s("h"),
            expected: 1,
            actual: 2,
        },
        ExprError::BytecodeTooLong { len: 2, max: 1 },
        ExprError::ConstantPoolOverflow,
    ];
    let count = samples.iter().map(expr_error_variant_name).count();
    let _ = expr_error_variant_name(&ExprError::UnsupportedLiteral { literal: s("lit") });
    count.saturating_add(1)
}

fn expr_error_variant_name(error: &ExprError) -> &'static str {
    match error {
        ExprError::UnexpectedToken { .. } => "UnexpectedToken",
        ExprError::UnexpectedEof => "UnexpectedEof",
        ExprError::UnknownOperator { .. } => "UnknownOperator",
        ExprError::UnknownHelper { .. } => "UnknownHelper",
        ExprError::StackOverflow { .. } => "StackOverflow",
        ExprError::StackUnderflow => "StackUnderflow",
        ExprError::TypeMismatch { .. } => "TypeMismatch",
        ExprError::DivisionByZero => "DivisionByZero",
        ExprError::IntegerOverflow => "IntegerOverflow",
        ExprError::InvalidReference { .. } => "InvalidReference",
        ExprError::ExpressionTooLong { .. } => "ExpressionTooLong",
        ExprError::UnterminatedString => "UnterminatedString",
        ExprError::IntegerOutOfRange => "IntegerOutOfRange",
        ExprError::UnexpectedChar { .. } => "UnexpectedChar",
        ExprError::ParseDepthExceeded { .. } => "ParseDepthExceeded",
        ExprError::TooManyHelperArgs { .. } => "TooManyHelperArgs",
        ExprError::HelperArityMismatch { .. } => "HelperArityMismatch",
        ExprError::BytecodeTooLong { .. } => "BytecodeTooLong",
        ExprError::ConstantPoolOverflow => "ConstantPoolOverflow",
        ExprError::UnsupportedLiteral { .. } => "UnsupportedLiteral",
    }
}

fn action_variant_count() -> usize {
    let samples = [
        ActionError::UnknownAction {
            action: ActionId::new(1),
        },
        ActionError::InvalidTicket,
        ActionError::PayloadTooLarge {
            max_bytes: 1,
            actual_bytes: 2,
        },
        ActionError::OutputSlotOutOfBounds {
            slot: 2,
            max_slots: 1,
        },
        ActionError::NonIdempotentReplayBlocked,
        ActionError::CompletionAlreadyRecorded,
        ActionError::QueueFull,
        ActionError::EncodingFailed,
        ActionError::DispatchFailed,
    ];
    samples.iter().map(action_error_variant_name).count()
}

fn action_error_variant_name(error: &ActionError) -> &'static str {
    match error {
        ActionError::UnknownAction { .. } => "UnknownAction",
        ActionError::InvalidTicket => "InvalidTicket",
        ActionError::PayloadTooLarge { .. } => "PayloadTooLarge",
        ActionError::OutputSlotOutOfBounds { .. } => "OutputSlotOutOfBounds",
        ActionError::NonIdempotentReplayBlocked => "NonIdempotentReplayBlocked",
        ActionError::CompletionAlreadyRecorded => "CompletionAlreadyRecorded",
        ActionError::QueueFull => "QueueFull",
        ActionError::EncodingFailed => "EncodingFailed",
        ActionError::DispatchFailed => "DispatchFailed",
    }
}

fn workflow_variant_count() -> usize {
    let samples = [
        WorkflowError::EmptyNodes,
        WorkflowError::EntryOutOfBounds { entry: step() },
        WorkflowError::StepOutOfBounds { step: step() },
        WorkflowError::SlotOutOfBounds { slot: slot() },
        WorkflowError::ConstOutOfBounds {
            constant: ConstIdx::new(1),
        },
        WorkflowError::NodeIdMismatch {
            expected: step(),
            actual: StepIdx::new(2),
        },
        WorkflowError::Expression(CoreError::QueueFull),
        WorkflowError::ResourceContractExceeded { resource: "r" },
        WorkflowError::ResourceContractTooLarge { resource: "r" },
        WorkflowError::EmptyBranchTable,
    ];
    samples.iter().map(workflow_error_variant_name).count()
}

fn workflow_error_variant_name(error: &WorkflowError) -> &'static str {
    match error {
        WorkflowError::EmptyNodes => "EmptyNodes",
        WorkflowError::EntryOutOfBounds { .. } => "EntryOutOfBounds",
        WorkflowError::StepOutOfBounds { .. } => "StepOutOfBounds",
        WorkflowError::SlotOutOfBounds { .. } => "SlotOutOfBounds",
        WorkflowError::ConstOutOfBounds { .. } => "ConstOutOfBounds",
        WorkflowError::NodeIdMismatch { .. } => "NodeIdMismatch",
        WorkflowError::Expression(_) => "Expression",
        WorkflowError::ResourceContractExceeded { .. } => "ResourceContractExceeded",
        WorkflowError::ResourceContractTooLarge { .. } => "ResourceContractTooLarge",
        WorkflowError::EmptyBranchTable => "EmptyBranchTable",
    }
}

fn diagnostic_parse_variant_count() -> usize {
    let samples = [
        DiagnosticCodeParseError::InvalidFormat,
        DiagnosticCodeParseError::UnsupportedCode,
    ];
    samples.iter().map(diagnostic_parse_variant_name).count()
}

fn diagnostic_parse_variant_name(error: &DiagnosticCodeParseError) -> &'static str {
    match error {
        DiagnosticCodeParseError::InvalidFormat => "InvalidFormat",
        DiagnosticCodeParseError::UnsupportedCode => "UnsupportedCode",
    }
}

fn compile_constructible_variant_count() -> usize {
    let samples = compile_constructible_samples();
    let count = samples.iter().map(compile_error_variant_name).count();
    let _ = compile_error_variant_name(&compile_utf8_error());
    count.saturating_add(1)
}

fn compile_constructible_samples() -> [CompileError; 68] {
    [
        CompileError::SourceTooLarge {
            actual: 2,
            limit: 1,
        },
        CompileError::EmptySource,
        CompileError::DocumentCount { count: 2 },
        CompileError::TopLevelNotMapping,
        CompileError::NonStringKey { mark: mark() },
        CompileError::DuplicateKey {
            key: b("k"),
            mark: mark(),
        },
        CompileError::AliasForbidden { mark: mark() },
        CompileError::AnchorForbidden { mark: mark() },
        CompileError::MergeKeyForbidden { mark: mark() },
        CompileError::TagForbidden { mark: mark() },
        CompileError::BadValue,
        CompileError::FloatForbidden,
        CompileError::DepthLimit { depth: 2, limit: 1 },
        CompileError::NodeLimit { limit: 1 },
        CompileError::SequenceLimit {
            actual: 2,
            limit: 1,
        },
        CompileError::MappingLimit {
            actual: 2,
            limit: 1,
        },
        CompileError::ScalarLimit {
            actual: 2,
            limit: 1,
        },
        CompileError::Workflow(WorkflowError::EmptyNodes),
        CompileError::MissingField { field: "f" },
        CompileError::UnknownTopLevelField { field: b("f") },
        CompileError::InvalidVersion { actual: b("v") },
        CompileError::InvalidTriggerCount { count: 2 },
        CompileError::UnknownTriggerKind { trigger: b("t") },
        CompileError::TriggerShape {
            trigger: b("t"),
            expected: "map",
        },
        CompileError::UnknownTriggerField {
            trigger: "t",
            field: b("f"),
        },
        CompileError::MissingTriggerField {
            trigger: "t",
            field: "f",
        },
        CompileError::InvalidTriggerField {
            trigger: "t",
            field: "f",
            expected: "e",
        },
        CompileError::FieldShape {
            field: "f",
            expected: "map",
        },
        CompileError::UnknownInputSchemaField { field: b("f") },
        CompileError::InvalidInputSchema {
            field: "f",
            expected: "e",
        },
        CompileError::UnsupportedTopLevelResult,
        CompileError::EmptySteps,
        CompileError::InvalidName {
            field: "f",
            value: b("v"),
        },
        CompileError::MissingStepId { step: 0 },
        CompileError::DuplicateStepId { id: b("id") },
        CompileError::StepShape { step: 0 },
        CompileError::UnknownStepField {
            step: 0,
            field: b("f"),
        },
        CompileError::UnknownStepPrimitiveField {
            step: 0,
            primitive: "do",
            field: b("f"),
        },
        CompileError::MissingStepPrimitive { step: 0 },
        CompileError::MultipleStepPrimitives { step: 0 },
        CompileError::UnsupportedStepPrimitive {
            step: 0,
            primitive: "x",
        },
        CompileError::UnsupportedStepControlField {
            step: 0,
            field: b("f"),
        },
        CompileError::MissingStepField {
            step: 0,
            field: "f",
        },
        CompileError::StepFieldShape {
            step: 0,
            field: "f",
            expected: "e",
        },
        CompileError::StepIndexOutOfRange { value: 1 },
        CompileError::SlotIndexOutOfRange { value: -1 },
        CompileError::BranchTargetOutOfRange { value: -1 },
        CompileError::BackwardBranchTarget { step: 1, target: 0 },
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "p",
            field: "f",
            value: 2,
            limit: 1,
        },
        CompileError::LastStepMustFinish,
        CompileError::UnsupportedConstantValue { step: 0 },
        CompileError::UnknownReferenceRoot {
            reference: b("$x"),
            root: b("x"),
        },
        CompileError::IllegalReference {
            reference: b("$runtime"),
        },
        CompileError::UnknownReferenceName {
            kind: "input",
            reference: b("$input.x"),
            name: b("x"),
        },
        CompileError::UnsupportedAccessorReference {
            reference: b("$input.x.y"),
            root: b("x"),
            path: b("y"),
        },
        CompileError::UnknownStepTarget { step: 0, target: 1 },
        CompileError::UnreachableStep { step: 1 },
        CompileError::TypeMismatch {
            field: "f",
            expected: "a",
            found: "b",
        },
        CompileError::UnknownSlotType {
            field: "f",
            slot: 1,
        },
        CompileError::SecretTaintLeak { field: "f" },
        CompileError::ExpressionUnexpectedChar {
            expression: b("?"),
            index: 0,
            found: '?',
        },
        CompileError::ExpressionUnterminatedString {
            expression: b("\""),
            index: 0,
        },
        CompileError::ExpressionIntegerOutOfRange {
            expression: b("1"),
            index: 0,
        },
        CompileError::ExpressionLimitExceeded {
            expression: b("1"),
            limit: "tokens",
            max: 1,
        },
        CompileError::ExpressionUnexpectedToken {
            expression: b("1"),
            index: 0,
            expected: "term",
        },
        CompileError::ExpressionUnknownIdentifier {
            expression: b("x"),
            index: 0,
            identifier: b("x"),
        },
        CompileError::ExpressionLoweringUnsupported { feature: "helper" },
        CompileError::ExpressionHelperArity {
            helper: "h",
            expected: 1,
            actual: 2,
        },
    ]
}

fn compile_error_variant_name(error: &CompileError) -> &'static str {
    match error {
        CompileError::SourceTooLarge { .. } => "SourceTooLarge",
        CompileError::Utf8(_) => "Utf8",
        CompileError::EmptySource => "EmptySource",
        CompileError::Parse(_) => "Parse",
        CompileError::DocumentCount { .. } => "DocumentCount",
        CompileError::TopLevelNotMapping => "TopLevelNotMapping",
        CompileError::NonStringKey { .. } => "NonStringKey",
        CompileError::DuplicateKey { .. } => "DuplicateKey",
        CompileError::AliasForbidden { .. } => "AliasForbidden",
        CompileError::AnchorForbidden { .. } => "AnchorForbidden",
        CompileError::MergeKeyForbidden { .. } => "MergeKeyForbidden",
        CompileError::TagForbidden { .. } => "TagForbidden",
        CompileError::BadValue => "BadValue",
        CompileError::FloatForbidden => "FloatForbidden",
        CompileError::DepthLimit { .. } => "DepthLimit",
        CompileError::NodeLimit { .. } => "NodeLimit",
        CompileError::SequenceLimit { .. } => "SequenceLimit",
        CompileError::MappingLimit { .. } => "MappingLimit",
        CompileError::ScalarLimit { .. } => "ScalarLimit",
        CompileError::Workflow(_) => "Workflow",
        CompileError::MissingField { .. } => "MissingField",
        CompileError::UnknownTopLevelField { .. } => "UnknownTopLevelField",
        CompileError::InvalidVersion { .. } => "InvalidVersion",
        CompileError::InvalidTriggerCount { .. } => "InvalidTriggerCount",
        CompileError::UnknownTriggerKind { .. } => "UnknownTriggerKind",
        CompileError::TriggerShape { .. } => "TriggerShape",
        CompileError::UnknownTriggerField { .. } => "UnknownTriggerField",
        CompileError::MissingTriggerField { .. } => "MissingTriggerField",
        CompileError::InvalidTriggerField { .. } => "InvalidTriggerField",
        CompileError::FieldShape { .. } => "FieldShape",
        CompileError::UnknownInputSchemaField { .. } => "UnknownInputSchemaField",
        CompileError::InvalidInputSchema { .. } => "InvalidInputSchema",
        CompileError::UnsupportedTopLevelResult => "UnsupportedTopLevelResult",
        CompileError::EmptySteps => "EmptySteps",
        CompileError::InvalidName { .. } => "InvalidName",
        CompileError::MissingStepId { .. } => "MissingStepId",
        CompileError::DuplicateStepId { .. } => "DuplicateStepId",
        CompileError::StepShape { .. } => "StepShape",
        CompileError::UnknownStepField { .. } => "UnknownStepField",
        CompileError::UnknownStepPrimitiveField { .. } => "UnknownStepPrimitiveField",
        CompileError::MissingStepPrimitive { .. } => "MissingStepPrimitive",
        CompileError::MultipleStepPrimitives { .. } => "MultipleStepPrimitives",
        CompileError::UnsupportedStepPrimitive { .. } => "UnsupportedStepPrimitive",
        CompileError::UnsupportedStepControlField { .. } => "UnsupportedStepControlField",
        CompileError::MissingStepField { .. } => "MissingStepField",
        CompileError::StepFieldShape { .. } => "StepFieldShape",
        CompileError::StepIndexOutOfRange { .. } => "StepIndexOutOfRange",
        CompileError::SlotIndexOutOfRange { .. } => "SlotIndexOutOfRange",
        CompileError::BranchTargetOutOfRange { .. } => "BranchTargetOutOfRange",
        CompileError::BackwardBranchTarget { .. } => "BackwardBranchTarget",
        CompileError::PrimitiveLoweringLimitExceeded { .. } => "PrimitiveLoweringLimitExceeded",
        CompileError::LastStepMustFinish => "LastStepMustFinish",
        CompileError::UnsupportedConstantValue { .. } => "UnsupportedConstantValue",
        CompileError::UnknownReferenceRoot { .. } => "UnknownReferenceRoot",
        CompileError::IllegalReference { .. } => "IllegalReference",
        CompileError::UnknownReferenceName { .. } => "UnknownReferenceName",
        CompileError::UnsupportedAccessorReference { .. } => "UnsupportedAccessorReference",
        CompileError::UnknownStepTarget { .. } => "UnknownStepTarget",
        CompileError::UnreachableStep { .. } => "UnreachableStep",
        CompileError::TypeMismatch { .. } => "TypeMismatch",
        CompileError::UnknownSlotType { .. } => "UnknownSlotType",
        CompileError::SecretTaintLeak { .. } => "SecretTaintLeak",
        CompileError::ExpressionUnexpectedChar { .. } => "ExpressionUnexpectedChar",
        CompileError::ExpressionUnterminatedString { .. } => "ExpressionUnterminatedString",
        CompileError::ExpressionIntegerOutOfRange { .. } => "ExpressionIntegerOutOfRange",
        CompileError::ExpressionLimitExceeded { .. } => "ExpressionLimitExceeded",
        CompileError::ExpressionUnexpectedToken { .. } => "ExpressionUnexpectedToken",
        CompileError::ExpressionUnknownIdentifier { .. } => "ExpressionUnknownIdentifier",
        CompileError::ExpressionLoweringUnsupported { .. } => "ExpressionLoweringUnsupported",
        CompileError::ExpressionHelperArity { .. } => "ExpressionHelperArity",
    }
}

fn codegen_constructible_variant_count() -> usize {
    let samples = [
        CodegenError::UnsupportedIr { feature: "f" },
        CodegenError::FormatBufferOverflow,
        CodegenError::RustfmtFailed { detail: s("d") },
        CodegenError::CompileCheckFailed { detail: s("d") },
        CodegenError::SemanticMismatch { detail: s("d") },
        CodegenError::TrybuildFixture { detail: s("d") },
    ];
    let count = samples.iter().map(codegen_error_variant_name).count();
    let _ = codegen_error_variant_name(&codegen_io_error());
    count
}

fn codegen_error_variant_name(error: &CodegenError) -> &'static str {
    match error {
        CodegenError::UnsupportedIr { .. } => "UnsupportedIr",
        CodegenError::FormatBufferOverflow => "FormatBufferOverflow",
        CodegenError::RustfmtFailed { .. } => "RustfmtFailed",
        CodegenError::CompileCheckFailed { .. } => "CompileCheckFailed",
        CodegenError::SemanticMismatch { .. } => "SemanticMismatch",
        CodegenError::Io(_) => "Io",
        CodegenError::TrybuildFixture { .. } => "TrybuildFixture",
    }
}

fn runtime_engine_variant_count() -> usize {
    let samples = [
        RuntimeEngineError::Core(CoreError::QueueFull),
        RuntimeEngineError::Action(ActionError::QueueFull),
        RuntimeEngineError::RetryExhausted {
            action: ActionId::new(1),
            attempts: 1,
        },
        RuntimeEngineError::TaintViolation { step: step() },
    ];
    samples
        .iter()
        .map(runtime_engine_error_variant_name)
        .count()
}

fn runtime_engine_error_variant_name(error: &RuntimeEngineError) -> &'static str {
    match error {
        RuntimeEngineError::Core(_) => "Core",
        RuntimeEngineError::Action(_) => "Action",
        RuntimeEngineError::RetryExhausted { .. } => "RetryExhausted",
        RuntimeEngineError::TaintViolation { .. } => "TaintViolation",
    }
}

fn workflow_resolution_variant_count() -> usize {
    let samples = [
        WorkflowResolutionError::Required,
        WorkflowResolutionError::NotFound,
        WorkflowResolutionError::InvalidArtifact,
    ];
    samples
        .iter()
        .map(workflow_resolution_error_variant_name)
        .count()
}

fn workflow_resolution_error_variant_name(error: &WorkflowResolutionError) -> &'static str {
    match error {
        WorkflowResolutionError::Required => "Required",
        WorkflowResolutionError::NotFound => "NotFound",
        WorkflowResolutionError::InvalidArtifact => "InvalidArtifact",
    }
}

fn ipc_client_constructible_variant_count() -> usize {
    let samples = [
        IpcClientError::ConnectFailed { source: io_error() },
        IpcClientError::IoError { source: io_error() },
        IpcClientError::FrameError {
            source: IpcError::Full,
        },
        IpcClientError::EncodeFailed,
    ];
    samples.iter().map(ipc_client_error_variant_name).count()
}

fn ipc_client_error_variant_name(error: &IpcClientError) -> &'static str {
    match error {
        IpcClientError::ConnectFailed { .. } => "ConnectFailed",
        IpcClientError::IoError { .. } => "IoError",
        IpcClientError::FrameError { .. } => "FrameError",
        IpcClientError::EncodeFailed => "EncodeFailed",
    }
}

fn ipc_server_constructible_variant_count() -> usize {
    let samples = [
        IpcServerError::BindFailed { source: io_error() },
        IpcServerError::PollFailed { source: io_error() },
        IpcServerError::AcceptFailed { source: io_error() },
        IpcServerError::TooManyClients,
        IpcServerError::ResponseEncodeFailed,
        IpcServerError::ResponseWriteFailed { source: io_error() },
        IpcServerError::IncompleteFrame,
        IpcServerError::ReadBufferTooLarge,
        IpcServerError::FrameInvalid {
            source: IpcError::Full,
        },
    ];
    samples.iter().map(ipc_server_error_variant_name).count()
}

fn ipc_server_error_variant_name(error: &IpcServerError) -> &'static str {
    match error {
        IpcServerError::BindFailed { .. } => "BindFailed",
        IpcServerError::PollFailed { .. } => "PollFailed",
        IpcServerError::AcceptFailed { .. } => "AcceptFailed",
        IpcServerError::TooManyClients => "TooManyClients",
        IpcServerError::ResponseEncodeFailed => "ResponseEncodeFailed",
        IpcServerError::ResponseWriteFailed { .. } => "ResponseWriteFailed",
        IpcServerError::IncompleteFrame => "IncompleteFrame",
        IpcServerError::ReadBufferTooLarge => "ReadBufferTooLarge",
        IpcServerError::FrameInvalid { .. } => "FrameInvalid",
    }
}

fn recovery_constructible_variant_count() -> usize {
    let digest = WorkflowDigest::from_bytes([1; 32]);
    let samples = [
        RecoveryError::Journal(JournalError::KeyCapacity),
        RecoveryError::WorkflowSourceDigestMismatch {
            expected: digest,
            found: digest,
        },
        RecoveryError::CompiledIrDigestMismatch {
            expected: digest,
            found: digest,
        },
        RecoveryError::ActionAbiMismatch {
            action_id: ActionId::new(1),
        },
        RecoveryError::PolicyDigestMismatch { step: step() },
        RecoveryError::NonIdempotentActionBlocked {
            action: ActionId::new(1),
            step: step(),
        },
        RecoveryError::ReplayDivergence {
            step: step(),
            detail: s("d"),
        },
        RecoveryError::NoRecoveryData { run: run() },
        RecoveryError::CorruptSnapshot {
            run: run(),
            seq: seq(),
        },
        RecoveryError::TerminalStateMismatch {
            expected: s("done"),
            found: s("running"),
        },
        RecoveryError::FrameDimensionOverflow { run: run() },
    ];
    samples.iter().map(recovery_error_variant_name).count()
}

fn recovery_error_variant_name(error: &RecoveryError) -> &'static str {
    match error {
        RecoveryError::Journal(_) => "Journal",
        RecoveryError::WorkflowSourceDigestMismatch { .. } => "WorkflowSourceDigestMismatch",
        RecoveryError::CompiledIrDigestMismatch { .. } => "CompiledIrDigestMismatch",
        RecoveryError::ActionAbiMismatch { .. } => "ActionAbiMismatch",
        RecoveryError::PolicyDigestMismatch { .. } => "PolicyDigestMismatch",
        RecoveryError::NonIdempotentActionBlocked { .. } => "NonIdempotentActionBlocked",
        RecoveryError::ReplayDivergence { .. } => "ReplayDivergence",
        RecoveryError::NoRecoveryData { .. } => "NoRecoveryData",
        RecoveryError::CorruptSnapshot { .. } => "CorruptSnapshot",
        RecoveryError::TerminalStateMismatch { .. } => "TerminalStateMismatch",
        RecoveryError::FrameDimensionOverflow { .. } => "FrameDimensionOverflow",
    }
}

fn compile_utf8_error() -> CompileError {
    let invalid_byte = u8::MAX;
    let bytes = [invalid_byte];
    match std::str::from_utf8(&bytes) {
        Ok(_) => CompileError::EmptySource,
        Err(error) => CompileError::Utf8(error),
    }
}

fn codegen_io_error() -> CodegenError {
    CodegenError::Io(io_error())
}

fn io_error() -> io::Error {
    io::Error::from(io::ErrorKind::Other)
}

fn mark() -> SourceMark {
    SourceMark {
        index: 0,
        end_index: 0,
        line: 0,
        column: 0,
        available: false,
    }
}

fn step() -> StepIdx {
    StepIdx::new(1)
}

fn slot() -> SlotIdx {
    SlotIdx::new(1)
}

fn run() -> vb_core::RunId {
    vb_core::RunId::new(1)
}

fn seq() -> EventSeq {
    EventSeq::new(1)
}

fn s(value: &str) -> String {
    String::from(value)
}

fn b(value: &str) -> Box<str> {
    Box::<str>::from(value)
}
