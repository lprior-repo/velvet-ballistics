#![forbid(unsafe_code)]
//! Runtime limits profile matrix — per-profile resource limits.
//!
//! Three canonical profiles (Strict, Journaled, Relaxed) each return
//! profile-specific boundedness policies, resource contracts, and shard
//! configurations. All values are non-zero, finite, and bounded by the
//! hard limits in `crate::limits`.

use std::num::{NonZeroU16, NonZeroU32, NonZeroU64, NonZeroUsize};

use crate::budget::BoundednessPolicy;
use crate::limits::*;
use crate::policy::ContractViolation;
use crate::policy::profile_name::ProfileName;
use crate::policy::profile_validation_error::ProfileValidationError;
use crate::workflow::types::ResourceContract;

/// Per-profile resource limits.
///
/// Every field is `NonZero*` to enforce positivity at the type level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct RuntimeLimitsProfile {
    pub name: ProfileName,
    pub active_runs: NonZeroUsize,
    pub ready_queue_depth: NonZeroU32,
    pub ipc_frame_bytes: NonZeroU32,
    pub action_input_bytes: NonZeroU32,
    pub action_output_bytes: NonZeroU32,
    pub step_output_bytes: NonZeroU32,
    pub result_bytes: NonZeroU32,
    pub trace_ring_capacity: NonZeroUsize,
    pub journal_writer_queue_capacity: NonZeroUsize,
    pub for_each_item_count: NonZeroU32,
    pub together_branch_count: NonZeroU16,
    pub collect_pages: NonZeroU32,
    pub collect_items: NonZeroU32,
    pub collect_time_seconds: NonZeroU64,
    pub repeat_attempts: NonZeroU16,
    pub repeat_time_seconds: NonZeroU64,
    pub retry_attempts: NonZeroU16,
    pub max_wait_duration_seconds: NonZeroU64,
    pub ask_timeout_seconds: NonZeroU64,
}

