//! Old golden export: generate valid ActionScheduledTicket golden records
//! using the actual old production serializer and prove the actual old
//! public decoder accepts them.
//!
//! This test constructs old JournalEvent::ActionScheduledTicket values
//! with the original seven-field ActionTicket and three-arg
//! compute_action_idempotency_key, serializes them via the old encode_record
//! with correct kind/magic/sequence and real CRC/BLAKE3, then feeds those
//! bytes to the old public parse_event. Decoded events must match originals
//! exactly. Valid envelopes are exported as binary golden fixtures.

use vb_core::{
    action::compute_action_idempotency_key, ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest,
};
use vb_storage::{
    codec::{decode_record, encode_record},
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    events::JournalEvent,
    journal::parse_event,
    types::EventSeq,
};
const FIXTURE_DIR: &str = "../../.beads/vb-79ffw/old-golden";
const OLD_SHA: &str = "423d8ae8b667d82b4d36e57d29d7ef2aede08bdf";

#[derive(Debug, Clone)]
struct CaseSpec {
    name: &'static str,
    run: u64,
    seq: u64,
    step: u16,
    action: u16,
    attempt: u16,
    capacity: u16,
    input_slot: u16,
    output_slot: u16,
}

fn make_event(spec: &CaseSpec) -> JournalEvent {
    let run = RunId::new(spec.run);
    let seq = EventSeq::new(spec.seq);
    let step = StepIdx::new(spec.step);
    let action = ActionId::new(spec.action);
    let idempotency_key =
        compute_action_idempotency_key(run, vb_core::SeqNo::new(spec.seq), action);
    let ticket = vb_core::action::issue_action_ticket(
        run,
        step,
        vb_core::SeqNo::new(spec.seq),
        action,
        spec.attempt,
        idempotency_key,
        spec.capacity,
    );
    JournalEvent::ActionScheduledTicket {
        run,
        seq,
        ticket,
        input: SlotIdx::new(spec.input_slot),
        output: SlotIdx::new(spec.output_slot),
        action_abi_digest: WorkflowDigest::from_bytes([0u8; 32]),
    }
}

fn test_case(spec: &CaseSpec) -> Result<Vec<u8>, String> {
    let event = make_event(spec);

    // Encode using old production serializer
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map_err(|e| format!("encode failed: {}", e))?;

    // Decode using old public decoder
    let decoded =
        parse_event(&bytes).map_err(|e| format!("parse_event failed: {}", e))?;

    // Assert exact decoded event equality
    if decoded != event {
        return Err(format!("decoded event != original event for case {}", spec.name));
    }

    // Also verify envelope kind matches
    let (envelope, _) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map_err(|e| format!("decode_record failed: {}", e))?;
    if envelope.record_kind != 34 {
        return Err(format!("envelope kind {} != 34 for case {}", envelope.record_kind, spec.name));
    }

    Ok(bytes)
}

#[test]
fn old_golden_export_all_cases() {
    let cases = vec![
        CaseSpec {
            name: "small-canonical",
            run: 1,
            seq: 1,
            step: 0,
            action: 0,
            attempt: 1,
            capacity: 3,
            input_slot: 0,
            output_slot: 1,
        },
        CaseSpec {
            name: "high-values",
            run: 65535,
            seq: 65535,
            step: 65535,
            action: 65535,
            attempt: 10,
            capacity: 15,
            input_slot: 100,
            output_slot: 200,
        },
        CaseSpec {
            name: "max-attempt-capacity",
            run: 123,
            seq: 456,
            step: 7,
            action: 42,
            attempt: 65535,
            capacity: 65535,
            input_slot: 1,
            output_slot: 2,
        },
    ];

    let mut errors = Vec::new();
    match std::fs::create_dir_all(FIXTURE_DIR) {
        Ok(()) => {}
        Err(e) => {
            errors.push(format!("failed to create fixture directory: {}", e));
        }
    }

    let mut manifest = Vec::new();
    manifest.push(format!("{{\n  \"oldSHA\": \"{}\",\n  \"cases\": [", OLD_SHA));

    let mut valid_count = 0u32;
    let mut valid_names = Vec::new();

    for (i, spec) in cases.iter().enumerate() {
        let result = test_case(spec);
        match result {
            Ok(bytes) => {
                let path = format!("{}/{}.bin", FIXTURE_DIR, spec.name);
                match std::fs::write(&path, &bytes) {
                    Ok(()) => {}
                    Err(e) => {
                        errors.push(format!("failed to write fixture {}: {}", spec.name, e));
                    }
                }
                let digest = blake3::hash(&bytes);
                valid_count += 1;
                valid_names.push(spec.name.to_string());
                let case_json = format!(
                    "
      {{
        \"name\": \"{}\",
        \"file\": \"{}.bin\",
        \"length\": {},
        \"blake3\": \"{}\",
        \"run\": {},
        \"seq\": {},
        \"step\": {},
        \"action\": {},
        \"attempt\": {},
        \"capacity\": {},
        \"input_slot\": {},
        \"output_slot\": {},
        \"record_kind\": 34,
        \"valid\": true
      }}",
                    spec.name,
                    spec.name,
                    bytes.len(),
                    digest.to_hex(),
                    spec.run,
                    spec.seq,
                    spec.step,
                    spec.action,
                    spec.attempt,
                    spec.capacity,
                    spec.input_slot,
                    spec.output_slot
                );
                manifest.push(case_json);
                eprintln!("[OK] {} -> {} bytes", spec.name, bytes.len());
            }
            Err(e) => {
                eprintln!("[FAIL] {}: {}", spec.name, e);
                errors.push(format!("{}: {}", spec.name, e));
                let case_json = format!(
                    "
      {{
        \"name\": \"{}\",
        \"error\": \"{}\",
        \"run\": {},
        \"seq\": {},
        \"step\": {},
        \"action\": {},
        \"attempt\": {},
        \"capacity\": {},
        \"valid\": false
      }}",
                    spec.name,
                    e,
                    spec.run,
                    spec.seq,
                    spec.step,
                    spec.action,
                    spec.attempt,
                    spec.capacity
                );
                manifest.push(case_json);
            }
        }
        if i < cases.len() - 1 {
            manifest.push(",".to_string());
        }
    }

    manifest.push("\n  ]\n}\n".to_string());
    let manifest_str = manifest.join("");
    match std::fs::write(
        format!("{}/manifest.json", FIXTURE_DIR),
        manifest_str,
    ) {
        Ok(()) => {
            eprintln!("[DONE] Manifest written to {}/manifest.json", FIXTURE_DIR);
        }
        Err(e) => {
            errors.push(format!("failed to write manifest: {}", e));
        }
    }

    // All three intended cases must be valid
    assert!(
        errors.is_empty(),
        "export test failed for case(s): {}",
        errors.join("; ")
    );
    assert_eq!(
        valid_count,
        3,
        "expected 3 valid exported cases, got {}",
        valid_count
    );
    assert!(
        valid_names.contains(&"small-canonical".to_string())
            && valid_names.contains(&"high-values".to_string())
            && valid_names.contains(&"max-attempt-capacity".to_string()),
        "missing expected valid case names: {:?}",
        valid_names
    );
}
