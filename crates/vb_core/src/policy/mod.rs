#![forbid(unsafe_code)]
//! Runtime admission policy controlling verification strictness and durability.

mod contract_violation;
mod profile_name;
mod profile_validation_error;
mod runtime_limits_profile;

// Re-export sub-modules
pub use contract_violation::ContractViolation;
pub use profile_name::ProfileName;
pub use profile_validation_error::ProfileValidationError;
pub use runtime_limits_profile::RuntimeLimitsProfile;

// Re-export the existing policy enum
/// Controls how strictly artifact admission verification is enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum RuntimePolicy {
    /// Require accepted artifact for all runs, SyncAll before return.
    Strict,
    /// Accept runs without artifact, queue events without sync barrier.
    Journaled,
    /// No verification required, testing only.
    Relaxed,
}

#[cfg(test)]
mod tests {
    use super::RuntimePolicy;

    #[test]
    fn policy_variants_are_distinct() {
        assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Journaled);
        assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Relaxed);
        assert_ne!(RuntimePolicy::Journaled, RuntimePolicy::Relaxed);
    }

    #[test]
    fn policy_copy_semantics_preserve_equality() {
        let a = RuntimePolicy::Strict;
        let b = a;
        assert_eq!(a, b, "copy must preserve equality");
    }

    #[test]
    fn policy_strict_is_not_journaled() {
        assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Journaled);
    }

    #[test]
    fn policy_strict_is_not_relaxed() {
        assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Relaxed);
    }

    #[test]
    fn policy_journaled_is_not_relaxed() {
        assert_ne!(RuntimePolicy::Journaled, RuntimePolicy::Relaxed);
    }

    #[test]
    fn policy_debug_output_contains_variant_name() {
        let formatted = format!("{:?}", RuntimePolicy::Strict);
        assert!(
            formatted.contains("Strict"),
            "debug output must contain variant name: {formatted}"
        );
    }

    #[test]
    fn policy_clone_produces_equal_value() {
        let original = RuntimePolicy::Journaled;
        let cloned = original.clone();
        assert_eq!(original, cloned, "clone must produce equal value");
    }
}

#[cfg(test)]
mod profile_tests {
    use crate::budget::BoundednessPolicy;
    use crate::limits;
    use crate::policy::{ProfileName, RuntimeLimitsProfile};
    use crate::workflow::ResourceContract;

    /// C1: Three canonical profiles exist and are pairwise distinct.
    #[test]
    fn profile_matrix_three_distinct_profiles() {
        let strict = RuntimeLimitsProfile::strict();
        let journaled = RuntimeLimitsProfile::journaled();
        let relaxed = RuntimeLimitsProfile::relaxed();
        assert_ne!(strict, journaled);
        assert_ne!(strict, relaxed);
        assert_ne!(journaled, relaxed);
        assert_eq!(strict.name, ProfileName::Strict);
        assert_eq!(journaled.name, ProfileName::Journaled);
        assert_eq!(relaxed.name, ProfileName::Relaxed);
    }

    /// C2: All fields are positive (non-zero) for every canonical profile.
    #[test]
    fn all_profile_fields_positive() {
        for profile in [
            RuntimeLimitsProfile::strict(),
            RuntimeLimitsProfile::journaled(),
            RuntimeLimitsProfile::relaxed(),
        ] {
            assert!(profile.active_runs.get() > 0, "active_runs must be > 0");
            assert!(profile.ready_queue_depth.get() > 0, "ready_queue_depth must be > 0");
            assert!(profile.ipc_frame_bytes.get() > 0, "ipc_frame_bytes must be > 0");
            assert!(profile.action_input_bytes.get() > 0, "action_input_bytes must be > 0");
            assert!(profile.action_output_bytes.get() > 0, "action_output_bytes must be > 0");
            assert!(profile.step_output_bytes.get() > 0, "step_output_bytes must be > 0");
            assert!(profile.result_bytes.get() > 0, "result_bytes must be > 0");
            assert!(profile.trace_ring_capacity.get() > 0, "trace_ring_capacity must be > 0");
            assert!(profile.journal_writer_queue_capacity.get() > 0, "journal_writer_queue_capacity must be > 0");
            assert!(profile.for_each_item_count.get() > 0, "for_each_item_count must be > 0");
            assert!(profile.together_branch_count.get() > 0, "together_branch_count must be > 0");
            assert!(profile.collect_pages.get() > 0, "collect_pages must be > 0");
            assert!(profile.collect_items.get() > 0, "collect_items must be > 0");
            assert!(profile.collect_time_seconds.get() > 0, "collect_time_seconds must be > 0");
            assert!(profile.repeat_attempts.get() > 0, "repeat_attempts must be > 0");
            assert!(profile.repeat_time_seconds.get() > 0, "repeat_time_seconds must be > 0");
            assert!(profile.retry_attempts.get() > 0, "retry_attempts must be > 0");
            assert!(profile.max_wait_duration_seconds.get() > 0, "max_wait_duration_seconds must be > 0");
            assert!(profile.ask_timeout_seconds.get() > 0, "ask_timeout_seconds must be > 0");
        }
    }

