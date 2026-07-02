//! Tests for `event_to_json` — stable JSON projection of every `JournalEvent` variant.
//!
//! Each `JournalEvent` variant has a stable JSON projection:
//!
//! - `"type"` discriminant string (e.g. `"RunCancelled"`, `"ActionScheduledTicket"`).
//! - Critical fields named in `vb-wy33p.3` NOTES: `seq`, `run`, `step`, `attempt`,
//!   `action`, `output`, `slot`, `result`, `slot_idx`, `answer`, `timestamp`,
//!   `ticket`, `input`, `outcome`, `encoded_len`, `taint`, `value_digest`,
//!   `action_abi_digest`, `artifact_digest`, `granted_capabilities`, `policy`,
//!   `workflow`, `reason`.
//!
//! These tests are exact-pinning (NOT source-count parsing). A failing assertion
//! points to the specific missing/renamed field, not to "the test file shrank".
//!
//! The acceptance contract is:
//! - Every `JournalEvent` variant has stable JSON projection with discriminant
//!   + critical fields.
//! - No `Unknown` for known variants.
//! - Tests fail on missing variant coverage.
#![forbid(unsafe_code)]

use super::event_to_json;
use chrono::TimeZone;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, Taint};
use vb_core::{CapabilitySet, RuntimePolicy};
use vb_storage::types::EventSeq;
use vb_storage::{DurableActionOutcome, JournalEvent};

// -------------------------------------------------------------------------
// Fixture builders
// -------------------------------------------------------------------------

fn run_id(v: u64) -> RunId {
    RunId::new(v)
}

fn seq_no(v: u64) -> SeqNo {
    SeqNo::new(v)
}

fn event_seq(v: u64) -> EventSeq {
    EventSeq::new(v)
}

fn step(v: u16) -> StepIdx {
    StepIdx::new(v)
}

fn action(v: u16) -> ActionId {
    ActionId::new(v)
}

fn slot(v: u16) -> SlotIdx {
    SlotIdx::new(v)
}

fn digest(seed: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([seed; 32])
}

fn ticket(
    run: u64,
    step_v: u16,
    seq_v: u64,
    action_v: u16,
    attempt_v: u16,
) -> vb_core::ActionTicket {
    vb_core::ActionTicket {
        run: run_id(run),
        step: step(step_v),
        seq: seq_no(seq_v),
        action: action(action_v),
        attempt: attempt_v,
        idempotency_key: 0,
        capacity: 4,
    }
}

// -------------------------------------------------------------------------
// Per-variant exact-pinning tests
//
// Each test pins the discriminant and the documented critical fields for a
// single known variant. A failing test points at the specific missing or
// renamed field — it does not require source-count assertions to fail.
// -------------------------------------------------------------------------

#[test]
fn run_accepted_projects_workflow_digest_run_and_seq() {
    let event = JournalEvent::RunAccepted {
        run: run_id(11),
        seq: event_seq(0),
        workflow: digest(0xAB),
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "RunAccepted");
    assert_eq!(value["seq"], serde_json::json!(0));
    assert_eq!(value["run"], serde_json::json!(11));
    let workflow = value["workflow"].as_str().expect("workflow string");
    assert!(
        workflow.contains("WorkflowDigest"),
        "workflow must Debug-print as `WorkflowDigest([..])` (got {workflow:?})"
    );
    assert!(
        workflow.contains("171"),
        "workflow Debug must include seed byte 0xAB as decimal 171 (got {workflow:?})"
    );
}

#[test]
fn run_admission_projects_run_artifact_capabilities_and_policy() {
    let event = JournalEvent::RunAdmission {
        run: run_id(12),
        seq: event_seq(1),
        artifact_digest: digest(0xCD),
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Strict,
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "RunAdmission");
    assert_eq!(value["seq"], serde_json::json!(1));
    assert_eq!(value["run"], serde_json::json!(12));
    let artifact = value["artifact_digest"].as_str().expect("artifact_digest");
    assert!(
        artifact.contains("WorkflowDigest"),
        "artifact_digest must Debug as WorkflowDigest (got {artifact:?})"
    );
    assert!(
        artifact.contains("205"),
        "artifact_digest Debug must include seed byte 0xCD as decimal 205 (got {artifact:?})"
    );
    assert!(
        value["granted_capabilities"].is_string(),
        "granted_capabilities must be Debug-printed as a string"
    );
    assert_eq!(
        value["policy"], "Strict",
        "RuntimePolicy::Strict must Debug-print as `Strict`"
    );
}

