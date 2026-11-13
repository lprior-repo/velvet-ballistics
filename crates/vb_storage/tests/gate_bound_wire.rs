//! Additive bound-gate wire compatibility (vb-ifn5b).
//!
//! Old RecordKind 44/48 payloads keep their layout. Bound records use
//! appended kinds 54/55. Ids 52/53 stay unassigned.

#![forbid(unsafe_code)]

use vb_core::ids::{ActionId, RunId};
use vb_core::manual_gate::{
    ApprovalReceipt, ApproverId, ApproverRole, BoundApprovalReceipt, BoundGateOutcomeRecord,
    BoundManualGate, GateBinding, GateDecision, GateDigest, GateId, GateOutcome, GateOutcomeRecord,
    ManualGate, OperatorTrust, RiskClass, SeparationOfDuties,
};
use vb_storage::codec::{decode_journal_event, encode_record};
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use vb_storage::events::JournalEvent;
use vb_storage::records::RecordKind;
use vb_storage::types::EventSeq;

fn digest(byte: u8) -> GateDigest {
    GateDigest::from_bytes([byte; 32])
}

fn role() -> ApproverRole {
    ApproverRole::new("deploy-approver".to_string()).expect("role")
}

fn person(value: &str) -> ApproverId {
    ApproverId::new(value.to_string()).expect("person")
}

fn binding() -> GateBinding {
    GateBinding {
        artifact_digest: digest(4),
        policy_digest: digest(5),
        input_digest: digest(6),
        attempt: 1,
    }
}

fn gate(run: RunId) -> ManualGate {
    ManualGate {
        id: GateId::new(7),
        run,
        action: ActionId::new(9),
        risk: RiskClass::High,
        compensatable: false,
        required_role: role(),
        separation_of_duties: SeparationOfDuties::DistinctFromRequester,
        evidence_bundle_digest: digest(1),
        prompt_digest: digest(2),
        timeout_ms: 1_000,
        escalation_policy_digest: digest(3),
        break_glass_allowed: true,
        approval_reason_required: true,
    }
}

fn record(run: RunId) -> GateOutcomeRecord {
    let gate = gate(run);
    let receipt = ApprovalReceipt::bind(
        gate.id,
        gate.run,
        GateDecision::Approve,
        person("alice"),
        gate.evidence_bundle_digest,
        gate.prompt_digest,
        500,
        digest(9),
    )
    .expect("approval receipt");
    GateOutcomeRecord {
        gate: gate.id,
        run: gate.run,
        action: gate.action,
        decision: Some(GateDecision::Approve),
        outcome: GateOutcome::Approved,
        approver: Some(person("alice")),
        approver_role: Some(role()),
        requester: person("bob"),
        reason: Some("checked".to_string()),
        evidence_shown: true,
        evidence_bundle_digest: gate.evidence_bundle_digest,
        prompt_digest: gate.prompt_digest,
        escalation_policy_digest: gate.escalation_policy_digest,
        delegated_to: None,
        break_glass: false,
        break_glass_justification: None,
        opened_at_ms: 0,
        decided_at_ms: 500,
        receipt: Some(receipt),
    }
}

fn bound_record(run: RunId) -> BoundGateOutcomeRecord {
    let record = record(run);
    let old_receipt = record.receipt.clone().expect("receipt");
    let bound = BoundApprovalReceipt {
        receipt: old_receipt,
        action: record.action,
        requester: person("bob"),
        approver_role: role(),
        risk: RiskClass::High,
        separation: SeparationOfDuties::DistinctFromRequester,
        binding: binding(),
        opening_seq: 0,
        operator_trust: OperatorTrust::InProcessCaller,
        digest: digest(7),
    };
    BoundGateOutcomeRecord {
        record,
        binding: binding(),
        opening_seq: 0,
        operator_trust: Some(OperatorTrust::InProcessCaller),
        bound_receipt: Some(bound),
    }
}

#[test]
fn bound_record_kinds_are_appended_without_moving_old_ids() {
    assert_eq!(RecordKind::GateDecision.id(), 44);
    assert_eq!(RecordKind::GateOpened.id(), 48);
    assert_eq!(RecordKind::GateOpenedBound.id(), 54);
    assert_eq!(RecordKind::GateDecisionBound.id(), 55);
    assert_eq!(RecordKind::from_id(54), Some(RecordKind::GateOpenedBound));
    assert_eq!(RecordKind::from_id(55), Some(RecordKind::GateDecisionBound));
    assert_eq!(RecordKind::from_id(52), None);
    assert_eq!(RecordKind::from_id(53), None);
}

