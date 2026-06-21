#![forbid(unsafe_code)]
//! Domain contract for runtime admission policy and limits profiles.
//!
//! - [`RuntimeLimitsProfile`] — per-profile resource limits (Strict /
//!   Journaled / Relaxed).
//! - [`RuntimeLimitsConfig`] — raw configuration for smart construction.
//!
//! All profile values are non-zero, finite, and bounded by the hard limits
//! in `crate::limits`.

use std::num::{NonZeroU16, NonZeroU32, NonZeroU64, NonZeroUsize};

use crate::budget::BoundednessPolicy;
use crate::limits::*;
use crate::policy::ContractViolation;
use crate::policy::profile_name::ProfileName;
use crate::policy::profile_validation_error::ProfileValidationError;
use crate::workflow::resource_contract::ResourceContract;

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

/// Raw configuration for [`RuntimeLimitsProfile::new`].
///
/// Holds primitive (non-`NonZero*`) values to give call sites a stable,
/// panic-free construction path. The smart constructor validates every
/// field and converts it to the corresponding `NonZero*` representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuntimeLimitsConfig {
    pub active_runs: usize,
    pub ready_queue_depth: u32,
    pub ipc_frame_bytes: u32,
    pub action_input_bytes: u32,
    pub action_output_bytes: u32,
    pub step_output_bytes: u32,
    pub result_bytes: u32,
    pub trace_ring_capacity: usize,
    pub journal_writer_queue_capacity: usize,
    pub for_each_item_count: u32,
    pub together_branch_count: u16,
    pub collect_pages: u32,
    pub collect_items: u32,
    pub collect_time_seconds: u64,
    pub repeat_attempts: u16,
    pub repeat_time_seconds: u64,
    pub retry_attempts: u16,
    pub max_wait_duration_seconds: u64,
    pub ask_timeout_seconds: u64,
}

/// Convert a `usize` to `u64`, clamping the impossible overflow case to
/// `u64::MAX` for diagnostic use. `usize` is at most `u64::MAX` on every
/// supported target, so the `None` branch is never taken in practice.
#[inline]
fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// Convert a `usize` to `u16`, clamping to `u16::MAX` on overflow.
///
/// The smart constructor enforces `active_runs <= MAX_STEPS_PER_WORKFLOW
/// = 1_000` (which fits in `u16`) and the constant `MAX_*` values used in
/// `ResourceContract::from_profile` are all ≤ 65_535, so the clamp is
/// unreachable in practice but provides a safe fallback.
#[inline]
fn usize_to_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

/// Convert a `usize` to `u32`, clamping to `u32::MAX` on overflow.
///
/// The smart constructor enforces `active_runs <= MAX_STEPS_PER_WORKFLOW
/// = 1_000` and `journal_writer_queue_capacity <= u32::MAX`, so the clamp
/// is unreachable in practice for validated profiles.
#[inline]
fn usize_to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// Convert a `usize` to `u32`, returning `false` when the value would
/// truncate. Used to enforce `journal_writer_queue_capacity <= u32::MAX`.
#[inline]
fn usize_fits_in_u32(value: usize) -> bool {
    u32::try_from(value).is_ok()
}

/// Convert a raw `usize` to `NonZeroUsize`, returning a typed error on zero.
#[inline]
fn nz_usize(value: usize, field: &'static str) -> Result<NonZeroUsize, ProfileValidationError> {
    match NonZeroUsize::new(value) {
        Some(nz) => Ok(nz),
        None => Err(ProfileValidationError::ZeroValue { field, value: 0 }),
    }
}

/// Compile-time `1` constant for [`NonZero*`] types. The impossible
/// `None` arm of `NonZero*::new(1)` uses `loop {}` (the divergent
/// function type `!`) to satisfy the no-`unwrap`/no-`expect`/no-`panic`
/// Holzman-Rust rules.
trait NonZeroOne {
    const ONE: Self;
}

impl NonZeroOne for NonZeroUsize {
    const ONE: Self = match NonZeroUsize::new(1) {
        Some(v) => v,
        None => loop {},
    };
}
impl NonZeroOne for NonZeroU32 {
    const ONE: Self = match NonZeroU32::new(1) {
        Some(v) => v,
        None => loop {},
    };
}
impl NonZeroOne for NonZeroU64 {
    const ONE: Self = match NonZeroU64::new(1) {
        Some(v) => v,
        None => loop {},
    };
}
impl NonZeroOne for NonZeroU16 {
    const ONE: Self = match NonZeroU16::new(1) {
        Some(v) => v,
        None => loop {},
    };
}

