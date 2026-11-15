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

#[test]
fn step_and_slot_events_retain_all_fields() {
    use vb_core::{SlotIdx, StepIdx};
    let cases = [
        (
            JournalEvent::StepStarted {
                run: RunId::new(11),
                seq: EventSeq::new(23),
                step: StepIdx::new(7),
                attempt: 3,
            },
            serde_json::json!({"type":"StepStarted","run":11,"seq":23,"step":7,"attempt":3}),
        ),
        (
            JournalEvent::StepSucceeded {
                run: RunId::new(12),
                seq: EventSeq::new(24),
                step: StepIdx::new(8),
                output: SlotIdx::new(9),
            },
            serde_json::json!({"type":"StepSucceeded","run":12,"seq":24,"step":8,"output":9}),
        ),
        (
            JournalEvent::StepFailed {
                run: RunId::new(13),
                seq: EventSeq::new(25),
                step: StepIdx::new(10),
                attempt: 4,
            },
            serde_json::json!({"type":"StepFailed","run":13,"seq":25,"step":10,"attempt":4}),
        ),
        (
            JournalEvent::SlotWrittenEvent {
                run: RunId::new(14),
                seq: EventSeq::new(26),
                slot: SlotIdx::new(11),
                value: Some(vec![3, 5]),
                extra: Some(vec![7, 9]),
                attempt: 5,
            },
            serde_json::json!({"type":"SlotWritten","run":14,"seq":26,"slot":11,"value":[3,5],"extra":[7,9],"attempt":5}),
        ),
        (
            JournalEvent::SlotWrittenEvent {
                run: RunId::new(15),
                seq: EventSeq::new(27),
                slot: SlotIdx::new(12),
                value: None,
                extra: None,
                attempt: 6,
            },
            serde_json::json!({"type":"SlotWritten","run":15,"seq":27,"slot":12,"value":null,"extra":null,"attempt":6}),
        ),
    ];
    for (event, expected) in cases {
        assert_eq!(event_to_json(&event), expected);
    }
}

#[test]
fn lifecycle_events_retain_sequence_reason_and_attempt() -> Result<(), Box<dyn std::error::Error>> {
    use chrono::{TimeZone, Utc};
    let timestamp = Utc
        .with_ymd_and_hms(2026, 3, 4, 5, 6, 7)
        .single()
        .ok_or("invalid fixture time")?;
    let cases = [
        (
            JournalEvent::RunCancelled {
                run: RunId::new(31),
                seq: EventSeq::new(41),
                attempt: 2,
                reason: Some("operator stop".into()),
            },
            serde_json::json!({"type":"RunCancelled","run":31,"seq":41,"attempt":2,"reason":"operator stop"}),
        ),
        (
            JournalEvent::RunKilled {
                run: RunId::new(32),
                seq: EventSeq::new(42),
                attempt: 3,
            },
            serde_json::json!({"type":"RunKilled","run":32,"seq":42,"attempt":3}),
        ),
        (
            JournalEvent::RunFailedEvent {
                run: RunId::new(33),
                seq: EventSeq::new(43),
                attempt: 4,
            },
            serde_json::json!({"type":"RunFailed","run":33,"seq":43,"attempt":4}),
        ),
        (
            JournalEvent::RunResumed {
                run: RunId::new(34),
                seq: EventSeq::new(44),
                timestamp,
            },
            serde_json::json!({"type":"RunResumed","run":34,"seq":44,"timestamp":"2026-03-04T05:06:07+00:00"}),
        ),
        (
            JournalEvent::RunRetried {
                run: RunId::new(35),
                seq: EventSeq::new(45),
                timestamp,
            },
            serde_json::json!({"type":"RunRetried","run":35,"seq":45,"timestamp":"2026-03-04T05:06:07+00:00"}),
        ),
        (
            JournalEvent::RunAnswered {
                run: RunId::new(36),
                seq: EventSeq::new(46),
                slot_idx: vb_core::SlotIdx::new(8),
                answer: vb_core::ConstValue::I64(-17),
                timestamp,
            },
            serde_json::json!({"type":"RunAnswered","run":36,"seq":46,"slot_idx":8,"answer":"I64(-17)","timestamp":"2026-03-04T05:06:07+00:00"}),
        ),
        (
            JournalEvent::RunFinished {
                run: RunId::new(37),
                seq: EventSeq::new(47),
                result: vb_core::SlotIdx::new(9),
                attempt: 5,
            },
            serde_json::json!({"type":"RunFinished","run":37,"seq":47,"result":9,"attempt":5}),
        ),
    ];
    for (event, expected) in cases {
        assert_eq!(event_to_json(&event), expected);
    }
    Ok(())
}