impl RuntimeLimitsProfile {
    /// Smart constructor — validates all fields against hard limits.
    pub fn new(
        name: ProfileName,
        active_runs: usize,
        ready_queue_depth: u32,
        ipc_frame_bytes: u32,
        action_input_bytes: u32,
        action_output_bytes: u32,
        step_output_bytes: u32,
        result_bytes: u32,
        trace_ring_capacity: usize,
        journal_writer_queue_capacity: usize,
        for_each_item_count: u32,
        together_branch_count: u16,
        collect_pages: u32,
        collect_items: u32,
        collect_time_seconds: u64,
        repeat_attempts: u16,
        repeat_time_seconds: u64,
        retry_attempts: u16,
        max_wait_duration_seconds: u64,
        ask_timeout_seconds: u64,
    ) -> Result<Self, ProfileValidationError> {
        if active_runs == 0 || active_runs > MAX_STEPS_PER_WORKFLOW {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "active_runs",
                value: active_runs as u64,
                limit: MAX_STEPS_PER_WORKFLOW as u64,
            });
        }
        if ready_queue_depth == 0 || ready_queue_depth > MAX_QUEUE_DEPTH {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "ready_queue_depth",
                value: ready_queue_depth as u64,
                limit: MAX_QUEUE_DEPTH as u64,
            });
        }
        if ipc_frame_bytes == 0 || ipc_frame_bytes > MAX_IPC_PAYLOAD_BYTES {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "ipc_frame_bytes",
                value: ipc_frame_bytes as u64,
                limit: MAX_IPC_PAYLOAD_BYTES as u64,
            });
        }
        if action_input_bytes == 0 || action_input_bytes > MAX_INPUT_BYTES {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "action_input_bytes",
                value: action_input_bytes as u64,
                limit: MAX_INPUT_BYTES as u64,
            });
        }
        if action_output_bytes == 0 || action_output_bytes > MAX_OUTPUT_BYTES {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "action_output_bytes",
                value: action_output_bytes as u64,
                limit: MAX_OUTPUT_BYTES as u64,
            });
        }
        if step_output_bytes == 0 || step_output_bytes > MAX_OUTPUT_BYTES {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "step_output_bytes",
                value: step_output_bytes as u64,
                limit: MAX_OUTPUT_BYTES as u64,
            });
        }
        if result_bytes == 0 || result_bytes > MAX_OUTPUT_BYTES {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "result_bytes",
                value: result_bytes as u64,
                limit: MAX_OUTPUT_BYTES as u64,
            });
        }
        if trace_ring_capacity == 0 {
            return Err(ProfileValidationError::ZeroValue {
                field: "trace_ring_capacity",
                value: 0,
            });
        }
        if journal_writer_queue_capacity == 0 {
            return Err(ProfileValidationError::ZeroValue {
                field: "journal_writer_queue_capacity",
                value: 0,
            });
        }
        if for_each_item_count == 0 {
            return Err(ProfileValidationError::ZeroValue {
                field: "for_each_item_count",
                value: 0,
            });
        }
        if together_branch_count == 0 || together_branch_count > MAX_FANOUT {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "together_branch_count",
                value: together_branch_count as u64,
                limit: MAX_FANOUT as u64,
            });
        }
        if collect_pages == 0 {
            return Err(ProfileValidationError::ZeroValue {
                field: "collect_pages",
                value: 0,
            });
        }
        if collect_items == 0 || collect_items > MAX_COLLECT_ITEMS {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "collect_items",
                value: collect_items as u64,
                limit: MAX_COLLECT_ITEMS as u64,
            });
        }
        if collect_time_seconds == 0 {
            return Err(ProfileValidationError::ZeroValue {
                field: "collect_time_seconds",
                value: 0,
            });
        }
        if repeat_attempts == 0 || repeat_attempts > MAX_RETRY_ATTEMPTS {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "repeat_attempts",
                value: repeat_attempts as u64,
                limit: MAX_RETRY_ATTEMPTS as u64,
            });
        }
        if repeat_time_seconds == 0 {
            return Err(ProfileValidationError::ZeroValue {
                field: "repeat_time_seconds",
                value: 0,
            });
        }
        if retry_attempts == 0 || retry_attempts > MAX_RETRY_ATTEMPTS {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "retry_attempts",
                value: retry_attempts as u64,
                limit: MAX_RETRY_ATTEMPTS as u64,
            });
        }
        if max_wait_duration_seconds == 0 {
            return Err(ProfileValidationError::ZeroValue {
                field: "max_wait_duration_seconds",
                value: 0,
            });
        }
        if ask_timeout_seconds == 0 {
            return Err(ProfileValidationError::ZeroValue {
                field: "ask_timeout_seconds",
                value: 0,
            });
        }

        Ok(Self {
            name,
            active_runs: NonZeroUsize::new(active_runs).unwrap(),
            ready_queue_depth: NonZeroU32::new(ready_queue_depth).unwrap(),
            ipc_frame_bytes: NonZeroU32::new(ipc_frame_bytes).unwrap(),
            action_input_bytes: NonZeroU32::new(action_input_bytes).unwrap(),
            action_output_bytes: NonZeroU32::new(action_output_bytes).unwrap(),
            step_output_bytes: NonZeroU32::new(step_output_bytes).unwrap(),
            result_bytes: NonZeroU32::new(result_bytes).unwrap(),
            trace_ring_capacity: NonZeroUsize::new(trace_ring_capacity).unwrap(),
            journal_writer_queue_capacity: NonZeroUsize::new(journal_writer_queue_capacity)
                .unwrap(),
            for_each_item_count: NonZeroU32::new(for_each_item_count).unwrap(),
            together_branch_count: NonZeroU16::new(together_branch_count).unwrap(),
            collect_pages: NonZeroU32::new(collect_pages).unwrap(),
            collect_items: NonZeroU32::new(collect_items).unwrap(),
            collect_time_seconds: NonZeroU64::new(collect_time_seconds).unwrap(),
            repeat_attempts: NonZeroU16::new(repeat_attempts).unwrap(),
            repeat_time_seconds: NonZeroU64::new(repeat_time_seconds).unwrap(),
            retry_attempts: NonZeroU16::new(retry_attempts).unwrap(),
            max_wait_duration_seconds: NonZeroU64::new(max_wait_duration_seconds).unwrap(),
            ask_timeout_seconds: NonZeroU64::new(ask_timeout_seconds).unwrap(),
        })
    }

    /// Canonical Strict profile — most restrictive limits.
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            name: ProfileName::Strict,
            active_runs: NonZeroUsize::new(16).unwrap(),
            ready_queue_depth: NonZeroU32::new(256).unwrap(),
            ipc_frame_bytes: NonZeroU32::new(4_096).unwrap(),
            action_input_bytes: NonZeroU32::new(4_096).unwrap(),
            action_output_bytes: NonZeroU32::new(4_096).unwrap(),
            step_output_bytes: NonZeroU32::new(4_096).unwrap(),
            result_bytes: NonZeroU32::new(65_536).unwrap(),
            trace_ring_capacity: NonZeroUsize::new(1_024).unwrap(),
            journal_writer_queue_capacity: NonZeroUsize::new(512).unwrap(),
            for_each_item_count: NonZeroU32::new(32).unwrap(),
            together_branch_count: NonZeroU16::new(4).unwrap(),
            collect_pages: NonZeroU32::new(4).unwrap(),
            collect_items: NonZeroU32::new(64).unwrap(),
            collect_time_seconds: NonZeroU64::new(1).unwrap(),
            repeat_attempts: NonZeroU16::new(2).unwrap(),
            repeat_time_seconds: NonZeroU64::new(1).unwrap(),
            retry_attempts: NonZeroU16::new(2).unwrap(),
            max_wait_duration_seconds: NonZeroU64::new(10).unwrap(),
            ask_timeout_seconds: NonZeroU64::new(30).unwrap(),
        }
    }

    /// Canonical Journaled profile — moderate limits.
    #[must_use]
    pub const fn journaled() -> Self {
        Self {
            name: ProfileName::Journaled,
            active_runs: NonZeroUsize::new(64).unwrap(),
            ready_queue_depth: NonZeroU32::new(1_024).unwrap(),
            ipc_frame_bytes: NonZeroU32::new(65_536).unwrap(),
            action_input_bytes: NonZeroU32::new(65_536).unwrap(),
            action_output_bytes: NonZeroU32::new(65_536).unwrap(),
            step_output_bytes: NonZeroU32::new(65_536).unwrap(),
            result_bytes: NonZeroU32::new(1_048_576).unwrap(),
            trace_ring_capacity: NonZeroUsize::new(4_096).unwrap(),
            journal_writer_queue_capacity: NonZeroUsize::new(2_048).unwrap(),
            for_each_item_count: NonZeroU32::new(256).unwrap(),
            together_branch_count: NonZeroU16::new(16).unwrap(),
            collect_pages: NonZeroU32::new(16).unwrap(),
            collect_items: NonZeroU32::new(4_096).unwrap(),
            collect_time_seconds: NonZeroU64::new(5).unwrap(),
            repeat_attempts: NonZeroU16::new(4).unwrap(),
            repeat_time_seconds: NonZeroU64::new(5).unwrap(),
            retry_attempts: NonZeroU16::new(4).unwrap(),
            max_wait_duration_seconds: NonZeroU64::new(60).unwrap(),
            ask_timeout_seconds: NonZeroU64::new(120).unwrap(),
        }
    }

    /// Canonical Relaxed profile — most permissive limits.
    #[must_use]
    pub const fn relaxed() -> Self {
        Self {
            name: ProfileName::Relaxed,
            active_runs: NonZeroUsize::new(256).unwrap(),
            ready_queue_depth: NonZeroU32::new(4_096).unwrap(),
            ipc_frame_bytes: NonZeroU32::new(262_144).unwrap(),
            action_input_bytes: NonZeroU32::new(262_144).unwrap(),
            action_output_bytes: NonZeroU32::new(262_144).unwrap(),
            step_output_bytes: NonZeroU32::new(262_144).unwrap(),
            result_bytes: NonZeroU32::new(16_777_216).unwrap(),
            trace_ring_capacity: NonZeroUsize::new(16_384).unwrap(),
            journal_writer_queue_capacity: NonZeroUsize::new(8_192).unwrap(),
            for_each_item_count: NonZeroU32::new(1_024).unwrap(),
            together_branch_count: NonZeroU16::new(64).unwrap(),
            collect_pages: NonZeroU32::new(64).unwrap(),
            collect_items: NonZeroU32::new(65_536).unwrap(),
            collect_time_seconds: NonZeroU64::new(30).unwrap(),
            repeat_attempts: NonZeroU16::new(8).unwrap(),
            repeat_time_seconds: NonZeroU64::new(30).unwrap(),
            retry_attempts: NonZeroU16::new(8).unwrap(),
            max_wait_duration_seconds: NonZeroU64::new(300).unwrap(),
            ask_timeout_seconds: NonZeroU64::new(600).unwrap(),
        }
    }

    /// Converts the profile to a `BoundednessPolicy`.
    #[must_use]
    pub fn to_policy(&self) -> BoundednessPolicy {
        BoundednessPolicy::from_profile(self)
    }

    /// Converts the profile to a `ResourceContract`.
    #[must_use]
    pub fn to_resource_contract(&self) -> ResourceContract {
        ResourceContract::from_profile(self)
    }

    /// Returns the canonical profile by name.
    #[must_use]
    pub const fn by_name(name: ProfileName) -> Self {
        match name {
            ProfileName::Strict => Self::strict(),
            ProfileName::Journaled => Self::journaled(),
            ProfileName::Relaxed => Self::relaxed(),
        }
    }
}

