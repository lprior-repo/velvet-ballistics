#![allow(
    unused_imports,
    dead_code,
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
use super::prelude::*;

#[test]
fn decode_record_returns_header_length_mismatch_for_wrong_len() {
    // Given an encoded record with header_len patched to 99
    // When decode_record is called
    // Then it returns HeaderLengthMismatch with found=99
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    let mut encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encoding should succeed");
    let len_bytes = 99u32.to_le_bytes();
    encoded[8] = len_bytes[0];
    encoded[9] = len_bytes[1];
    encoded[10] = len_bytes[2];
    encoded[11] = len_bytes[3];
    let header_prefix = &encoded[..56];
    let checksum = crc32c::crc32c(header_prefix);
    encoded[56] = (checksum & 0xFF) as u8;
    encoded[57] = ((checksum >> 8) & 0xFF) as u8;
    encoded[58] = ((checksum >> 16) & 0xFF) as u8;
    encoded[59] = ((checksum >> 24) & 0xFF) as u8;

    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    let Err(JournalError::HeaderLengthMismatch { found }) = result else {
        panic!("expected HeaderLengthMismatch, got {:?}", result);
    };
    assert_eq!(found, 99);
}

// --- Section 2: Key Function Behavior Tests ---

#[test]
fn run_event_key_produces_expected_key_bytes() {
    // Given run_id=1, seq=0
    // When run_event_key is called
    // Then the key is [0x11][1_be][0_be]
    let key = run_event_key(RunId::new(1), EventSeq::new(0));
    let key = key.expect("run_event_key should succeed");
    assert_eq!(key[0], 0x11);
    assert_eq!(key[1..9], 1u64.to_be_bytes());
    assert_eq!(key[9..17], 0u64.to_be_bytes());
}

#[test]
fn run_header_key_produces_expected_key_bytes() {
    // Given run_id=0xAABBCCDD_EEFF0011
    // When run_header_key is called
    // Then the key is [0x10][run_id_be]
    let run = RunId::new(0xAABB_CCDD_EEFF_0011);
    let key = run_header_key(run);
    let key = key.expect("run_header_key should succeed");
    assert_eq!(key[0], 0x10);
    assert_eq!(key[1..9], run.get().to_be_bytes());
}

#[test]
fn run_snapshot_key_produces_expected_key_bytes() {
    // Given run_id=5, seq=99
    // When run_snapshot_key is called
    // Then the key is [0x12][5_be][99_be]
    let key = run_snapshot_key(RunId::new(5), EventSeq::new(99));
    let key = key.expect("run_snapshot_key should succeed");
    assert_eq!(key[0], 0x12);
    assert_eq!(key[1..9], 5u64.to_be_bytes());
    assert_eq!(key[9..17], 99u64.to_be_bytes());
}

#[test]
fn workflow_source_key_produces_expected_key_bytes() {
    // Given a 32-byte digest of all 7s
    // When workflow_source_key is called
    // Then the key is [0x01][digest]
    let digest = [7u8; 32];
    let key = workflow_source_key(digest);
    let key = key.expect("workflow_source_key should succeed");
    assert_eq!(key[0], 0x01);
    assert_eq!(key[1..33], digest);
}

#[test]
fn compiled_ir_key_produces_expected_key_bytes() {
    // Given a 32-byte digest of all 2s
    // When compiled_ir_key is called
    // Then the key is [0x02][digest]
    let digest = [2u8; 32];
    let key = compiled_ir_key(digest);
    let key = key.expect("compiled_ir_key should succeed");
    assert_eq!(key[0], 0x02);
    assert_eq!(key[1..33], digest);
}

#[test]
fn index_action_key_produces_expected_key_bytes() {
    // Given action=100, run=200, step=300
    // When index_action_key is called
    // Then the key is [0x32][action_u16_be][run_u64_be][step_u16_be]
    let key = index_action_key(ActionId::new(100), RunId::new(200), StepIdx::new(300));
    let key = key.expect("index_action_key should succeed");
    assert_eq!(key[0], 0x32);
    assert_eq!(key[1..3], 100u16.to_be_bytes());
    assert_eq!(key[3..11], 200u64.to_be_bytes());
    assert_eq!(key[11..13], 300u16.to_be_bytes());
}

#[test]
fn index_status_key_produces_expected_key_bytes() {
    // Given state=5, timestamp=1000, run=50
    // When index_status_key is called
    // Then the key is [0x30][state_u8][timestamp_u64_be][run_u64_be]
    let key = index_status_key(IndexStatusState::Other(5), 1000, RunId::new(50));
    let key = key.expect("index_status_key should succeed");
    assert_eq!(key[0], 0x30);
    assert_eq!(key[1], 5);
    assert_eq!(key[2..10], 1000u64.to_be_bytes());
    assert_eq!(key[10..18], 50u64.to_be_bytes());
}

#[test]
fn index_workflow_key_produces_expected_key_bytes() {
    // Given workflow_id=42, run=99
    // When index_workflow_key is called
    // Then the key is [0x31][workflow_u32_be][run_u64_be]
    let key = index_workflow_key(WorkflowId::new(42), RunId::new(99));
    let key = key.expect("index_workflow_key should succeed");
    assert_eq!(key[0], 0x31);
    assert_eq!(key[1..5], 42u32.to_be_bytes());
    assert_eq!(key[5..13], 99u64.to_be_bytes());
}

#[test]
fn blob_key_produces_expected_key_bytes() {
    // Given a 32-byte digest of all 0xAB
    // When blob_key is called
    // Then the key is [0x20][digest]
    let digest = [0xAB; 32];
    let key = blob_key(digest);
    let key = key.expect("blob_key should succeed");
    assert_eq!(key[0], 0x20);
    assert_eq!(key[1..33], digest);
}

// --- Section 3: BDD Integration-Style Tests ---

#[test]
fn journal_opens_and_closes_without_error() {
    // Given a temporary directory
    // When FjallJournal::open is called
    // Then the journal opens successfully
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None);
    assert!(journal.is_ok(), "journal should open with default config");
}

#[test]
fn public_open_wrappers_create_declared_keyspaces() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");

    let journal = open_store(temp_dir.path());
    assert!(journal.is_ok(), "open_store should succeed");
    drop(journal);

    let reopened = init_keyspaces(temp_dir.path());
    assert!(reopened.is_ok(), "init_keyspaces should succeed");
    assert_eq!(FjallJournal::declared_keyspaces().len(), 10);
}