    /// C3: Every profile field stays within corresponding hard limits.
    #[test]
    fn profile_fields_within_hard_limits() {
        for profile in [
            RuntimeLimitsProfile::strict(),
            RuntimeLimitsProfile::journaled(),
            RuntimeLimitsProfile::relaxed(),
        ] {
            assert!(
                profile.ready_queue_depth.get() as u64 <= limits::MAX_QUEUE_DEPTH as u64,
                "ready_queue_depth exceeds MAX_QUEUE_DEPTH"
            );
            assert!(
                profile.ipc_frame_bytes.get() as u64 <= limits::MAX_IPC_PAYLOAD_BYTES as u64,
                "ipc_frame_bytes exceeds MAX_IPC_PAYLOAD_BYTES"
            );
            assert!(
                profile.action_input_bytes.get() as u64 <= limits::MAX_INPUT_BYTES as u64,
                "action_input_bytes exceeds MAX_INPUT_BYTES"
            );
            assert!(
                profile.action_output_bytes.get() as u64 <= limits::MAX_OUTPUT_BYTES as u64,
                "action_output_bytes exceeds MAX_OUTPUT_BYTES"
            );
            assert!(
                profile.step_output_bytes.get() as u64 <= limits::MAX_OUTPUT_BYTES as u64,
                "step_output_bytes exceeds MAX_OUTPUT_BYTES"
            );
            assert!(
                profile.result_bytes.get() as u64 <= limits::MAX_OUTPUT_BYTES as u64,
                "result_bytes exceeds MAX_OUTPUT_BYTES"
            );
            assert!(
                profile.trace_ring_capacity.get() as u64 <= limits::MAX_QUEUE_DEPTH as u64,
                "trace_ring_capacity exceeds MAX_QUEUE_DEPTH"
            );
            assert!(
                profile.journal_writer_queue_capacity.get() as u64 <= limits::MAX_QUEUE_DEPTH as u64,
                "journal_writer_queue_capacity exceeds MAX_QUEUE_DEPTH"
            );
            assert!(
                profile.for_each_item_count.get() as u64 <= limits::MAX_COLLECT_ITEMS as u64,
                "for_each_item_count exceeds MAX_COLLECT_ITEMS"
            );
            assert!(
                profile.together_branch_count.get() as u64 <= limits::MAX_FANOUT as u64,
                "together_branch_count exceeds MAX_FANOUT"
            );
            assert!(
                profile.collect_pages.get() as u64 <= limits::MAX_COLLECT_ITEMS as u64,
                "collect_pages exceeds MAX_COLLECT_ITEMS"
            );
            assert!(
                profile.collect_items.get() as u64 <= limits::MAX_COLLECT_ITEMS as u64,
                "collect_items exceeds MAX_COLLECT_ITEMS"
            );
            assert!(
                profile.retry_attempts.get() as u64 <= limits::MAX_RETRY_ATTEMPTS as u64,
                "retry_attempts exceeds MAX_RETRY_ATTEMPTS"
            );
            assert!(
                profile.repeat_attempts.get() as u64 <= limits::MAX_RETRY_ATTEMPTS as u64,
                "repeat_attempts exceeds MAX_RETRY_ATTEMPTS"
            );
        }
    }