#[test]
fn old_gate_opened_payload_keeps_tag_48_and_layout() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(3);
    let event = JournalEvent::GateOpenedEvent {
        run,
        seq: EventSeq::new(1),
        gate: gate(run),
        opened_at_ms: 100,
        requester: person("bob"),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::GateOpened,
        1,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (envelope, decoded) =
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    assert_eq!(envelope.record_kind, 48);
    assert_eq!(decoded, event);
    Ok(())
}

const BASE_OPENED_HEX: &str = "454a4256010030003c0000008200000001000000000000001d632757791d5a98ad440ab7dcc47fac1a4a8e993f1de485eb342d3ba5025ae09f4c760030030107030902000f6465706c6f792d617070726f7665720101010101010101010101010101010101010101010101010101010101010101010202020202020202020202020202020202020202020202020202020202020202e807030303030303030303030303030303030303030303030303030303030303030301016403626f62";

const BASE_DECISION_HEX: &str = "454a425601002c003c000000010100000200000000000000a7a07efeb33fee8701cacd2ce8e9c01ec73134fb1015fe0adf27315b7df019c73954ed622c03020703090100000105616c696365010f6465706c6f792d617070726f76657203626f620107636865636b65640101010101010101010101010101010101010101010101010101010101010101010202020202020202020202020202020202020202020202020202020202020202030303030303030303030303030303030303030303030303030303030303030300000000f4030107030005616c69636501010101010101010101010101010101010101010101010101010101010101010202020202020202020202020202020202020202020202020202020202020202f4030909090909090909090909090909090909090909090909090909090909090909";

fn decode_hex(value: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::with_capacity(value.len() / 2);
    let mut chars = value.as_bytes().chunks_exact(2);
    for pair in &mut chars {
        let text = std::str::from_utf8(pair)?;
        bytes.push(u8::from_str_radix(text, 16)?);
    }
    Ok(bytes)
}

#[test]
fn frozen_base_gate_opened_bytes_still_decode_and_reencode()
-> Result<(), Box<dyn std::error::Error>> {
    let bytes = decode_hex(BASE_OPENED_HEX)?;
    let (envelope, decoded) =
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    assert_eq!(envelope.record_kind, 48);
    let reencoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::GateOpened,
        1,
        &decoded,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(reencoded, bytes);
    Ok(())
}

#[test]
fn frozen_base_gate_decision_bytes_still_decode_and_reencode()
-> Result<(), Box<dyn std::error::Error>> {
    let bytes = decode_hex(BASE_DECISION_HEX)?;
    let (envelope, decoded) =
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    assert_eq!(envelope.record_kind, 44);
    let reencoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::GateDecision,
        2,
        &decoded,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(reencoded, bytes);
    Ok(())
}

#[test]
fn old_gate_decision_payload_keeps_tag_44_and_layout() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(3);
    let event = JournalEvent::GateDecisionEvent {
        run,
        seq: EventSeq::new(2),
        record: Box::new(record(run)),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::GateDecision,
        2,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (envelope, decoded) =
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    assert_eq!(envelope.record_kind, 44);
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn bound_gate_events_round_trip_under_tags_54_and_55() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(3);
    let opened = JournalEvent::GateOpenedBoundEvent {
        run,
        seq: EventSeq::new(3),
        gate: BoundManualGate {
            gate: gate(run),
            binding: binding(),
        },
        opened_at_ms: 100,
        requester: person("bob"),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::GateOpenedBound,
        3,
        &opened,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (envelope, decoded) =
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    assert_eq!(envelope.record_kind, 54);
    assert_eq!(decoded, opened);

    let decision = JournalEvent::GateDecisionBoundEvent {
        run,
        seq: EventSeq::new(4),
        record: Box::new(bound_record(run)),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::GateDecisionBound,
        4,
        &decision,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (envelope, decoded) =
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    assert_eq!(envelope.record_kind, 55);
    assert_eq!(decoded, decision);
    Ok(())
}