/// Convert a raw `u32` to `NonZeroU32`, returning a typed error on zero.
#[inline]
fn nz_u32(value: u32, field: &'static str) -> Result<NonZeroU32, ProfileValidationError> {
    match NonZeroU32::new(value) {
        Some(nz) => Ok(nz),
        None => Err(ProfileValidationError::ZeroValue { field, value: 0 }),
    }
}

/// Convert a raw `u64` to `NonZeroU64`, returning a typed error on zero.
#[inline]
fn nz_u64(value: u64, field: &'static str) -> Result<NonZeroU64, ProfileValidationError> {
    match NonZeroU64::new(value) {
        Some(nz) => Ok(nz),
        None => Err(ProfileValidationError::ZeroValue { field, value: 0 }),
    }
}

/// Convert a raw `u16` to `NonZeroU16`, returning a typed error on zero.
#[inline]
fn nz_u16(value: u16, field: &'static str) -> Result<NonZeroU16, ProfileValidationError> {
    match NonZeroU16::new(value) {
        Some(nz) => Ok(nz),
        None => Err(ProfileValidationError::ZeroValue { field, value: 0 }),
    }
}

impl RuntimeLimitsProfile {
    /// Smart constructor — validates all fields against hard limits.
    ///
    /// Returns a [`ProfileValidationError`] when any field is zero or
    /// exceeds its corresponding hard limit.
    pub fn new(
        name: ProfileName,
        config: RuntimeLimitsConfig,
    ) -> Result<Self, ProfileValidationError> {
        if config.active_runs == 0 || config.active_runs > MAX_STEPS_PER_WORKFLOW {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "active_runs",
                value: usize_to_u64(config.active_runs),
                limit: usize_to_u64(MAX_STEPS_PER_WORKFLOW),
            });
        }
        if config.ready_queue_depth == 0 || config.ready_queue_depth > MAX_QUEUE_DEPTH {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "ready_queue_depth",
                value: u64::from(config.ready_queue_depth),
                limit: u64::from(MAX_QUEUE_DEPTH),
            });
        }
        if config.ipc_frame_bytes == 0 || config.ipc_frame_bytes > MAX_IPC_PAYLOAD_BYTES {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "ipc_frame_bytes",
                value: u64::from(config.ipc_frame_bytes),
                limit: u64::from(MAX_IPC_PAYLOAD_BYTES),
            });
        }
        if config.action_input_bytes == 0 || config.action_input_bytes > MAX_INPUT_BYTES {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "action_input_bytes",
                value: u64::from(config.action_input_bytes),
                limit: u64::from(MAX_INPUT_BYTES),
            });
        }
        if config.action_output_bytes == 0 || config.action_output_bytes > MAX_OUTPUT_BYTES {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "action_output_bytes",
                value: u64::from(config.action_output_bytes),
                limit: u64::from(MAX_OUTPUT_BYTES),
            });
        }
        if config.step_output_bytes == 0 || config.step_output_bytes > MAX_OUTPUT_BYTES {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "step_output_bytes",
                value: u64::from(config.step_output_bytes),
                limit: u64::from(MAX_OUTPUT_BYTES),
            });
        }
        if config.result_bytes == 0 || config.result_bytes > MAX_OUTPUT_BYTES {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "result_bytes",
                value: u64::from(config.result_bytes),
                limit: u64::from(MAX_OUTPUT_BYTES),
            });
        }
        if config.journal_writer_queue_capacity == 0
            || !usize_fits_in_u32(config.journal_writer_queue_capacity)
        {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "journal_writer_queue_capacity",
                value: usize_to_u64(config.journal_writer_queue_capacity),
                limit: u64::from(u32::MAX),
            });
        }
        if config.together_branch_count == 0 || config.together_branch_count > MAX_FANOUT {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "together_branch_count",
                value: u64::from(config.together_branch_count),
                limit: u64::from(MAX_FANOUT),
            });
        }
        if config.collect_items == 0 || config.collect_items > MAX_COLLECT_ITEMS {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "collect_items",
                value: u64::from(config.collect_items),
                limit: u64::from(MAX_COLLECT_ITEMS),
            });
        }
        if config.repeat_attempts == 0 || config.repeat_attempts > MAX_RETRY_ATTEMPTS {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "repeat_attempts",
                value: u64::from(config.repeat_attempts),
                limit: u64::from(MAX_RETRY_ATTEMPTS),
            });
        }
        if config.retry_attempts == 0 || config.retry_attempts > MAX_RETRY_ATTEMPTS {
            return Err(ProfileValidationError::ExceedsHardLimit {
                field: "retry_attempts",
                value: u64::from(config.retry_attempts),
                limit: u64::from(MAX_RETRY_ATTEMPTS),
            });
        }

        Ok(Self {
            name,
            active_runs: nz_usize(config.active_runs, "active_runs")?,
            ready_queue_depth: nz_u32(config.ready_queue_depth, "ready_queue_depth")?,
            ipc_frame_bytes: nz_u32(config.ipc_frame_bytes, "ipc_frame_bytes")?,
            action_input_bytes: nz_u32(config.action_input_bytes, "action_input_bytes")?,
            action_output_bytes: nz_u32(config.action_output_bytes, "action_output_bytes")?,
            step_output_bytes: nz_u32(config.step_output_bytes, "step_output_bytes")?,
            result_bytes: nz_u32(config.result_bytes, "result_bytes")?,
            trace_ring_capacity: nz_usize(config.trace_ring_capacity, "trace_ring_capacity")?,
            journal_writer_queue_capacity: nz_usize(
                config.journal_writer_queue_capacity,
                "journal_writer_queue_capacity",
            )?,
            for_each_item_count: nz_u32(config.for_each_item_count, "for_each_item_count")?,
            together_branch_count: nz_u16(config.together_branch_count, "together_branch_count")?,
            collect_pages: nz_u32(config.collect_pages, "collect_pages")?,
            collect_items: nz_u32(config.collect_items, "collect_items")?,
            collect_time_seconds: nz_u64(config.collect_time_seconds, "collect_time_seconds")?,
            repeat_attempts: nz_u16(config.repeat_attempts, "repeat_attempts")?,
            repeat_time_seconds: nz_u64(config.repeat_time_seconds, "repeat_time_seconds")?,
            retry_attempts: nz_u16(config.retry_attempts, "retry_attempts")?,
            max_wait_duration_seconds: nz_u64(
                config.max_wait_duration_seconds,
                "max_wait_duration_seconds",
            )?,
            ask_timeout_seconds: nz_u64(config.ask_timeout_seconds, "ask_timeout_seconds")?,
        })
    }

    /// Internal constructor for pre-validated configurations.
    ///
    /// The caller is responsible for ensuring every field in `config` is
    /// positive and within the corresponding hard limit.  This is used
    /// by the canonical profile factories, whose values are validated by
    /// hand and cannot fail validation.
    fn from_validated_config(name: ProfileName, config: RuntimeLimitsConfig) -> Self {
        // The `nz_*` helpers below return `Result<_, ProfileValidationError>`.
        // The canonical profile factories only pass positive literals,
        // so the `Err` branch is genuinely unreachable.  We use `loop {}`
        // (a divergent function) as the fallback for the impossible `None`
        // case of `NonZero*::new(1)`.  This is unwrap-free, expect-free,
        // panic-free, and process::exit-free.
        let nz_or_one = |result: Result<T, ProfileValidationError>| -> T
        where
            T: NonZeroOne,
        {
            match result {
                Ok(v) => v,
                Err(_) => T::ONE,
            }
        };
        let nz_usize_val = |result| nz_or_one::<NonZeroUsize>(result);
        let nz_u32_val = |result| nz_or_one::<NonZeroU32>(result);
        let nz_u64_val = |result| nz_or_one::<NonZeroU64>(result);
        let nz_u16_val = |result| nz_or_one::<NonZeroU16>(result);
        Self {
            name,
            active_runs: nz_usize_val(nz_usize(config.active_runs, "active_runs")),
            ready_queue_depth: nz_u32_val(nz_u32(config.ready_queue_depth, "ready_queue_depth")),
            ipc_frame_bytes: nz_u32_val(nz_u32(config.ipc_frame_bytes, "ipc_frame_bytes")),
            action_input_bytes: nz_u32_val(nz_u32(config.action_input_bytes, "action_input_bytes")),
            action_output_bytes: nz_u32_val(nz_u32(
                config.action_output_bytes,
                "action_output_bytes",
            )),
            step_output_bytes: nz_u32_val(nz_u32(config.step_output_bytes, "step_output_bytes")),
            result_bytes: nz_u32_val(nz_u32(config.result_bytes, "result_bytes")),
            trace_ring_capacity: nz_usize_val(nz_usize(
                config.trace_ring_capacity,
                "trace_ring_capacity",
            )),
            journal_writer_queue_capacity: nz_usize_val(nz_usize(
                config.journal_writer_queue_capacity,
                "journal_writer_queue_capacity",
            )),
            for_each_item_count: nz_u32_val(nz_u32(
                config.for_each_item_count,
                "for_each_item_count",
            )),
            together_branch_count: nz_u16_val(nz_u16(
                config.together_branch_count,
                "together_branch_count",
            )),
            collect_pages: nz_u32_val(nz_u32(config.collect_pages, "collect_pages")),
            collect_items: nz_u32_val(nz_u32(config.collect_items, "collect_items")),
            collect_time_seconds: nz_u64_val(nz_u64(
                config.collect_time_seconds,
                "collect_time_seconds",
            )),
            repeat_attempts: nz_u16_val(nz_u16(config.repeat_attempts, "repeat_attempts")),
            repeat_time_seconds: nz_u64_val(nz_u64(
                config.repeat_time_seconds,
                "repeat_time_seconds",
            )),
            retry_attempts: nz_u16_val(nz_u16(config.retry_attempts, "retry_attempts")),
            max_wait_duration_seconds: nz_u64_val(nz_u64(
                config.max_wait_duration_seconds,
                "max_wait_duration_seconds",
            )),
            ask_timeout_seconds: nz_u64_val(nz_u64(
                config.ask_timeout_seconds,
                "ask_timeout_seconds",
            )),
        }
    }

    /// Canonical Strict profile — most restrictive limits.
    #[must_use]
    pub fn strict() -> Self {
        let config = RuntimeLimitsConfig {
            active_runs: 16,
            ready_queue_depth: 256,
            ipc_frame_bytes: 4_096,
            action_input_bytes: 4_096,
            action_output_bytes: 4_096,
            step_output_bytes: 4_096,
            result_bytes: 65_536,
            trace_ring_capacity: 1_024,
            journal_writer_queue_capacity: 512,
            for_each_item_count: 32,
            together_branch_count: 4,
            collect_pages: 4,
            collect_items: 64,
            collect_time_seconds: 1,
            repeat_attempts: 2,
            repeat_time_seconds: 1,
            retry_attempts: 2,
            max_wait_duration_seconds: 10,
            ask_timeout_seconds: 30,
        };
        // Values are hand-validated positive literals that are well within
        // every hard limit, so we can construct infallibly.
        Self::from_validated_config(ProfileName::Strict, config)
    }

    /// Canonical Journaled profile — moderate limits.
    #[must_use]
    pub fn journaled() -> Self {
        let config = RuntimeLimitsConfig {
            active_runs: 64,
            ready_queue_depth: 1_024,
            ipc_frame_bytes: 65_536,
            action_input_bytes: 65_536,
            action_output_bytes: 65_536,
            step_output_bytes: 65_536,
            result_bytes: 1_048_576,
            trace_ring_capacity: 4_096,
            journal_writer_queue_capacity: 2_048,
            for_each_item_count: 256,
            together_branch_count: 16,
            collect_pages: 16,
            collect_items: 4_096,
            collect_time_seconds: 5,
            repeat_attempts: 4,
            repeat_time_seconds: 5,
            retry_attempts: 4,
            max_wait_duration_seconds: 60,
            ask_timeout_seconds: 120,
        };
        Self::from_validated_config(ProfileName::Journaled, config)
    }

    /// Canonical Relaxed profile — most permissive limits.
    #[must_use]
    pub fn relaxed() -> Self {
        let config = RuntimeLimitsConfig {
            active_runs: 256,
            ready_queue_depth: 4_096,
            ipc_frame_bytes: 262_144,
            action_input_bytes: 262_144,
            action_output_bytes: 262_144,
            step_output_bytes: 262_144,
            result_bytes: 16_777_216,
            trace_ring_capacity: 16_384,
            journal_writer_queue_capacity: 8_192,
            for_each_item_count: 1_024,
            together_branch_count: 64,
            collect_pages: 64,
            collect_items: 65_536,
            collect_time_seconds: 30,
            repeat_attempts: 8,
            repeat_time_seconds: 30,
            retry_attempts: 8,
            max_wait_duration_seconds: 300,
            ask_timeout_seconds: 600,
        };
        Self::from_validated_config(ProfileName::Relaxed, config)
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
    pub fn by_name(name: ProfileName) -> Self {
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
        // `active_runs` is bounded by `MAX_STEPS_PER_WORKFLOW = 1_000` in
        // the smart constructor, which fits in both `u16` and `u32`.
        let active_runs_u16 = usize_to_u16(profile.active_runs.get());
        let active_runs_u32 = usize_to_u32(profile.active_runs.get());
        // `journal_writer_queue_capacity` is bounded by `u32::MAX` in the
        // smart constructor.
        let journal_batch_u32 = usize_to_u32(profile.journal_writer_queue_capacity.get());
        // `max_wait_duration_seconds + repeat_time_seconds` are both
        // bounded; saturate on overflow so the policy never wraps.
        let run_time_seconds = profile
            .max_wait_duration_seconds
            .get()
            .saturating_add(profile.repeat_time_seconds.get());

        Self {
            max_total_steps: usize_to_u64(MAX_STEPS_PER_WORKFLOW),
            max_total_slots: usize_to_u64(MAX_SLOTS_PER_WORKFLOW),
            max_fanout: profile.together_branch_count.get(),
            max_nesting_depth: u16::from(MAX_LANGUAGE_NESTING_DEPTH),
            absolute_max_action_tickets: 1024,
            absolute_max_parallel: active_runs_u16,
            absolute_max_run_time_seconds: run_time_seconds,
            absolute_max_result_bytes: profile.result_bytes.get(),
            absolute_max_steps_executable: active_runs_u32,
            absolute_max_timer_entries: 256,
            absolute_max_trace_events: usize_to_u64(profile.trace_ring_capacity.get()),
            absolute_max_journal_batch_bytes: journal_batch_u32,
            absolute_max_queue_depth: profile.ready_queue_depth.get(),
            absolute_max_ipc_payload_bytes: profile.ipc_frame_bytes.get(),
            absolute_max_blob_bytes: usize_to_u64(profile.journal_writer_queue_capacity.get()),
            absolute_max_input_bytes: profile.action_input_bytes.get(),
        }
    }
}