impl BoundednessPolicy {
    /// Creates a boundedness policy from a runtime limits profile.
    #[must_use]
    pub fn from_profile(profile: &RuntimeLimitsProfile) -> Self {
        Self {
            max_total_steps: MAX_STEPS_PER_WORKFLOW as u64,
            max_total_slots: MAX_SLOTS_PER_WORKFLOW as u64,
            max_fanout: profile.together_branch_count.get(),
            max_nesting_depth: MAX_LANGUAGE_NESTING_DEPTH as u16,
            absolute_max_action_tickets: 1024,
            absolute_max_parallel: profile.active_runs.get() as u16,
            absolute_max_run_time_seconds: profile.max_wait_duration_seconds.get()
                + profile.repeat_time_seconds.get(),
            absolute_max_result_bytes: profile.result_bytes.get(),
            absolute_max_steps_executable: profile.active_runs.get() as u32,
            absolute_max_timer_entries: 256,
            absolute_max_trace_events: profile.trace_ring_capacity.get() as u64,
            absolute_max_journal_batch_bytes: profile.journal_writer_queue_capacity.get() as u32,
            absolute_max_queue_depth: profile.ready_queue_depth.get(),
            absolute_max_ipc_payload_bytes: profile.ipc_frame_bytes.get(),
            absolute_max_blob_bytes: profile.journal_writer_queue_capacity.get() as u64,
            absolute_max_input_bytes: profile.action_input_bytes.get(),
        }
    }
}

