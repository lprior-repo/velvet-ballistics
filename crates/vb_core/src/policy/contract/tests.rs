use super::*;
use crate::policy::profile_validation_error::ProfileValidationError;

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
    assert_eq!(
        rc.fits_within_hard_limits(),
        Ok(()),
        "strict profile contract must fit hard limits"
    );
}

#[test]
fn test_resource_contract_fits_within_profile() {
    let p = RuntimeLimitsProfile::strict();
    let rc = p.to_resource_contract();
    assert_eq!(
        rc.fits_within_profile(&p),
        Ok(()),
        "contract derived from profile must fit that profile"
    );
}

#[test]
fn test_new_validates_zero_active_runs() {
    let mut config = strict_config();
    config.active_runs = 0;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "active_runs",
                value: 0,
                ..
            })
        ),
        "zero active_runs must surface ExceedsHardLimit with field=active_runs, value=0, got {result:?}"
    );
}

#[test]
fn test_new_validates_journal_writer_queue_capacity_exceeds_max_journal_batch_bytes() {
    // CV-104: a profile with `journal_writer_queue_capacity > MAX_JOURNAL_BATCH_BYTES`
    // would pass the existing u32::MAX fit check but produce a resource contract
    // whose `max_journal_batch_bytes` exceeds the hard limit. The constructor
    // must reject the value at validation time so the resulting contract cannot
    // exceed MAX_JOURNAL_BATCH_BYTES (= 16_777_216).
    let mut config = strict_config();
    config.journal_writer_queue_capacity = MAX_JOURNAL_BATCH_BYTES as usize + 1;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "journal_writer_queue_capacity",
                value: v,
                limit: l,
            }) if v > l && l == u64::from(MAX_JOURNAL_BATCH_BYTES)
        ),
        "journal_writer_queue_capacity > MAX_JOURNAL_BATCH_BYTES must surface ExceedsHardLimit \
         with limit=MAX_JOURNAL_BATCH_BYTES, got {result:?}"
    );
}

#[test]
fn test_new_accepts_journal_writer_queue_capacity_at_max_journal_batch_bytes_boundary() {
    // CV-104 boundary: MAX_JOURNAL_BATCH_BYTES (16_777_216) must be accepted
    // (the check is `> MAX_JOURNAL_BATCH_BYTES`, not `>=`).
    let mut config = strict_config();
    config.journal_writer_queue_capacity = MAX_JOURNAL_BATCH_BYTES as usize;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        result.is_ok(),
        "journal_writer_queue_capacity == MAX_JOURNAL_BATCH_BYTES must be accepted (boundary), got {result:?}"
    );
}

#[test]
fn test_new_validates_zero_retry_attempts() {
    let mut config = strict_config();
    config.retry_attempts = 0;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "retry_attempts",
                value: 0,
                ..
            })
        ),
        "zero retry_attempts must surface ExceedsHardLimit with field=retry_attempts, value=0, got {result:?}"
    );
}

#[test]
fn test_new_validates_zero_trace_ring_capacity() {
    let mut config = strict_config();
    config.trace_ring_capacity = 0;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "trace_ring_capacity",
                value: 0,
                ..
            })
        ),
        "zero trace_ring_capacity must surface ExceedsHardLimit with field=trace_ring_capacity, value=0, got {result:?}"
    );
}

#[test]
fn test_new_validates_trace_ring_capacity_exceeds_limit() {
    let mut config = strict_config();
    config.trace_ring_capacity = MAX_TRACE_RING_CAPACITY + 1;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "trace_ring_capacity",
                value: v,
                limit: l,
            }) if v > l
        ),
        "trace_ring_capacity > MAX_TRACE_RING_CAPACITY must surface ExceedsHardLimit, got {result:?}"
    );
}

#[test]
fn test_new_validates_zero_for_each_item_count() {
    let mut config = strict_config();
    config.for_each_item_count = 0;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "for_each_item_count",
                value: 0,
                ..
            })
        ),
        "zero for_each_item_count must surface ExceedsHardLimit, got {result:?}"
    );
}

#[test]
fn test_new_validates_for_each_item_count_exceeds_limit() {
    let mut config = strict_config();
    config.for_each_item_count = MAX_FOR_EACH_ITEMS + 1;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "for_each_item_count",
                ..
            })
        ),
        "for_each_item_count > MAX_FOR_EACH_ITEMS must surface ExceedsHardLimit, got {result:?}"
    );
}

#[test]
fn test_new_validates_zero_collect_pages() {
    let mut config = strict_config();
    config.collect_pages = 0;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "collect_pages",
                value: 0,
                ..
            })
        ),
        "zero collect_pages must surface ExceedsHardLimit, got {result:?}"
    );
}