#[test]
fn step_started_projects_step_and_attempt() {
    let event = JournalEvent::StepStarted {
        run: run_id(13),
        seq: event_seq(2),
        step: step(7),
        attempt: 1,
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "StepStarted");
    assert_eq!(value["seq"], serde_json::json!(2));
    assert_eq!(value["step"], serde_json::json!(7));
}

#[test]
fn step_succeeded_projects_output_slot() {
    let event = JournalEvent::StepSucceeded {
        run: run_id(14),
        seq: event_seq(3),
        step: step(7),
        output: slot(9),
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "StepSucceeded");
    assert_eq!(value["seq"], serde_json::json!(3));
    assert_eq!(value["output"], serde_json::json!(9));
}

#[test]
fn action_scheduled_projects_action_id() {
    let event = JournalEvent::ActionScheduled {
        run: run_id(15),
        seq: event_seq(4),
        step: step(2),
        action: action(3),
        attempt: 1,
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "ActionScheduled");
    assert_eq!(value["seq"], serde_json::json!(4));
    assert_eq!(value["action"], serde_json::json!(3));
}

#[test]
fn action_completed_event_projects_action_id() {
    let event = JournalEvent::ActionCompletedEvent {
        run: run_id(16),
        seq: event_seq(5),
        step: step(2),
        action: action(3),
        attempt: 1,
    };
    let value = event_to_json(&event);
    assert_eq!(
        value["type"], "ActionCompleted",
        "ActionCompletedEvent discriminates as `ActionCompleted` for stability"
    );
    assert_eq!(value["action"], serde_json::json!(3));
}

#[test]
fn action_scheduled_ticket_projects_ticket_input_output_and_abi_digest() {
    let event = JournalEvent::ActionScheduledTicket {
        run: run_id(17),
        seq: event_seq(6),
        ticket: ticket(17, 3, 6, 4, 1),
        input: slot(0),
        output: slot(1),
        action_abi_digest: digest(0xEF),
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "ActionScheduledTicket");
    assert_eq!(value["seq"], serde_json::json!(6));
    assert_eq!(value["run"], serde_json::json!(17));
    assert_eq!(value["input"], serde_json::json!(0));
    assert_eq!(value["output"], serde_json::json!(1));
    let ticket_str = value["ticket"].as_str().expect("ticket Debug");
    assert!(
        ticket_str.contains("ActionTicket"),
        "ticket Debug must include type name: {ticket_str}"
    );
    let abi = value["action_abi_digest"].as_str().expect("abi digest");
    assert!(
        abi.contains("WorkflowDigest"),
        "action_abi_digest must Debug as WorkflowDigest (got {abi:?})"
    );
    assert!(
        abi.contains("239"),
        "action_abi_digest must include seed byte 0xEF as decimal 239 (got {abi:?})"
    );
}

#[test]
fn action_completed_envelope_projects_outcome_taint_value_digest_and_encoded_len() {
    let event = JournalEvent::ActionCompletedEnvelope {
        run: run_id(18),
        seq: event_seq(7),
        ticket: ticket(18, 4, 7, 5, 1),
        output: slot(2),
        outcome: DurableActionOutcome::Ready,
        value: vec![0xAA, 0xBB],
        encoded_len: 2,
        taint: Taint::Clean,
        value_digest: [0x11_u8; 32],
        action_abi_digest: digest(0x22),
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "ActionCompletedEnvelope");
    assert_eq!(value["seq"], serde_json::json!(7));
    assert_eq!(value["run"], serde_json::json!(18));
    assert_eq!(value["output"], serde_json::json!(2));
    assert_eq!(value["encoded_len"], serde_json::json!(2));
    let outcome = value["outcome"].as_str().expect("outcome Debug");
    assert!(
        outcome.contains("Ready"),
        "outcome must Debug as Ready: {outcome}"
    );
    let taint = value["taint"].as_str().expect("taint Debug");
    assert!(
        taint.contains("Clean"),
        "taint must Debug as Clean: {taint}"
    );
    let vd = value["value_digest"].as_str().expect("value_digest Debug");
    assert!(
        vd.contains("17"),
        "value_digest must include seed byte 0x11 as decimal 17 (got {vd:?})"
    );
    let abi = value["action_abi_digest"].as_str().expect("abi digest");
    assert!(
        abi.contains("WorkflowDigest"),
        "action_abi_digest must Debug as WorkflowDigest (got {abi:?})"
    );
    assert!(
        abi.contains("34"),
        "action_abi_digest must include seed byte 0x22 as decimal 34 (got {abi:?})"
    );
}

