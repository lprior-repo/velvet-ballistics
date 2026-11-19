//! Full JSON value equality tests for event_to_json with RunAdmission policy pins.
//!
//! Exercises BOTH RunPolicyPin::Pinned(distinct policy digest) and
//! RunPolicyPin::Unverified through the actual CLI replay serialization path.
//! Verifies distinct policy and input identity serialization.

#![forbid(unsafe_code)]

use super::event_to_json;
use vb_core::{CapabilitySet, RunId, RuntimePolicy, WorkflowDigest};
use vb_storage::{EventSeq, JournalEvent, RunInputBinding, RunPolicyPin};

fn artifact_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x01; 32])
}

fn input_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x02; 32])
}

fn policy_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x03; 32])
}

fn build_run_admission(policy_pin: RunPolicyPin) -> JournalEvent {
    JournalEvent::RunAdmission {
        input_digest: RunInputBinding::Digest(input_digest()),
        run: RunId::new(1),
        seq: EventSeq::new(1),
        artifact_digest: artifact_digest(),
        policy_pin,
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Strict,
    }
}

fn expected_json(policy_pin: RunPolicyPin) -> serde_json::Value {
    serde_json::json!({
        "seq": 1,
        "type": "RunAdmission",
        "run": 1,
        "artifact_digest": format!("{:?}", artifact_digest()),
        "input_digest": format!("{:?}", RunInputBinding::Digest(input_digest())),
        "granted_capabilities": format!("{:?}", CapabilitySet::empty()),
        "policy": format!("{:?}", RuntimePolicy::Strict),
        "policy_pin": format!("{:?}", policy_pin)
    })
}

#[test]
fn policy_pin_pinned_full_json_equality() {
    let event = build_run_admission(RunPolicyPin::Pinned(policy_digest()));
    let actual = event_to_json(&event);
    let expected = expected_json(RunPolicyPin::Pinned(policy_digest()));
    assert_eq!(
        actual, expected,
        "RunAdmission with Pinned policy must serialize all fields exactly"
    );
}

#[test]
fn policy_pin_unverified_full_json_equality() {
    let event = build_run_admission(RunPolicyPin::Unverified);
    let actual = event_to_json(&event);
    let expected = expected_json(RunPolicyPin::Unverified);
    assert_eq!(
        actual, expected,
        "RunAdmission with Unverified policy must serialize all fields exactly"
    );
}

fn sentinel_budget() -> vb_core::AggregateResourceBudget {
    vb_core::AggregateResourceBudget {
        max_steps_executable: 1,
        max_action_tickets: 1,
        max_parallel_in_flight: 1,
        max_retries_per_action: 1,
        max_gather_pages: 1,
        max_gather_items: 1,
        max_for_each_iterations: 1,
        max_together_branches: 1,
        max_repeat_attempts: 1,
        max_run_time_seconds: 1,
        max_result_bytes: 1,
        max_total_slots_written: 1,
        max_timer_entries: 1,
        max_trace_events: 1,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_blob_bytes: 1,
        max_input_bytes: 1,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        max_open_gates: 1,
        max_signal_subscriptions: 1,
        max_incident_emissions: 1,
        max_transitions: 1,
        max_journal_bytes_total: 1,
        max_value_store_entries: 1,
        max_cost_units: 987654321,
    }
}

fn metadata_events() -> [JournalEvent; 3] {
    let run = RunId::new(73);
    [
        JournalEvent::RunAdmissionBudget {
            run,
            seq: EventSeq::new(1),
            artifact_digest: artifact_digest(),
            input_digest: RunInputBinding::Digest(input_digest()),
            policy_pin: RunPolicyPin::Pinned(policy_digest()),
            granted_capabilities: CapabilitySet::empty(),
            policy: RuntimePolicy::Strict,
            budget: sentinel_budget(),
        },
        JournalEvent::CompatibilityDecisionEvent {
            run,
            seq: EventSeq::new(2),
            decision: vb_storage::compat::decide_boundary(
                vb_storage::compat::artifact_ir_version_inputs(
                    vb_storage::compat::CURRENT_IR_VERSION,
                ),
                [0x71; 32],
                [0x71; 32],
            ),
        },
        JournalEvent::RunBudgetOriginEvent {
            run,
            seq: EventSeq::new(3),
            started_millis: 987654321,
        },
    ]
}

#[test]
fn admission_metadata_projects_only_numeric_identity_and_stable_type() {
    let events = metadata_events();
    let expected = [
        serde_json::json!({"type": "RunAdmissionBudget", "seq": 1, "run": 73}),
        serde_json::json!({"type": "CompatibilityDecisionEvent", "seq": 2, "run": 73}),
        serde_json::json!({"type": "RunBudgetOriginEvent", "seq": 3, "run": 73}),
    ];
    for (event, expected) in events.iter().zip(expected) {
        let actual = event_to_json(event);
        assert_eq!(actual, expected);
        assert!(!actual.to_string().contains("987654321"));
    }
}

#[test]
fn metadata_projection_preserves_neighboring_event_order_and_legacy_fields() {
    let run = RunId::new(73);
    let [admission, compatibility, origin] = metadata_events();
    let events = [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::ZERO,
            workflow: artifact_digest(),
        },
        admission,
        compatibility,
        origin,
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(4),
            step: vb_core::StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(5),
            step: vb_core::StepIdx::new(0),
            output: vb_core::SlotIdx::new(0),
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(6),
            result: vb_core::SlotIdx::new(0),
            attempt: 1,
        },
    ];
    assert_eq!(
        events.map(|event| event_to_json(&event)),
        [
            serde_json::json!({"type":"RunAccepted", "seq":0, "run":73, "workflow":format!("{:?}", artifact_digest())}),
            serde_json::json!({"type":"RunAdmissionBudget", "seq":1, "run":73}),
            serde_json::json!({"type":"CompatibilityDecisionEvent", "seq":2, "run":73}),
            serde_json::json!({"type":"RunBudgetOriginEvent", "seq":3, "run":73}),
            serde_json::json!({"type":"StepStarted", "seq":4, "step":0}),
            serde_json::json!({"type":"StepSucceeded", "seq":5, "output":0}),
            serde_json::json!({"type":"RunFinished", "seq":6, "result":0}),
        ]
    );
}