impl ResourceContract {
    /// Creates a resource contract from a runtime limits profile.
    ///
    /// Not `const` because the `journal_writer_queue_capacity` truncation
    /// check uses `u32::try_from` (not stable in const context).
    #[must_use]
    pub fn from_profile(profile: &RuntimeLimitsProfile) -> Self {
        // All hard-limit constants used here are compile-time small
        // (≤ 65_535), so they fit in `u16`. `usize_to_u16` clamps the
        // impossible overflow case to `u16::MAX` for safety.
        let max_steps = usize_to_u16(MAX_STEPS_PER_WORKFLOW);
        let max_slots = usize_to_u16(MAX_SLOTS_PER_WORKFLOW);
        let max_constants = usize_to_u16(MAX_CONSTANTS);
        let max_accessors = usize_to_u16(MAX_ACCESSORS);
        let max_expressions = usize_to_u16(MAX_EXPRESSIONS);
        // `journal_writer_queue_capacity` is bounded by `u32::MAX` in the
        // smart constructor, so the conversion is lossless in practice.
        let max_journal_batch_bytes = usize_to_u32(profile.journal_writer_queue_capacity.get());

        Self {
            max_steps,
            max_slots,
            max_constants,
            max_accessors,
            max_expressions,
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
            max_journal_batch_bytes,
            allows_secret_results: false,
        }
    }

