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
    assert!(
        matches!(
            result,
            Err(ProfileValidationError::ExceedsHardLimit { field: "active_runs", value: 0, .. })
        ),
        "zero active_runs must surface ExceedsHardLimit with field=active_runs, value=0, got {result:?}"
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
            Err(ProfileValidationError::ExceedsHardLimit { field: "retry_attempts", value: 0, .. })
        ),
        "zero retry_attempts must surface ExceedsHardLimit with field=retry_attempts, value=0, got {result:?}"
    );
}