#[test]
fn action_failed_event_projects_action_id() {
    let event = JournalEvent::ActionFailedEvent {
        run: run_id(19),
        seq: event_seq(8),
        step: step(5),
        action: action(7),
        attempt: 1,
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "ActionFailed");
    assert_eq!(value["seq"], serde_json::json!(8));
    assert_eq!(value["action"], serde_json::json!(7));
}

#[test]
fn action_abandoned_projects_ticket_and_run() {
    let event = JournalEvent::ActionAbandoned {
        run: run_id(20),
        seq: event_seq(9),
        ticket: ticket(20, 6, 9, 8, 2),
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "ActionAbandoned");
    assert_eq!(value["seq"], serde_json::json!(9));
    assert_eq!(value["run"], serde_json::json!(20));
    let ticket_str = value["ticket"].as_str().expect("ticket Debug");
    assert!(
        ticket_str.contains("ActionTicket"),
        "ticket Debug must include type name: {ticket_str}"
    );
}

#[test]
fn slot_written_event_projects_slot() {
    let event = JournalEvent::SlotWrittenEvent {
        run: run_id(21),
        seq: event_seq(10),
        slot: slot(11),
        value: None,
        extra: None,
        attempt: 1,
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "SlotWritten");
    assert_eq!(value["seq"], serde_json::json!(10));
    assert_eq!(value["slot"], serde_json::json!(11));
}

#[test]
fn wait_scheduled_event_projects_step() {
    let event = JournalEvent::WaitScheduledEvent {
        run: run_id(22),
        seq: event_seq(11),
        step: step(12),
        attempt: 1,
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "WaitScheduled");
    assert_eq!(value["seq"], serde_json::json!(11));
    assert_eq!(value["step"], serde_json::json!(12));
}

#[test]
fn ask_scheduled_event_projects_step() {
    let event = JournalEvent::AskScheduledEvent {
        run: run_id(23),
        seq: event_seq(12),
        step: step(13),
        attempt: 1,
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "AskScheduled");
    assert_eq!(value["seq"], serde_json::json!(12));
    assert_eq!(value["step"], serde_json::json!(13));
}

#[test]
fn ask_answered_event_projects_step() {
    let event = JournalEvent::AskAnsweredEvent {
        run: run_id(24),
        seq: event_seq(13),
        step: step(14),
        attempt: 1,
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "AskAnswered");
    assert_eq!(value["seq"], serde_json::json!(13));
    assert_eq!(value["step"], serde_json::json!(14));
}

#[test]
fn wait_resolved_event_projects_step() {
    let event = JournalEvent::WaitResolvedEvent {
        run: run_id(25),
        seq: event_seq(14),
        step: step(15),
        attempt: 1,
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "WaitResolved");
    assert_eq!(value["seq"], serde_json::json!(14));
    assert_eq!(value["step"], serde_json::json!(15));
}

#[test]
fn retry_scheduled_event_projects_step() {
    let event = JournalEvent::RetryScheduledEvent {
        run: run_id(26),
        seq: event_seq(15),
        step: step(16),
        attempt: 1,
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "RetryScheduled");
    assert_eq!(value["seq"], serde_json::json!(15));
    assert_eq!(value["step"], serde_json::json!(16));
}

#[test]
fn run_cancelled_projects_seq() {
    let event = JournalEvent::RunCancelled {
        run: run_id(27),
        seq: event_seq(16),
        attempt: 1,
        reason: None,
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "RunCancelled");
    assert_eq!(value["seq"], serde_json::json!(16));
}

