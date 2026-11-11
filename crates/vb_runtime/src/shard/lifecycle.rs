#![forbid(unsafe_code)]
//! Run lifecycle management: submit, resume, cancel, action completion, timers.

#[cfg(test)]
#[path = "lifecycle_tests/duplicate_retry_bridge_tests.rs"]
mod duplicate_retry_bridge_tests;

#[path = "lifecycle/execution_binding.rs"]
mod execution_binding;
#[path = "lifecycle/fresh_inputs.rs"]
mod fresh_inputs;
#[path = "lifecycle/owned_activation.rs"]
pub(crate) mod owned_activation;
#[path = "lifecycle/recovered_admission.rs"]
mod recovered_admission;
#[path = "lifecycle/recovered_store.rs"]
mod recovered_store;
#[path = "lifecycle/registered_completion.rs"]
mod registered_completion;
#[path = "lifecycle/registered_completion_context.rs"]
mod registered_completion_context;
#[path = "lifecycle/registered_context_helpers.rs"]
mod registered_context_helpers;
pub(crate) use registered_completion::RegisteredActionContext;

include!("lifecycle/chunk_003.rs");
include!("lifecycle/chunk_001.rs");
include!("lifecycle/chunk_002.rs");
include!("lifecycle/timer_clock.rs");
include!("lifecycle/chunk_006.rs");
include!("lifecycle/chunk_005_prepare.rs");
include!("lifecycle/chunk_005_drive_apply.rs");
include!("lifecycle/chunk_005.rs");
include!("lifecycle/chunk_004.rs");
include!("lifecycle/gate_submission.rs");

#[cfg(test)]
mod tests {
    include!("lifecycle_tests/chunk_001.rs");
    include!("lifecycle_tests/chunk_002.rs");
    include!("lifecycle_tests/chunk_003.rs");
    include!("lifecycle_tests/chunk_004.rs");
    include!("lifecycle_tests/submission_pair_tests.rs");
    include!("lifecycle_tests/chunk_005.rs");
    include!("lifecycle_tests/chunk_006.rs");
    include!("lifecycle_tests/chunk_007.rs");
    include!("lifecycle_tests/chunk_008.rs");
    include!("lifecycle_tests/chunk_009.rs");
    include!("lifecycle_tests/chunk_010.rs");
    include!("lifecycle_tests/chunk_011.rs");
    include!("lifecycle_tests/chunk_012.rs");
    include!("lifecycle_tests/recovery_context_tests.rs");
    include!("lifecycle_tests/contract_validation_tests.rs");
    include!("lifecycle_tests/action_recovery_tests.rs");
    include!("lifecycle_tests/timer_due_regressions.rs");
    include!("lifecycle_tests/chunk_013.rs");
    include!("lifecycle_tests/chunk_014.rs");
    include!("lifecycle_tests/timer_wheel_cancel_authority_helpers.rs");
    include!("lifecycle_tests/timer_wheel_integration_regressions.rs");
    include!("lifecycle_tests/timer_wheel_capacity_regressions.rs");
    include!("lifecycle_tests/timer_wheel_authority_regressions.rs");
    include!("lifecycle_tests/timer_wheel_replacement_clock_regressions.rs");
    include!("lifecycle_tests/timer_wheel_recovery_regressions.rs");
    include!("lifecycle_tests/timer_wheel_recovery_policy_regressions.rs");
    include!("lifecycle_tests/timer_wheel_snapshot_bounds_fixture.rs");
    include!("lifecycle_tests/timer_wheel_snapshot_bounds_regressions.rs");
    include!("lifecycle_tests/timer_wheel_failure_retry_regressions.rs");
    include!("lifecycle_tests/timer_wheel_ask_answer_regressions.rs");
    include!("lifecycle_tests/bound_inputs_regressions.rs");

    mod bound_symbol_checkpoint {
        include!("lifecycle_tests/bound_symbol_checkpoint_fixture_tests.rs");
        include!("lifecycle_tests/bound_symbol_checkpoint_snapshot_tests.rs");
        include!("lifecycle_tests/bound_symbol_checkpoint_tests.rs");
    }

    mod strict_policy_sequence {
        include!("lifecycle_tests/strict_policy_sequence.rs");
    }

    mod strict_gate_sequence {
        include!("lifecycle_tests/strict_gate_sequence.rs");
    }

    mod strict_gate_opened_sequence {
        include!("lifecycle_tests/strict_gate_opened_sequence.rs");
    }

    mod gate_submission {
        include!("lifecycle_tests/gate_submission.rs");
    }
}