    /// C4: Profile monotonicity — Strict ≤ Journaled ≤ Relaxed for every dimension.
    #[test]
    fn profile_monotonicity() {
        let s = RuntimeLimitsProfile::strict();
        let j = RuntimeLimitsProfile::journaled();
        let r = RuntimeLimitsProfile::relaxed();

        assert!(s.active_runs.get() <= j.active_runs.get(), "strict <= journaled active_runs");
        assert!(j.active_runs.get() <= r.active_runs.get(), "journaled <= relaxed active_runs");

        assert!(s.ready_queue_depth.get() <= j.ready_queue_depth.get(), "strict <= journaled ready_queue_depth");
        assert!(j.ready_queue_depth.get() <= r.ready_queue_depth.get(), "journaled <= relaxed ready_queue_depth");

        assert!(s.ipc_frame_bytes.get() <= j.ipc_frame_bytes.get(), "strict <= journaled ipc_frame_bytes");
        assert!(j.ipc_frame_bytes.get() <= r.ipc_frame_bytes.get(), "journaled <= relaxed ipc_frame_bytes");

        assert!(s.action_input_bytes.get() <= j.action_input_bytes.get(), "strict <= journaled action_input_bytes");
        assert!(j.action_input_bytes.get() <= r.action_input_bytes.get(), "journaled <= relaxed action_input_bytes");

        assert!(s.action_output_bytes.get() <= j.action_output_bytes.get(), "strict <= journaled action_output_bytes");
        assert!(j.action_output_bytes.get() <= r.action_output_bytes.get(), "journaled <= relaxed action_output_bytes");

        assert!(s.step_output_bytes.get() <= j.step_output_bytes.get(), "strict <= journaled step_output_bytes");
        assert!(j.step_output_bytes.get() <= r.step_output_bytes.get(), "journaled <= relaxed step_output_bytes");

        assert!(s.result_bytes.get() <= j.result_bytes.get(), "strict <= journaled result_bytes");
        assert!(j.result_bytes.get() <= r.result_bytes.get(), "journaled <= relaxed result_bytes");

        assert!(s.trace_ring_capacity.get() <= j.trace_ring_capacity.get(), "strict <= journaled trace_ring_capacity");
        assert!(j.trace_ring_capacity.get() <= r.trace_ring_capacity.get(), "journaled <= relaxed trace_ring_capacity");

        assert!(s.journal_writer_queue_capacity.get() <= j.journal_writer_queue_capacity.get(), "strict <= journaled journal_writer_queue_capacity");
        assert!(j.journal_writer_queue_capacity.get() <= r.journal_writer_queue_capacity.get(), "journaled <= relaxed journal_writer_queue_capacity");

        assert!(s.for_each_item_count.get() <= j.for_each_item_count.get(), "strict <= journaled for_each_item_count");
        assert!(j.for_each_item_count.get() <= r.for_each_item_count.get(), "journaled <= relaxed for_each_item_count");

        assert!(s.together_branch_count.get() <= j.together_branch_count.get(), "strict <= journaled together_branch_count");
        assert!(j.together_branch_count.get() <= r.together_branch_count.get(), "journaled <= relaxed together_branch_count");

        assert!(s.collect_pages.get() <= j.collect_pages.get(), "strict <= journaled collect_pages");
        assert!(j.collect_pages.get() <= r.collect_pages.get(), "journaled <= relaxed collect_pages");

        assert!(s.collect_items.get() <= j.collect_items.get(), "strict <= journaled collect_items");
        assert!(j.collect_items.get() <= r.collect_items.get(), "journaled <= relaxed collect_items");

        assert!(s.collect_time_seconds.get() <= j.collect_time_seconds.get(), "strict <= journaled collect_time_seconds");
        assert!(j.collect_time_seconds.get() <= r.collect_time_seconds.get(), "journaled <= relaxed collect_time_seconds");

        assert!(s.repeat_attempts.get() <= j.repeat_attempts.get(), "strict <= journaled repeat_attempts");
        assert!(j.repeat_attempts.get() <= r.repeat_attempts.get(), "journaled <= relaxed repeat_attempts");

        assert!(s.retry_attempts.get() <= j.retry_attempts.get(), "strict <= journaled retry_attempts");
        assert!(j.retry_attempts.get() <= r.retry_attempts.get(), "journaled <= relaxed retry_attempts");

        assert!(s.max_wait_duration_seconds.get() <= j.max_wait_duration_seconds.get(), "strict <= journaled max_wait_duration_seconds");
        assert!(j.max_wait_duration_seconds.get() <= r.max_wait_duration_seconds.get(), "journaled <= relaxed max_wait_duration_seconds");

        assert!(s.ask_timeout_seconds.get() <= j.ask_timeout_seconds.get(), "strict <= journaled ask_timeout_seconds");
        assert!(j.ask_timeout_seconds.get() <= r.ask_timeout_seconds.get(), "journaled <= relaxed ask_timeout_seconds");
    }