impl ResourceContract {
    /// Creates a resource contract from a runtime limits profile.
    #[must_use]
    pub const fn from_profile(profile: &RuntimeLimitsProfile) -> Self {
        Self {
            max_steps: MAX_STEPS_PER_WORKFLOW as u16,
            max_slots: MAX_SLOTS_PER_WORKFLOW as u16,
            max_constants: MAX_CONSTANTS as u16,
            max_accessors: MAX_ACCESSORS as u16,
            max_expressions: MAX_EXPRESSIONS as u16,
            max_expr_stack: MAX_EXPRESSION_STACK,
            max_step_budget_per_tick: MAX_STEP_BUDGET,
            max_transitions_per_tick: MAX_STEP_BUDGET,
            max_input_bytes: profile.action_input_bytes.get(),
            max_output_bytes: profile.action_output_bytes.get(),
            max_blob_bytes: MAX_BLOB_BYTES,
            max_ipc_payload_bytes: profile.ipc_frame_bytes.get(),
            max_retry_attempts: profile.retry_attempts.get(),
            max_fanout: profile.together_branch_count.get(),
            max_collect_items: profile.collect_items.get(),
            max_queue_depth: profile.ready_queue_depth.get(),
            max_journal_batch_bytes: profile.journal_writer_queue_capacity.get() as u32,
            allows_secret_results: false,
        }
    }