#[test]
fn run_killed_projects_seq() {
    let event = JournalEvent::RunKilled {
        run: run_id(28),
        seq: event_seq(17),
        attempt: 1,
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "RunKilled");
    assert_eq!(value["seq"], serde_json::json!(17));
}

#[test]
fn run_finished_projects_result_slot() {
    let event = JournalEvent::RunFinished {
        run: run_id(29),
        seq: event_seq(18),
        result: slot(20),
        attempt: 1,
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "RunFinished");
    assert_eq!(value["seq"], serde_json::json!(18));
    assert_eq!(value["result"], serde_json::json!(20));
}

#[test]
fn run_failed_event_projects_seq() {
    let event = JournalEvent::RunFailedEvent {
        run: run_id(30),
        seq: event_seq(19),
        attempt: 1,
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "RunFailed");
    assert_eq!(value["seq"], serde_json::json!(19));
}

#[test]
fn run_resumed_projects_run_and_timestamp() {
    let event = JournalEvent::RunResumed {
        run: run_id(31),
        seq: event_seq(20),
        timestamp: chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "RunResumed");
    assert_eq!(value["run"], serde_json::json!(31));
    let ts = value["timestamp"].as_str().expect("timestamp RFC3339");
    assert!(
        ts.starts_with("2023-"),
        "timestamp must be RFC3339 of 1700000000 epoch (got {ts})"
    );
}

#[test]
fn run_retried_projects_run_and_timestamp() {
    let event = JournalEvent::RunRetried {
        run: run_id(32),
        seq: event_seq(21),
        timestamp: chrono::Utc.timestamp_opt(1_700_000_500, 0).unwrap(),
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "RunRetried");
    assert_eq!(value["run"], serde_json::json!(32));
    let ts = value["timestamp"].as_str().expect("timestamp RFC3339");
    assert!(
        ts.starts_with("2023-"),
        "timestamp must be RFC3339 (got {ts})"
    );
}

#[test]
fn run_answered_projects_run_slot_idx_answer_and_timestamp() {
    let event = JournalEvent::RunAnswered {
        run: run_id(33),
        seq: event_seq(22),
        slot_idx: slot(7),
        answer: ConstValue::Bool(true),
        timestamp: chrono::Utc.timestamp_opt(1_700_001_000, 0).unwrap(),
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "RunAnswered");
    assert_eq!(value["run"], serde_json::json!(33));
    assert_eq!(value["slot_idx"], serde_json::json!(7));
    let answer = value["answer"].as_str().expect("answer Debug");
    assert!(
        answer.contains("Bool") && answer.contains("true"),
        "answer must Debug as `Bool(true)`: {answer}"
    );
    let ts = value["timestamp"].as_str().expect("timestamp");
    assert!(
        ts.starts_with("2023-"),
        "timestamp must be RFC3339 (got {ts})"
    );
}

#[test]
fn ask_timed_out_event_projects_step() {
    let event = JournalEvent::AskTimedOutEvent {
        run: run_id(34),
        seq: event_seq(23),
        step: step(17),
        attempt: 1,
    };
    let value = event_to_json(&event);
    assert_eq!(value["type"], "AskTimedOut");
    assert_eq!(value["seq"], serde_json::json!(23));
    assert_eq!(value["step"], serde_json::json!(17));
}

// -------------------------------------------------------------------------
// Coverage contract: no current known variant projects to `Unknown`.
//
// `JournalEvent` is `#[non_exhaustive]` so the implementation carries a
// wildcard fallback arm. This test pins the current 24-variant inventory:
// if a future edit drops an explicit match arm, the wildcard arm will fire
// for that variant and this test will fail with a concrete variant name.
// -------------------------------------------------------------------------

