#![forbid(unsafe_code)]

use std::error::Error;
use vb_core::{AggregateResourceBudget, RuntimePolicy, WorkflowDigest};
use vb_storage::{
    CURRENT_SCHEMA_VERSION, EventSeq, FjallJournal, JournalError, JournalEvent,
    MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_BYTES, RecordKind,
    RunInputBinding, RunPolicyPin, decode_journal_event, decode_record_header, encode_record,
    encode_record_header,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const OLD_AA: &[u8] =
    include_bytes!("fixtures/tag47_historical/old47_aa_unbound_unverified.payload");
const OLD_09: &[u8] =
    include_bytes!("fixtures/tag47_historical/old47_09_unbound_unverified.payload");
const OLD_DISTINCT: &[u8] =
    include_bytes!("fixtures/tag47_historical/old47_09_distinct_budget_01_28.payload");
const UNIT_BUDGET: AggregateResourceBudget = AggregateResourceBudget {
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
    max_cost_units: 1,
};
const DISTINCT_BUDGET: AggregateResourceBudget = AggregateResourceBudget {
    max_steps_executable: 1,
    max_action_tickets: 2,
    max_parallel_in_flight: 3,
    max_retries_per_action: 4,
    max_gather_pages: 5,
    max_gather_items: 6,
    max_for_each_iterations: 7,
    max_together_branches: 8,
    max_repeat_attempts: 9,
    max_run_time_seconds: 10,
    max_result_bytes: 11,
    max_total_slots_written: 12,
    max_timer_entries: 13,
    max_trace_events: 14,
    max_queue_depth: 15,
    max_journal_batch_bytes: 16,
    max_ipc_payload_bytes: 17,
    max_blob_bytes: 18,
    max_input_bytes: 19,
    max_step_budget_per_tick: 20,
    max_transitions_per_tick: 21,
    max_open_gates: 22,
    max_signal_subscriptions: 23,
    max_incident_emissions: 24,
    max_transitions: 25,
    max_journal_bytes_total: 26,
    max_value_store_entries: 27,
    max_cost_units: 28,
};

#[derive(Clone, Copy)]
struct Fixture {
    payload: &'static [u8],
    artifact_byte: u8,
    budget: &'static AggregateResourceBudget,
}

fn fixtures() -> [Fixture; 3] {
    [
        Fixture {
            payload: OLD_AA,
            artifact_byte: 0xaa,
            budget: &UNIT_BUDGET,
        },
        Fixture {
            payload: OLD_09,
            artifact_byte: 0x09,
            budget: &UNIT_BUDGET,
        },
        Fixture {
            payload: OLD_DISTINCT,
            artifact_byte: 0x09,
            budget: &DISTINCT_BUDGET,
        },
    ]
}

fn assert_fields(event: &JournalEvent, fixture: Fixture) -> TestResult {
    let JournalEvent::RunAdmissionBudget {
        run,
        seq,
        artifact_digest,
        input_digest,
        policy_pin,
        granted_capabilities,
        policy,
        budget,
        ..
    } = event
    else {
        return Err("historical tag 47 decoded as another event".into());
    };
    assert_eq!(run.get(), 1);
    assert_eq!(seq.get(), 0);
    assert_eq!(
        *artifact_digest,
        WorkflowDigest::from_bytes([fixture.artifact_byte; 32])
    );
    assert_eq!(*input_digest, RunInputBinding::LegacyUnbound);
    assert_eq!(*policy_pin, RunPolicyPin::Unverified);
    assert!(granted_capabilities.is_empty());
    assert_eq!(*policy, RuntimePolicy::Strict);
    assert_eq!(budget, fixture.budget);
    assert_eq!(event.record_kind().id(), 47);
    Ok(())
}

fn decode_literal(fixture: Fixture) -> TestResult<JournalEvent> {
    assert_eq!(fixture.payload.first(), Some(&47));
    let (event, remainder) = postcard::take_from_bytes::<JournalEvent>(fixture.payload)?;
    assert!(remainder.is_empty());
    assert_fields(&event, fixture)?;
    Ok(event)
}

fn literal_record(payload: &[u8]) -> Result<Vec<u8>, JournalError> {
    let header = encode_record_header(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAdmissionBudget,
        0,
        payload,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let mut record = header.to_vec();
    record.extend_from_slice(payload);
    Ok(record)
}

fn assert_record(record: &[u8], fixture: Fixture) -> TestResult<JournalEvent> {
    assert_eq!(RECORD_HEADER_BYTES, 60);
    assert_eq!(CURRENT_SCHEMA_VERSION, 1);
    let header =
        decode_record_header(record, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    assert_eq!(header.header_len, 60);
    assert_eq!(header.schema_version, 1);
    assert_eq!(header.record_kind, 47);
    assert_eq!(header.sequence, 0);
    assert_eq!(usize::try_from(header.payload_len)?, fixture.payload.len());
    assert_eq!(
        record
            .get(RECORD_HEADER_BYTES..)
            .ok_or("missing record payload")?,
        fixture.payload
    );
    let (envelope, event) =
        decode_journal_event(record, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    assert_eq!(envelope.schema_version, 1);
    assert_eq!(envelope.record_kind, 47);
    assert_eq!(envelope.sequence, 0);
    assert_fields(&event, fixture)?;
    Ok(event)
}

#[test]
fn historical_tag47_raw_payloads_preserve_exact_eight_field_values() -> TestResult {
    for fixture in fixtures() {
        decode_literal(fixture)?;
    }
    Ok(())
}

#[test]
fn current_tag47_writer_preserves_independent_historical_payload_bytes() -> TestResult {
    for fixture in fixtures() {
        let event = decode_literal(fixture)?;
        let record = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAdmissionBudget,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        assert_record(&record, fixture)?;
    }
    Ok(())
}

#[test]
fn historical_tag47_literal_record_survives_strict_close_reopen_full_replay() -> TestResult {
    for fixture in fixtures() {
        let temp = tempfile::tempdir()?;
        let expected_record = literal_record(fixture.payload)?;
        let event = assert_record(&expected_record, fixture)?;
        let mut journal = FjallJournal::open(temp.path(), None)?;
        journal.append_strict(&event)?;
        let stored = journal
            .get_event_bytes(event.run_id(), EventSeq::new(0))?
            .ok_or("strict append did not retain event bytes")?;
        assert_eq!(stored, expected_record);
        assert_record(&stored, fixture)?;
        let before = journal.events_for_run_full(event.run_id())?;
        assert_eq!(before.as_slice(), std::slice::from_ref(&event));
        journal.close()?;
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None)?;
        let stored_again = reopened
            .get_event_bytes(event.run_id(), EventSeq::new(0))?
            .ok_or("reopened journal lost the event")?;
        assert_eq!(stored_again, expected_record);
        assert_record(&stored_again, fixture)?;
        let after = reopened.events_for_run_full(event.run_id())?;
        assert_eq!(after, before);
        let mut tracker = vb_storage::ActionReplayTracker::new();
        let replayed = vb_storage::recovery::replay_events(&after, &mut tracker, &[])?;
        assert_eq!(replayed, after);
    }
    Ok(())
}

fn assert_payload_refused(payload: &[u8]) -> TestResult {
    let record = literal_record(payload)?;
    assert!(matches!(
        decode_journal_event(
            &record,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES
        ),
        Err(JournalError::PostcardDecodeFailed(_))
    ));
    Ok(())
}

#[test]
fn historical_tag47_truncated_and_trailing_payloads_fail_with_valid_envelopes() -> TestResult {
    let mut truncated = OLD_09.to_vec();
    let _removed = truncated.pop().ok_or("empty historical fixture")?;
    assert_payload_refused(&truncated)?;
    let mut trailing = OLD_09.to_vec();
    trailing.push(0x7f);
    assert_payload_refused(&trailing)?;
    Ok(())
}

#[test]
fn unambiguous_unversioned_nine_field_none_control_is_refused() -> TestResult {
    let prefix = OLD_09
        .get(..3)
        .ok_or("missing historical run/sequence prefix")?;
    let suffix = OLD_09
        .get(3..)
        .ok_or("missing historical artifact suffix")?;
    let mut nine_field_none = prefix.to_vec();
    nine_field_none.push(0);
    nine_field_none.extend_from_slice(suffix);
    assert_payload_refused(&nine_field_none)?;
    Ok(())
}