#[test]
fn test_new_validates_collect_pages_exceeds_limit() {
    let mut config = strict_config();
    config.collect_pages = MAX_COLLECT_PAGES + 1;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "collect_pages",
                ..
            })
        ),
        "collect_pages > MAX_COLLECT_PAGES must surface ExceedsHardLimit, got {result:?}"
    );
}

#[test]
fn test_new_validates_zero_collect_time_seconds() {
    let mut config = strict_config();
    config.collect_time_seconds = 0;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "collect_time_seconds",
                value: 0,
                ..
            })
        ),
        "zero collect_time_seconds must surface ExceedsHardLimit, got {result:?}"
    );
}

#[test]
fn test_new_validates_collect_time_seconds_exceeds_limit() {
    let mut config = strict_config();
    config.collect_time_seconds = MAX_COLLECT_TIME_SECONDS + 1;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "collect_time_seconds",
                ..
            })
        ),
        "collect_time_seconds > MAX_COLLECT_TIME_SECONDS must surface ExceedsHardLimit, got {result:?}"
    );
}

#[test]
fn test_new_validates_zero_repeat_time_seconds() {
    let mut config = strict_config();
    config.repeat_time_seconds = 0;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "repeat_time_seconds",
                value: 0,
                ..
            })
        ),
        "zero repeat_time_seconds must surface ExceedsHardLimit, got {result:?}"
    );
}

#[test]
fn test_new_validates_repeat_time_seconds_exceeds_limit() {
    let mut config = strict_config();
    config.repeat_time_seconds = MAX_REPEAT_TIME_SECONDS + 1;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "repeat_time_seconds",
                ..
            })
        ),
        "repeat_time_seconds > MAX_REPEAT_TIME_SECONDS must surface ExceedsHardLimit, got {result:?}"
    );
}

#[test]
fn test_new_validates_zero_max_wait_duration_seconds() {
    let mut config = strict_config();
    config.max_wait_duration_seconds = 0;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "max_wait_duration_seconds",
                value: 0,
                ..
            })
        ),
        "zero max_wait_duration_seconds must surface ExceedsHardLimit, got {result:?}"
    );
}

#[test]
fn test_new_validates_max_wait_duration_seconds_exceeds_limit() {
    let mut config = strict_config();
    config.max_wait_duration_seconds = MAX_WAIT_DURATION_SECONDS + 1;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "max_wait_duration_seconds",
                ..
            })
        ),
        "max_wait_duration_seconds > MAX_WAIT_DURATION_SECONDS must surface ExceedsHardLimit, got {result:?}"
    );
}

#[test]
fn test_new_validates_zero_ask_timeout_seconds() {
    let mut config = strict_config();
    config.ask_timeout_seconds = 0;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "ask_timeout_seconds",
                value: 0,
                ..
            })
        ),
        "zero ask_timeout_seconds must surface ExceedsHardLimit, got {result:?}"
    );
}

#[test]
fn test_new_validates_ask_timeout_seconds_exceeds_limit() {
    let mut config = strict_config();
    config.ask_timeout_seconds = MAX_ASK_TIMEOUT_SECONDS + 1;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit {
                field: "ask_timeout_seconds",
                ..
            })
        ),
        "ask_timeout_seconds > MAX_ASK_TIMEOUT_SECONDS must surface ExceedsHardLimit, got {result:?}"
    );
}

#[test]
fn test_new_validates_max_trace_ring_capacity_boundary() {
    let mut config = strict_config();
    config.trace_ring_capacity = MAX_TRACE_RING_CAPACITY;
    let result = RuntimeLimitsProfile::new(ProfileName::Strict, config);
    assert!(
        result.is_ok(),
        "trace_ring_capacity == MAX_TRACE_RING_CAPACITY must be accepted, got {result:?}"
    );
}

#[test]
fn test_canonical_profiles_validate_against_hard_limits() {
    assert!(
        RuntimeLimitsProfile::strict()
            .to_policy()
            .absolute_max_trace_events
            <= u64::try_from(MAX_TRACE_RING_CAPACITY).unwrap_or(u64::MAX)
    );
    assert!(
        RuntimeLimitsProfile::journaled()
            .to_policy()
            .absolute_max_trace_events
            <= u64::try_from(MAX_TRACE_RING_CAPACITY).unwrap_or(u64::MAX)
    );
    assert!(
        RuntimeLimitsProfile::relaxed()
            .to_policy()
            .absolute_max_trace_events
            <= u64::try_from(MAX_TRACE_RING_CAPACITY).unwrap_or(u64::MAX)
    );
}