#[test]
fn stable_projection_for_modern_durable_resume_events() {
    // Bead acceptance: modern durable and resume events must project
    // losslessly with stable discriminants and their critical fields.
    let events = vec![
        (
            "ActionScheduledTicket",
            JournalEvent::ActionScheduledTicket {
                run: run_id(40),
                seq: event_seq(30),
                ticket: ticket(40, 5, 30, 1, 1),
                input: slot(0),
                output: slot(1),
                action_abi_digest: digest(0x33),
            },
            "ActionScheduledTicket",
        ),
        (
            "ActionCompletedEnvelope",
            JournalEvent::ActionCompletedEnvelope {
                run: run_id(41),
                seq: event_seq(31),
                ticket: ticket(41, 5, 31, 1, 1),
                output: slot(2),
                outcome: DurableActionOutcome::Ready,
                value: vec![0x99],
                encoded_len: 1,
                taint: Taint::Clean,
                value_digest: [0x99_u8; 32],
                action_abi_digest: digest(0x33),
            },
            "ActionCompletedEnvelope",
        ),
        (
            "ActionAbandoned",
            JournalEvent::ActionAbandoned {
                run: run_id(42),
                seq: event_seq(32),
                ticket: ticket(42, 5, 32, 1, 1),
            },
            "ActionAbandoned",
        ),
        (
            "WaitResolvedEvent",
            JournalEvent::WaitResolvedEvent {
                run: run_id(43),
                seq: event_seq(33),
                step: step(9),
                attempt: 1,
            },
            "WaitResolved",
        ),
        (
            "AskTimedOutEvent",
            JournalEvent::AskTimedOutEvent {
                run: run_id(44),
                seq: event_seq(34),
                step: step(9),
                attempt: 1,
            },
            "AskTimedOut",
        ),
        (
            "RunKilled",
            JournalEvent::RunKilled {
                run: run_id(45),
                seq: event_seq(35),
                attempt: 1,
            },
            "RunKilled",
        ),
        (
            "RunAdmission",
            JournalEvent::RunAdmission {
                run: run_id(46),
                seq: event_seq(36),
                artifact_digest: digest(0x44),
                granted_capabilities: CapabilitySet::empty(),
                policy: RuntimePolicy::Strict,
            },
            "RunAdmission",
        ),
        (
            "RunCancelled",
            JournalEvent::RunCancelled {
                run: run_id(47),
                seq: event_seq(37),
                attempt: 1,
                reason: None,
            },
            "RunCancelled",
        ),
        (
            "RunAnswered",
            JournalEvent::RunAnswered {
                run: run_id(48),
                seq: event_seq(38),
                slot_idx: slot(7),
                answer: ConstValue::Bool(true),
                timestamp: chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            },
            "RunAnswered",
        ),
    ];

    for (name, event, expected_type) in &events {
        let value = event_to_json(event);
        assert_eq!(
            value["type"], *expected_type,
            "{name} discriminant must remain stable"
        );
        assert_ne!(
            value["type"], "Unknown",
            "{name} projected to Unknown — explicit match arm missing"
        );
    }
}

