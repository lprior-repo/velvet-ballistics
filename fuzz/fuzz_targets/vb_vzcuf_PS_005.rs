// Cargo-fuzz target for encoded byte accounting (PS-005, C2).
#![no_main]
use libfuzzer_sys::fuzz_target;

fn record_header_len_usize() -> Option<usize> {
    usize::try_from(vb_storage::constants::RECORD_HEADER_LEN).ok()
}

fn fuzz_encoded_length_min(data: &[u8]) {
    use vb_core::{RunId, WorkflowDigest};
    use vb_storage::codec::encode_record;
    use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
    use vb_storage::events::JournalEvent;
    use vb_storage::records::RecordKind;
    use vb_storage::types::EventSeq;
    if data.len() < 8 {
        return;
    }
    let run_bytes: [u8; 8] = match data.get(0..8) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let run = u64::from_le_bytes(run_bytes);
    if run == 0 {
        return;
    }
    let event = JournalEvent::RunAccepted {
        run: RunId::new(run),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
    };
    let Some(header_len) = record_header_len_usize() else {
        return;
    };
    if let Ok(value) = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    ) {
        assert!(value.len() >= header_len);
        assert!(value.len() > header_len);
    }
}

fn fuzz_payload_vs_encoded(data: &[u8]) {
    use vb_core::{RunId, WorkflowDigest};
    use vb_storage::codec::encode_record;
    use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
    use vb_storage::events::JournalEvent;
    use vb_storage::records::RecordKind;
    use vb_storage::types::EventSeq;
    if data.len() < 8 {
        return;
    }
    let run_bytes: [u8; 8] = match data.get(0..8) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let run = u64::from_le_bytes(run_bytes);
    if run == 0 {
        return;
    }
    let event = JournalEvent::RunAccepted {
        run: RunId::new(run),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
    };
    let Some(header_len) = record_header_len_usize() else {
        return;
    };
    if let Ok(value) = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    ) && let Ok(payload_only) = postcard::to_allocvec(&event)
    {
        assert!(value.len() > payload_only.len());
        assert_eq!(
            value.len().checked_sub(payload_only.len()),
            Some(header_len)
        );
    }
}

fn fuzz_max_encoded_bounds(_data: &[u8]) {
    use vb_storage::constants::{MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_LEN};
    let Some(max_encoded) =
        u64::from(RECORD_HEADER_LEN).checked_add(u64::from(MAX_JOURNAL_EVENT_PAYLOAD_BYTES))
    else {
        return;
    };
    assert!(max_encoded < u64::MAX);
    assert_eq!(max_encoded, 1_048_636u64);
}

fuzz_target!(|data: &[u8]| {
    let Some((&selector, rest)) = data.split_first() else {
        return;
    };
    match selector.checked_rem(3) {
        Some(0) => fuzz_encoded_length_min(rest),
        Some(1) => fuzz_payload_vs_encoded(rest),
        Some(_) => fuzz_max_encoded_bounds(rest),
        None => {}
    }
});