    /// Checks whether this resource contract fits within the given profile.
    ///
    /// Returns `Ok(())` when every field is within the profile's corresponding
    /// limit, or the first `ContractViolation::ExceedsProfileLimit` when a
    /// field exceeds that limit.  Fields that have no direct profile mapping
    /// (steps, slots, constants, etc.) are validated against hard limits.
    pub fn fits_within_profile(
        &self,
        profile: &RuntimeLimitsProfile,
    ) -> Result<(), ContractViolation> {
        if self.max_input_bytes > profile.action_input_bytes.get() {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_input_bytes",
                actual: u64::from(self.max_input_bytes),
                profile_limit: u64::from(profile.action_input_bytes.get()),
            });
        }
        if self.max_output_bytes > profile.action_output_bytes.get() {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_output_bytes",
                actual: u64::from(self.max_output_bytes),
                profile_limit: u64::from(profile.action_output_bytes.get()),
            });
        }
        if self.max_ipc_payload_bytes > profile.ipc_frame_bytes.get() {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_ipc_payload_bytes",
                actual: u64::from(self.max_ipc_payload_bytes),
                profile_limit: u64::from(profile.ipc_frame_bytes.get()),
            });
        }
        if self.max_retry_attempts > profile.retry_attempts.get() {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_retry_attempts",
                actual: u64::from(self.max_retry_attempts),
                profile_limit: u64::from(profile.retry_attempts.get()),
            });
        }
        if self.max_fanout > profile.together_branch_count.get() {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_fanout",
                actual: u64::from(self.max_fanout),
                profile_limit: u64::from(profile.together_branch_count.get()),
            });
        }
        if self.max_collect_items > profile.collect_items.get() {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_collect_items",
                actual: u64::from(self.max_collect_items),
                profile_limit: u64::from(profile.collect_items.get()),
            });
        }
        if self.max_queue_depth > profile.ready_queue_depth.get() {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_queue_depth",
                actual: u64::from(self.max_queue_depth),
                profile_limit: u64::from(profile.ready_queue_depth.get()),
            });
        }
        let journal_batch_u32 = usize_to_u32(profile.journal_writer_queue_capacity.get());
        if self.max_journal_batch_bytes > journal_batch_u32 {
            return Err(ContractViolation::ExceedsProfileLimit {
                field: "max_journal_batch_bytes",
                actual: u64::from(self.max_journal_batch_bytes),
                profile_limit: u64::from(journal_batch_u32),
            });
        }
        // Fields without direct profile mapping → validated against hard limits.
        let max_steps_limit = usize_to_u64(MAX_STEPS_PER_WORKFLOW);
        if u64::from(self.max_steps) > max_steps_limit {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_steps",
                actual: u64::from(self.max_steps),
                hard_limit: max_steps_limit,
            });
        }
        let max_slots_limit = usize_to_u64(MAX_SLOTS_PER_WORKFLOW);
        if u64::from(self.max_slots) > max_slots_limit {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_slots",
                actual: u64::from(self.max_slots),
                hard_limit: max_slots_limit,
            });
        }
        let max_constants_limit = usize_to_u64(MAX_CONSTANTS);
        if u64::from(self.max_constants) > max_constants_limit {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_constants",
                actual: u64::from(self.max_constants),
                hard_limit: max_constants_limit,
            });
        }
        let max_accessors_limit = usize_to_u64(MAX_ACCESSORS);
        if u64::from(self.max_accessors) > max_accessors_limit {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_accessors",
                actual: u64::from(self.max_accessors),
                hard_limit: max_accessors_limit,
            });
        }
        let max_expressions_limit = usize_to_u64(MAX_EXPRESSIONS);
        if u64::from(self.max_expressions) > max_expressions_limit {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_expressions",
                actual: u64::from(self.max_expressions),
                hard_limit: max_expressions_limit,
            });
        }
        if u32::from(self.max_expr_stack) > u32::from(MAX_EXPRESSION_STACK) {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_expr_stack",
                actual: u64::from(self.max_expr_stack),
                hard_limit: u64::from(MAX_EXPRESSION_STACK),
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
    pub fn fits_within_hard_limits(&self) -> Result<(), ContractViolation> {
        let max_steps_limit = usize_to_u64(MAX_STEPS_PER_WORKFLOW);
        if u64::from(self.max_steps) > max_steps_limit {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_steps",
                actual: u64::from(self.max_steps),
                hard_limit: max_steps_limit,
            });
        }
        let max_slots_limit = usize_to_u64(MAX_SLOTS_PER_WORKFLOW);
        if u64::from(self.max_slots) > max_slots_limit {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_slots",
                actual: u64::from(self.max_slots),
                hard_limit: max_slots_limit,
            });
        }
        let max_constants_limit = usize_to_u64(MAX_CONSTANTS);
        if u64::from(self.max_constants) > max_constants_limit {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_constants",
                actual: u64::from(self.max_constants),
                hard_limit: max_constants_limit,
            });
        }
        let max_accessors_limit = usize_to_u64(MAX_ACCESSORS);
        if u64::from(self.max_accessors) > max_accessors_limit {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_accessors",
                actual: u64::from(self.max_accessors),
                hard_limit: max_accessors_limit,
            });
        }
        let max_expressions_limit = usize_to_u64(MAX_EXPRESSIONS);
        if u64::from(self.max_expressions) > max_expressions_limit {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_expressions",
                actual: u64::from(self.max_expressions),
                hard_limit: max_expressions_limit,
            });
        }
        if u32::from(self.max_expr_stack) > u32::from(MAX_EXPRESSION_STACK) {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_expr_stack",
                actual: u64::from(self.max_expr_stack),
                hard_limit: u64::from(MAX_EXPRESSION_STACK),
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
                actual: u64::from(self.max_input_bytes),
                hard_limit: u64::from(MAX_INPUT_BYTES),
            });
        }
        if self.max_output_bytes > MAX_OUTPUT_BYTES {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_output_bytes",
                actual: u64::from(self.max_output_bytes),
                hard_limit: u64::from(MAX_OUTPUT_BYTES),
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
                actual: u64::from(self.max_ipc_payload_bytes),
                hard_limit: u64::from(MAX_IPC_PAYLOAD_BYTES),
            });
        }
        if self.max_retry_attempts > MAX_RETRY_ATTEMPTS {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_retry_attempts",
                actual: u64::from(self.max_retry_attempts),
                hard_limit: u64::from(MAX_RETRY_ATTEMPTS),
            });
        }
        if self.max_fanout > MAX_FANOUT {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_fanout",
                actual: u64::from(self.max_fanout),
                hard_limit: u64::from(MAX_FANOUT),
            });
        }
        if self.max_collect_items > MAX_COLLECT_ITEMS {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_collect_items",
                actual: u64::from(self.max_collect_items),
                hard_limit: u64::from(MAX_COLLECT_ITEMS),
            });
        }
        if self.max_queue_depth > MAX_QUEUE_DEPTH {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_queue_depth",
                actual: u64::from(self.max_queue_depth),
                hard_limit: u64::from(MAX_QUEUE_DEPTH),
            });
        }
        if self.max_journal_batch_bytes > MAX_JOURNAL_BATCH_BYTES {
            return Err(ContractViolation::ExceedsHardLimit {
                field: "max_journal_batch_bytes",
                actual: u64::from(self.max_journal_batch_bytes),
                hard_limit: u64::from(MAX_JOURNAL_BATCH_BYTES),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod profile_tests {
    use super::*;

    fn strict_config() -> RuntimeLimitsConfig {
        RuntimeLimitsConfig {
            active_runs: 1,
            ready_queue_depth: 1,
            ipc_frame_bytes: 1,
            action_input_bytes: 1,
            action_output_bytes: 1,
            step_output_bytes: 1,
            result_bytes: 1,
            trace_ring_capacity: 1,
            journal_writer_queue_capacity: 1,
            for_each_item_count: 1,
            together_branch_count: 1,
            collect_pages: 1,
            collect_items: 1,
            collect_time_seconds: 1,
            repeat_attempts: 1,
            repeat_time_seconds: 1,
            retry_attempts: 1,
            max_wait_duration_seconds: 1,
            ask_timeout_seconds: 1,
        }
    }

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
        let mut config = strict_config();
        config.active_runs = 0;
        let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_new_validates_zero_retry_attempts() {
        let mut config = strict_config();
        config.retry_attempts = 0;
        let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
        assert!(result.is_err());
    }
}
