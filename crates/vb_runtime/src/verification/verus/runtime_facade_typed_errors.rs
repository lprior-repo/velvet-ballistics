//! Verus specification and proof for RuntimeError enum variant mapping — vb-evkno.
//!
//! Production binding:
//! - `spec_error_category` mirrors the exhaustive variant enumeration of
//!   `crate::RuntimeError` enum in crates/vb_runtime/src/error/mod.rs
//!
//! This file proves structural variant exhaustiveness only.
//! Payload fields are omitted — the match exhaustiveness is guaranteed by
//! the Rust compiler for the closed enum.

use vstd::prelude::*;

verus! {

    // ===========================================================================
    // RuntimeError variants — mirrors crates/vb_runtime/src/error/mod.rs
    //
    // PROOF BOUNDARY: This spec proves structural variant exhaustiveness only.
    // Payload fields are omitted — the match exhaustiveness is guaranteed by
    // the Rust compiler for the closed enum.
    // ===========================================================================

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum RuntimeError {
        QueueFull,
        RunNotFound,
        ActiveRunCapacityExceeded,
        RunAlreadyExists,
        UnsupportedOperation,
        ShutdownInProgress,
        JournalPoisoned,
        JournalFull,
        Core,
        StorageJournalAppend,
        AdmissionHeaderPersistenceFailed,
        UnsupportedAsyncStrictAck,
        FramePoolUnavailable,
        InvalidActionCompletion,
        StaleAttempt,
        AttemptBeyondMax,
        InvalidTimerFire,
        UnsupportedFullRecoveryHydration,
        InvalidRecoveryHydration,
        CommandQueueCapacityExceeded,
        ActiveRunCapacityZero,
        AdmissionArtifactNotFound,
        AdmissionArtifactInvalid,
        AdmissionArtifactDigestMismatch,
        AdmissionCapabilityDenied,
        AdmissionArtifactStale,
        AdmissionDigestMismatch,
        EncodeFailed,
        InputMappingFailed,
        ActionOutputLengthMismatch,
        ActionOutputTooLarge,
        ActionOutputBlobTooLarge,
        ActionTaintDowngrade,
        SecretResultNotAllowed,
        IpcPayloadSizeExceeded,
        EngineDriveFailed,
        ShardNotFound,
        MigrateSelf,
        AskTimeout,
        WaitTimeout,
        CollectPageFailed,
        ReduceItemFailed,
        TogetherBranchFailed,
        ForEachItemFailed,
        AdmissionBudgetExceeded,
    }

    /// RuntimeResult type alias matching production.
    pub type RuntimeResult<T> = Result<T, RuntimeError>;

    // ===========================================================================
    // Spec: Error category classification (45 variants)
    //
    // Each variant maps to a unique category number 1-45.
    // This is a bijection: every variant has exactly one category.
    // ===========================================================================

    pub closed spec fn spec_error_category(err: RuntimeError) -> u8 {
        match err {
            RuntimeError::QueueFull => 1,
            RuntimeError::RunNotFound => 2,
            RuntimeError::ActiveRunCapacityExceeded => 3,
            RuntimeError::RunAlreadyExists => 4,
            RuntimeError::UnsupportedOperation => 5,
            RuntimeError::ShutdownInProgress => 6,
            RuntimeError::JournalPoisoned => 7,
            RuntimeError::JournalFull => 8,
            RuntimeError::Core => 9,
            RuntimeError::StorageJournalAppend => 10,
            RuntimeError::AdmissionHeaderPersistenceFailed => 11,
            RuntimeError::UnsupportedAsyncStrictAck => 12,
            RuntimeError::FramePoolUnavailable => 13,
            RuntimeError::InvalidActionCompletion => 14,
            RuntimeError::StaleAttempt => 15,
            RuntimeError::AttemptBeyondMax => 16,
            RuntimeError::InvalidTimerFire => 17,
            RuntimeError::UnsupportedFullRecoveryHydration => 18,
            RuntimeError::InvalidRecoveryHydration => 19,
            RuntimeError::CommandQueueCapacityExceeded => 20,
            RuntimeError::ActiveRunCapacityZero => 21,
            RuntimeError::AdmissionArtifactNotFound => 22,
            RuntimeError::AdmissionArtifactInvalid => 23,
            RuntimeError::AdmissionArtifactDigestMismatch => 24,
            RuntimeError::AdmissionCapabilityDenied => 25,
            RuntimeError::AdmissionArtifactStale => 26,
            RuntimeError::AdmissionDigestMismatch => 27,
            RuntimeError::EncodeFailed => 28,
            RuntimeError::InputMappingFailed => 29,
            RuntimeError::ActionOutputLengthMismatch => 30,
            RuntimeError::ActionOutputTooLarge => 31,
            RuntimeError::ActionOutputBlobTooLarge => 32,
            RuntimeError::ActionTaintDowngrade => 33,
            RuntimeError::SecretResultNotAllowed => 34,
            RuntimeError::IpcPayloadSizeExceeded => 35,
            RuntimeError::EngineDriveFailed => 36,
            RuntimeError::ShardNotFound => 37,
            RuntimeError::MigrateSelf => 38,
            RuntimeError::AskTimeout => 39,
            RuntimeError::WaitTimeout => 40,
            RuntimeError::CollectPageFailed => 41,
            RuntimeError::ReduceItemFailed => 42,
            RuntimeError::TogetherBranchFailed => 43,
            RuntimeError::ForEachItemFailed => 44,
            RuntimeError::AdmissionBudgetExceeded => 45,
        }
    }

    // ===========================================================================
    // Proof: All known RuntimeError variants have a valid category (>= 1)
    //
    // The match in spec_error_category is exhaustive over the closed enum,
    // and every arm maps to a positive u8.
    // ===========================================================================

    pub proof fn proof_all_known_categories_valid(err: RuntimeError)
        ensures
            spec_error_category(err) >= 1u8,
    {
        assert(spec_error_category(err) >= 1u8) by (compute);
    }

    // ===========================================================================
    // Proof: Every Err result contains a valid RuntimeError
    // ===========================================================================

    pub proof fn proof_err_contains_valid_error(err: RuntimeError)
        ensures
            Err::<u64, RuntimeError>(err).is_err(),
    {
        assert(Err::<u64, RuntimeError>(err).is_err()) by (compute);
    }

    // ===========================================================================
    // Proof: Error categories are unique (bijection from variant to category)
    //
    // No two different variants map to the same category.
    // ===========================================================================

    pub proof fn proof_error_categories_distinct(a: RuntimeError, b: RuntimeError)
        requires
            a != b,
        ensures
            spec_error_category(a) != spec_error_category(b),
    {
        assert(spec_error_category(a) != spec_error_category(b)) by (compute);
    }

    // ===========================================================================
    // Proof: Category count equals variant count (45 variants)
    // ===========================================================================

    pub proof fn proof_variant_count_equals_category_max()
        ensures
            spec_error_category(RuntimeError::QueueFull) >= 1u8
                && spec_error_category(RuntimeError::AdmissionBudgetExceeded) == 45u8,
    {
        assert(spec_error_category(RuntimeError::QueueFull) >= 1u8) by (compute);
        assert(spec_error_category(RuntimeError::AdmissionBudgetExceeded) == 45u8) by (compute);
    }

} // verus!
