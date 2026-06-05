use super::common::*;

/// Integration test: SlotWrittenEvent with value=Some(bytes) roundtrips correctly.
#[test]
fn slot_written_event_with_value_some_bytes_roundtrips() {
    let original_value = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(77),
        slot: SlotIdx::new(3),
        value: Some(original_value.clone()),
        extra: None,
        attempt: 1,
    };

    let bytes = encode_journal_event_record(&event).expect("valid event should encode");
    let parsed = parse_event(&bytes).expect("canonical encoding must parse successfully");

    match parsed {
        JournalEvent::SlotWrittenEvent {
            value: Some(decoded_value),
            ..
        } => {
            assert_eq!(
                decoded_value, original_value,
                "value must survive roundtrip"
            );
        }
        other => panic!("expected SlotWrittenEvent with value, got {:?}", other),
    }
}

/// Integration test: SlotWrittenEvent with large value bytes roundtrips correctly.
#[test]
fn slot_written_event_with_large_value_roundtrips() {
    let large_value = vec![0xAB_u8; 1024];
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(88),
        slot: SlotIdx::new(u16::MAX.into()),
        value: Some(large_value.clone()),
        extra: None,
        attempt: u16::MAX,
    };

    let bytes = encode_journal_event_record(&event).expect("valid event should encode");
    let parsed = parse_event(&bytes).expect("canonical encoding must parse successfully");

    match parsed {
        JournalEvent::SlotWrittenEvent {
            value: Some(decoded_value),
            ..
        } => {
            assert_eq!(
                decoded_value.len(),
                large_value.len(),
                "large value length must be preserved"
            );
            assert_eq!(
                decoded_value, large_value,
                "large value content must be preserved"
            );
        }
        other => panic!(
            "expected SlotWrittenEvent with large value, got {:?}",
            other
        ),
    }
}