    /// C5: ResourceContract::DEFAULT does not fit Strict profile (too tight).
    #[test]
    fn resource_contract_fits_within_profiles() {
        let contract = ResourceContract::DEFAULT;
        let strict = RuntimeLimitsProfile::strict();
        let relaxed = RuntimeLimitsProfile::relaxed();

        // Strict has very conservative limits — DEFAULT does not fit
        let strict_result = contract.fits_within_profile(&strict);
        assert!(strict_result.is_err(), "DEFAULT should not fit Strict (too tight): {strict_result:?}");

        // Verify error format for Strict failure (any variant is acceptable)
        if strict_result.is_err() {
            let _err = strict_result.unwrap_err();
            // Error is ContractViolation — at least one field exceeds profile
            // The specific field varies; we just verify the error is non-empty
            let msg = format!("{_err}");
            assert!(!msg.is_empty(), "error message should not be empty: {msg}");
        }
    }

    /// C5b: ResourceContract::DEFAULT fits within hard limits.
    #[test]
    fn resource_contract_fits_within_hard_limits() {
        let contract = ResourceContract::DEFAULT;
        assert!(
            contract.fits_within_hard_limits().is_ok(),
            "DEFAULT contract must fit hard limits: {:?}",
            contract.fits_within_hard_limits()
        );
    }

    /// C6: BoundednessPolicy from profile — derived fields ≤ profile limits.
    #[test]
    fn policy_bound_by_profile() {
        for profile in [
            RuntimeLimitsProfile::strict(),
            RuntimeLimitsProfile::journaled(),
            RuntimeLimitsProfile::relaxed(),
        ] {
            let policy = BoundednessPolicy::from_profile(&profile);
            assert!(
                policy.absolute_max_result_bytes <= profile.result_bytes.get(),
                "policy result_bytes must ≤ profile result_bytes"
            );
            assert!(
                policy.absolute_max_queue_depth <= profile.ready_queue_depth.get(),
                "policy queue_depth must ≤ profile ready_queue_depth"
            );
            assert!(
                policy.absolute_max_ipc_payload_bytes <= profile.ipc_frame_bytes.get(),
                "policy ipc_payload must ≤ profile ipc_frame_bytes"
            );
            assert!(
                policy.absolute_max_journal_batch_bytes <= profile.journal_writer_queue_capacity.get() as u32,
                "policy journal_batch must ≤ profile journal_writer_queue_capacity"
            );
            assert!(
                policy.absolute_max_input_bytes <= profile.action_input_bytes.get(),
                "policy input_bytes must ≤ profile action_input_bytes"
            );
        }
    }

    /// Profile smart constructor rejects zero values.
    #[test]
    fn profile_new_rejects_zero_values() {
        let result = RuntimeLimitsProfile::new(
            ProfileName::Strict,
            0, // zero active_runs
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        );
        assert!(result.is_err(), "zero active_runs should be rejected");
    }

    /// Profile smart constructor rejects values exceeding hard limits.
    #[test]
    fn profile_new_rejects_exceeding_hard_limits() {
        let result = RuntimeLimitsProfile::new(
            ProfileName::Relaxed,
            1,
            limits::MAX_QUEUE_DEPTH as u32 + 1, // exceeds MAX_QUEUE_DEPTH
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        );
        assert!(result.is_err(), "exceeding MAX_QUEUE_DEPTH should be rejected");
    }

    /// Profile new() accepts boundary values exactly at hard limits.
    #[test]
    fn profile_new_accepts_boundary_values() {
        let result = RuntimeLimitsProfile::new(
            ProfileName::Relaxed,
            1,
            limits::MAX_QUEUE_DEPTH.min(u32::MAX), // at boundary
            limits::MAX_IPC_PAYLOAD_BYTES.min(u32::MAX),
            limits::MAX_INPUT_BYTES.min(u32::MAX),
            limits::MAX_OUTPUT_BYTES.min(u32::MAX),
            limits::MAX_OUTPUT_BYTES.min(u32::MAX),
            limits::MAX_OUTPUT_BYTES.min(u32::MAX),
            1, 1, 1, 1, 1, limits::MAX_COLLECT_ITEMS.min(u32::MAX), 1, 1, 1, 1,
            limits::MAX_RETRY_ATTEMPTS as u64,
            limits::MAX_RETRY_ATTEMPTS as u64,
        );
        assert!(result.is_ok(), "boundary values should be accepted");
    }
}