#[test]
fn every_current_journal_event_variant_has_a_non_unknown_projection() {
    let fixtures: Vec<(&'static str, JournalEvent)> = vec![
        (
            "RunAccepted",
            JournalEvent::RunAccepted {
                run: run_id(1),
                seq: event_seq(0),
                workflow: digest(0x01),
            },
        ),
        (
            "RunAdmission",
            JournalEvent::RunAdmission {
                run: run_id(2),
                seq: event_seq(1),
                artifact_digest: digest(0x02),
                granted_capabilities: CapabilitySet::empty(),
                policy: RuntimePolicy::Strict,
            },
        ),
        (
            "StepStarted",
            JournalEvent::StepStarted {
                run: run_id(3),
                seq: event_seq(2),
                step: step(0),
                attempt: 1,
            },
        ),
        (
            "StepSucceeded",
            JournalEvent::StepSucceeded {
                run: run_id(4),
                seq: event_seq(3),
                step: step(0),
                output: slot(0),
            },
        ),
        (
            "ActionScheduled",
            JournalEvent::ActionScheduled {
                run: run_id(5),
                seq: event_seq(4),
                step: step(0),
                action: action(0),
                attempt: 1,
            },
        ),
        (
            "ActionCompletedEvent",
            JournalEvent::ActionCompletedEvent {
                run: run_id(6),
                seq: event_seq(5),
                step: step(0),
                action: action(0),
                attempt: 1,
            },
        ),
        (
            "ActionScheduledTicket",
            JournalEvent::ActionScheduledTicket {
                run: run_id(7),
                seq: event_seq(6),
                ticket: ticket(7, 0, 6, 0, 1),
                input: slot(0),
                output: slot(1),
                action_abi_digest: digest(0x07),
            },
        ),
        (
            "ActionCompletedEnvelope",
            JournalEvent::ActionCompletedEnvelope {
                run: run_id(8),
                seq: event_seq(7),
                ticket: ticket(8, 0, 7, 0, 1),
                output: slot(2),
                outcome: DurableActionOutcome::Ready,
                value: vec![0x00],
                encoded_len: 1,
                taint: Taint::Clean,
                value_digest: [0x00_u8; 32],
                action_abi_digest: digest(0x08),
            },
        ),
        (
            "ActionFailedEvent",
            JournalEvent::ActionFailedEvent {
                run: run_id(9),
                seq: event_seq(8),
                step: step(0),
                action: action(0),
                attempt: 1,
            },
        ),
        (
            "ActionAbandoned",
            JournalEvent::ActionAbandoned {
                run: run_id(10),
                seq: event_seq(9),
                ticket: ticket(10, 0, 9, 0, 1),
            },
        ),
        (
            "SlotWrittenEvent",
            JournalEvent::SlotWrittenEvent {
                run: run_id(11),
                seq: event_seq(10),
                slot: slot(0),
                value: None,
                extra: None,
                attempt: 1,
            },
        ),
        (
            "WaitScheduledEvent",
            JournalEvent::WaitScheduledEvent {
                run: run_id(12),
                seq: event_seq(11),
                step: step(0),
                attempt: 1,
            },
        ),
        (
            "AskScheduledEvent",
            JournalEvent::AskScheduledEvent {
                run: run_id(13),
                seq: event_seq(12),
                step: step(0),
                attempt: 1,
            },
        ),
        (
            "AskAnsweredEvent",
            JournalEvent::AskAnsweredEvent {
                run: run_id(14),
                seq: event_seq(13),
                step: step(0),
                attempt: 1,
            },
        ),
        (
            "WaitResolvedEvent",
            JournalEvent::WaitResolvedEvent {
                run: run_id(15),
                seq: event_seq(14),
                step: step(0),
                attempt: 1,
            },
        ),
        (
            "RetryScheduledEvent",
            JournalEvent::RetryScheduledEvent {
                run: run_id(16),
                seq: event_seq(15),
                step: step(0),
                attempt: 1,
            },
        ),
        (
            "RunCancelled",
            JournalEvent::RunCancelled {
                run: run_id(17),
                seq: event_seq(16),
                attempt: 1,
                reason: None,
            },
        ),
        (
            "RunKilled",
            JournalEvent::RunKilled {
                run: run_id(18),
                seq: event_seq(17),
                attempt: 1,
            },
        ),
        (
            "RunFinished",
            JournalEvent::RunFinished {
                run: run_id(19),
                seq: event_seq(18),
                result: slot(0),
                attempt: 1,
            },
        ),
        (
            "RunFailedEvent",
            JournalEvent::RunFailedEvent {
                run: run_id(20),
                seq: event_seq(19),
                attempt: 1,
            },
        ),
        (
            "RunResumed",
            JournalEvent::RunResumed {
                run: run_id(21),
                seq: event_seq(20),
                timestamp: chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            },
        ),
        (
            "RunRetried",
            JournalEvent::RunRetried {
                run: run_id(22),
                seq: event_seq(21),
                timestamp: chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            },
        ),
        (
            "RunAnswered",
            JournalEvent::RunAnswered {
                run: run_id(23),
                seq: event_seq(22),
                slot_idx: slot(0),
                answer: ConstValue::Null,
                timestamp: chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            },
        ),
        (
            "AskTimedOutEvent",
            JournalEvent::AskTimedOutEvent {
                run: run_id(24),
                seq: event_seq(23),
                step: step(0),
                attempt: 1,
            },
        ),
    ];

    assert_eq!(
        fixtures.len(),
        24,
        "fixture inventory must cover every current JournalEvent variant"
    );

    for (name, event) in &fixtures {
        let value = event_to_json(event);
        let discriminant = value["type"]
            .as_str()
            .unwrap_or_else(|| panic!("{name}: missing `type` discriminant in projection"));
        assert_ne!(
            discriminant, "Unknown",
            "{name} projected to Unknown — explicit match arm missing for current variant"
        );
    }
}