    /// Checks whether this resource contract fits within the given profile.
    ///
    /// Returns `Ok(())` when every field is within the profile's corresponding
    /// limit, or the first `ContractViolation::ExceedsProfileLimit` when a
    /// field exceeds that limit.  Fields that have no direct profile mapping
    /// (steps, slots, constants, etc.) are validated against hard limits.
    #[must_use]
    pub fn fits_within_profile(
        &self,
        profile: &RuntimeLimitsProfile,
    ) -> Result<(), ContractViolation> {
        if self.max_input_bytes > profile.action_input_bytes.get() {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_input_bytes",
                actual: self.max_input_bytes as u64,
                profile_limit: profile.action_input_bytes.get() as u64,
            });
        }
        if self.max_output_bytes > profile.action_output_bytes.get() {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_output_bytes",
                actual: self.max_output_bytes as u64,
                profile_limit: profile.action_output_bytes.get() as u64,
            });
        }
        if self.max_ipc_payload_bytes > profile.ipc_frame_bytes.get() {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_ipc_payload_bytes",
                actual: self.max_ipc_payload_bytes as u64,
                profile_limit: profile.ipc_frame_bytes.get() as u64,
            });
        }
        if self.max_retry_attempts > profile.retry_attempts.get() {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_retry_attempts",
                actual: self.max_retry_attempts as u64,
                profile_limit: profile.retry_attempts.get() as u64,
            });
        }
        if self.max_fanout > profile.together_branch_count.get() {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_fanout",
                actual: self.max_fanout as u64,
                profile_limit: profile.together_branch_count.get() as u64,
            });
        }
        if self.max_collect_items > profile.collect_items.get() {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_collect_items",
                actual: self.max_collect_items as u64,
                profile_limit: profile.collect_items.get() as u64,
            });
        }
        if self.max_queue_depth > profile.ready_queue_depth.get() {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_queue_depth",
                actual: self.max_queue_depth as u64,
                profile_limit: profile.ready_queue_depth.get() as u64,
            });
        }
        if self.max_journal_batch_bytes > profile.journal_writer_queue_capacity.get() as u32 {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_journal_batch_bytes",
                actual: self.max_journal_batch_bytes as u64,
                profile_limit: profile.journal_writer_queue_capacity.get() as u64,
            });
        }
        // Fields without direct profile mapping → validated against hard limits.
        if self.max_steps as usize > MAX_STEPS_PER_WORKFLOW {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_steps",
                actual: self.max_steps as u64,
                hard_limit: MAX_STEPS_PER_WORKFLOW as u64,
            });
        }
        if self.max_slots as usize > MAX_SLOTS_PER_WORKFLOW {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_slots",
                actual: self.max_slots as u64,
                hard_limit: MAX_SLOTS_PER_WORKFLOW as u64,
            });
        }
        if self.max_constants as usize > MAX_CONSTANTS {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_constants",
                actual: self.max_constants as u64,
                hard_limit: MAX_CONSTANTS as u64,
            });
        }
        if self.max_accessors as usize > MAX_ACCESSORS {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_accessors",
                actual: self.max_accessors as u64,
                hard_limit: MAX_ACCESSORS as u64,
            });
        }
        if self.max_expressions as usize > MAX_EXPRESSIONS {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_expressions",
                actual: self.max_expressions as u64,
                hard_limit: MAX_EXPRESSIONS as u64,
            });
        }
        if self.max_expr_stack as usize > MAX_EXPRESSION_STACK as usize {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_expr_stack",
                actual: self.max_expr_stack as u64,
                hard_limit: MAX_EXPRESSION_STACK as u64,
            });
        }
        if self.max_step_budget_per_tick > MAX_STEP_BUDGET {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_step_budget_per_tick",
                actual: self.max_step_budget_per_tick,
                hard_limit: MAX_STEP_BUDGET,
            });
        }
        if self.max_transitions_per_tick > MAX_STEP_BUDGET {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_transitions_per_tick",
                actual: self.max_transitions_per_tick,
                hard_limit: MAX_STEP_BUDGET,
            });
        }
        if self.max_blob_bytes > MAX_BLOB_BYTES {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_blob_bytes",
                actual: self.max_blob_bytes,
                hard_limit: MAX_BLOB_BYTES,
            });
        }
        Ok(())
    }

    /// Checks whether all fields are within hard limits.
    ///
    /// Returns `Ok(())` when every field stays within its hard limit, or the
    /// first `ContractViolation::ExceedsHardLimit` for the field that exceeds
    /// its limit.  Adds checks for `max_transitions_per_tick` and
    /// `max_expr_stack` which were previously unvalidated.
    #[must_use]
    pub fn fits_within_hard_limits(&self) -> Result<(), ContractViolation> {
        if self.max_steps as usize > MAX_STEPS_PER_WORKFLOW {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_steps",
                actual: self.max_steps as u64,
                hard_limit: MAX_STEPS_PER_WORKFLOW as u64,
            });
        }
        if self.max_slots as usize > MAX_SLOTS_PER_WORKFLOW {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_slots",
                actual: self.max_slots as u64,
                hard_limit: MAX_SLOTS_PER_WORKFLOW as u64,
            });
        }
        if self.max_constants as usize > MAX_CONSTANTS {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_constants",
                actual: self.max_constants as u64,
                hard_limit: MAX_CONSTANTS as u64,
            });
        }
        if self.max_accessors as usize > MAX_ACCESSORS {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_accessors",
                actual: self.max_accessors as u64,
                hard_limit: MAX_ACCESSORS as u64,
            });
        }
        if self.max_expressions as usize > MAX_EXPRESSIONS {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_expressions",
                actual: self.max_expressions as u64,
                hard_limit: MAX_EXPRESSIONS as u64,
            });
        }
        if self.max_expr_stack as usize > MAX_EXPRESSION_STACK as usize {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_expr_stack",
                actual: self.max_expr_stack as u64,
                hard_limit: MAX_EXPRESSION_STACK as u64,
            });
        }
        if self.max_step_budget_per_tick > MAX_STEP_BUDGET {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_step_budget_per_tick",
                actual: self.max_step_budget_per_tick,
                hard_limit: MAX_STEP_BUDGET,
            });
        }
        if self.max_transitions_per_tick > MAX_STEP_BUDGET {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_transitions_per_tick",
                actual: self.max_transitions_per_tick,
                hard_limit: MAX_STEP_BUDGET,
            });
        }
        if self.max_input_bytes > MAX_INPUT_BYTES {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_input_bytes",
                actual: self.max_input_bytes as u64,
                hard_limit: MAX_INPUT_BYTES as u64,
            });
        }
        if self.max_output_bytes > MAX_OUTPUT_BYTES {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_output_bytes",
                actual: self.max_output_bytes as u64,
                hard_limit: MAX_OUTPUT_BYTES as u64,
            });
        }
        if self.max_blob_bytes > MAX_BLOB_BYTES {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_blob_bytes",
                actual: self.max_blob_bytes,
                hard_limit: MAX_BLOB_BYTES,
            });
        }
        if self.max_ipc_payload_bytes > MAX_IPC_PAYLOAD_BYTES {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_ipc_payload_bytes",
                actual: self.max_ipc_payload_bytes as u64,
                hard_limit: MAX_IPC_PAYLOAD_BYTES as u64,
            });
        }
        if self.max_retry_attempts > MAX_RETRY_ATTEMPTS {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_retry_attempts",
                actual: self.max_retry_attempts as u64,
                hard_limit: MAX_RETRY_ATTEMPTS as u64,
            });
        }
        if self.max_fanout > MAX_FANOUT {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_fanout",
                actual: self.max_fanout as u64,
                hard_limit: MAX_FANOUT as u64,
            });
        }
        if self.max_collect_items > MAX_COLLECT_ITEMS {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_collect_items",
                actual: self.max_collect_items as u64,
                hard_limit: MAX_COLLECT_ITEMS as u64,
            });
        }
        if self.max_queue_depth > MAX_QUEUE_DEPTH {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_queue_depth",
                actual: self.max_queue_depth as u64,
                hard_limit: MAX_QUEUE_DEPTH as u64,
            });
        }
        if self.max_journal_batch_bytes > MAX_JOURNAL_BATCH_BYTES {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_journal_batch_bytes",
                actual: self.max_journal_batch_bytes as u64,
                hard_limit: MAX_JOURNAL_BATCH_BYTES as u64,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod profile_tests {
    use super::*;

    #[test]
    fn test_strict_profile_exists() {
        let p = RuntimeLimitsProfile::strict();
        assert_eq!(p.name, ProfileName::Strict);
        assert!(p.active_runs.get() >= 1);
        assert!(p.ready_queue_depth.get() >= 1);
    }

    #[test]
    fn test_journaled_profile_exists() {
        let p = RuntimeLimitsProfile::journaled();
        assert_eq!(p.name, ProfileName::Journaled);
    }

    #[test]
    fn test_relaxed_profile_exists() {
        let p = RuntimeLimitsProfile::relaxed();
        assert_eq!(p.name, ProfileName::Relaxed);
    }

    #[test]
    fn test_strict_all_fields_nonzero() {
        let p = RuntimeLimitsProfile::strict();
        assert!(p.active_runs.get() > 0);
        assert!(p.ready_queue_depth.get() > 0);
        assert!(p.ipc_frame_bytes.get() > 0);
        assert!(p.action_input_bytes.get() > 0);
        assert!(p.action_output_bytes.get() > 0);
        assert!(p.step_output_bytes.get() > 0);
        assert!(p.result_bytes.get() > 0);
        assert!(p.trace_ring_capacity.get() > 0);
        assert!(p.journal_writer_queue_capacity.get() > 0);
        assert!(p.for_each_item_count.get() > 0);
        assert!(p.together_branch_count.get() > 0);
        assert!(p.collect_pages.get() > 0);
        assert!(p.collect_items.get() > 0);
        assert!(p.collect_time_seconds.get() > 0);
        assert!(p.repeat_attempts.get() > 0);
        assert!(p.repeat_time_seconds.get() > 0);
        assert!(p.retry_attempts.get() > 0);
        assert!(p.max_wait_duration_seconds.get() > 0);
        assert!(p.ask_timeout_seconds.get() > 0);
    }

    #[test]
    fn test_strict_within_hard_limits() {
        let p = RuntimeLimitsProfile::strict();
        assert!(p.active_runs.get() <= MAX_STEPS_PER_WORKFLOW);
        assert!(p.ready_queue_depth.get() <= MAX_QUEUE_DEPTH);
        assert!(p.ipc_frame_bytes.get() <= MAX_IPC_PAYLOAD_BYTES);
        assert!(p.action_input_bytes.get() <= MAX_INPUT_BYTES);
        assert!(p.action_output_bytes.get() <= MAX_OUTPUT_BYTES);
        assert!(p.result_bytes.get() <= MAX_OUTPUT_BYTES);
        assert!(p.together_branch_count.get() <= MAX_FANOUT);
        assert!(p.collect_items.get() <= MAX_COLLECT_ITEMS);
        assert!(p.retry_attempts.get() <= MAX_RETRY_ATTEMPTS);
        assert!(p.repeat_attempts.get() <= MAX_RETRY_ATTEMPTS);
    }

    #[test]
    fn test_journaled_within_hard_limits() {
        let p = RuntimeLimitsProfile::journaled();
        assert!(p.active_runs.get() <= MAX_STEPS_PER_WORKFLOW);
        assert!(p.ready_queue_depth.get() <= MAX_QUEUE_DEPTH);
        assert!(p.ipc_frame_bytes.get() <= MAX_IPC_PAYLOAD_BYTES);
        assert!(p.action_input_bytes.get() <= MAX_INPUT_BYTES);
        assert!(p.action_output_bytes.get() <= MAX_OUTPUT_BYTES);
        assert!(p.retry_attempts.get() <= MAX_RETRY_ATTEMPTS);
        assert!(p.collect_items.get() <= MAX_COLLECT_ITEMS);
    }

    #[test]
    fn test_relaxed_within_hard_limits() {
        let p = RuntimeLimitsProfile::relaxed();
        assert!(p.active_runs.get() <= MAX_STEPS_PER_WORKFLOW);
        assert!(p.ready_queue_depth.get() <= MAX_QUEUE_DEPTH);
        assert!(p.ipc_frame_bytes.get() <= MAX_IPC_PAYLOAD_BYTES);
        assert!(p.action_input_bytes.get() <= MAX_INPUT_BYTES);
        assert!(p.action_output_bytes.get() <= MAX_OUTPUT_BYTES);
        assert!(p.result_bytes.get() <= MAX_OUTPUT_BYTES);
        assert!(p.retry_attempts.get() <= MAX_RETRY_ATTEMPTS);
        assert!(p.collect_items.get() <= MAX_COLLECT_ITEMS);
    }

    #[test]
    fn test_to_policy_returns_valid_boundedness_policy() {
        let p = RuntimeLimitsProfile::strict();
        let policy = p.to_policy();
        assert!(policy.max_total_steps > 0);
        assert!(policy.max_total_slots > 0);
        assert!(policy.max_fanout > 0);
    }

    #[test]
    fn test_to_resource_contract_returns_valid_contract() {
        let p = RuntimeLimitsProfile::strict();
        let rc = p.to_resource_contract();
        assert!(rc.max_steps > 0);
        assert!(rc.max_slots > 0);
        assert!(rc.max_input_bytes > 0);
    }

    #[test]
    fn test_resource_contract_fits_within_hard_limits() {
        let p = RuntimeLimitsProfile::strict();
        let rc = p.to_resource_contract();
        assert!(
            rc.fits_within_hard_limits().is_ok(),
            "strict profile contract must fit hard limits: {:?}",
            rc.fits_within_hard_limits()
        );
    }

    #[test]
    fn test_resource_contract_fits_within_profile() {
        let p = RuntimeLimitsProfile::strict();
        let rc = p.to_resource_contract();
        assert!(
            rc.fits_within_profile(&p).is_ok(),
            "contract derived from profile must fit that profile: {:?}",
            rc.fits_within_profile(&p)
        );
    }

    #[test]
    fn test_new_validates_zero_active_runs() {
        let result = RuntimeLimitsProfile::new(
            ProfileName::Strict,
            0,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_new_validates_zero_retry_attempts() {
        let result = RuntimeLimitsProfile::new(
            ProfileName::Strict,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            0,
            1,
            1,
        );
        assert!(result.is_err());
    }
}
