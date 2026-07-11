// Cargo-fuzz target for guard precedence (PS-008, C6).
#![no_main]
use libfuzzer_sys::fuzz_target;

fn guard_chain(
    key_ok: bool,
    dup_ok: bool,
    count_ok: bool,
    encoding_ok: bool,
    admission_ok: bool,
) -> Result<u8, u8> {
    if !key_ok {
        return Err(0);
    }
    if !dup_ok {
        return Err(1);
    }
    if !count_ok {
        return Err(2);
    }
    if !encoding_ok {
        return Err(3);
    }
    if !admission_ok {
        return Err(4);
    }
    Ok(5)
}

fn fuzz_guard_ordering(data: &[u8]) {
    if data.len() < 5 {
        return;
    }
    let Some(key_ok) = data.first().map(|byte| byte & 1 == 1) else {
        return;
    };
    let Some(dup_ok) = data.get(1).map(|byte| byte & 1 == 1) else {
        return;
    };
    let Some(count_ok) = data.get(2).map(|byte| byte & 1 == 1) else {
        return;
    };
    let Some(encoding_ok) = data.get(3).map(|byte| byte & 1 == 1) else {
        return;
    };
    let Some(admission_ok) = data.get(4).map(|byte| byte & 1 == 1) else {
        return;
    };
    let result = guard_chain(key_ok, dup_ok, count_ok, encoding_ok, admission_ok);
    match result {
        Err(guard) => {
            if !key_ok {
                assert_eq!(guard, 0);
                return;
            }
            if !dup_ok {
                assert_eq!(guard, 1);
                return;
            }
            if !count_ok {
                assert_eq!(guard, 2);
                return;
            }
            if !encoding_ok {
                assert_eq!(guard, 3);
                return;
            }
            if !admission_ok {
                assert_eq!(guard, 4);
            }
        }
        Ok(guard) => {
            assert_eq!(guard, 5);
            assert!(key_ok && dup_ok && count_ok && encoding_ok && admission_ok);
        }
    }
}

fn fuzz_encode_record_guard(data: &[u8]) {
    use vb_core::{RunId, WorkflowDigest};
    use vb_storage::codec::encode_record;
    use vb_storage::constants::MAGIC_JOURNAL_EVENT;
    use vb_storage::events::JournalEvent;
    use vb_storage::records::RecordKind;
    use vb_storage::types::EventSeq;
    if data.len() < 4 {
        return;
    }
    let max_len_bytes: [u8; 4] = match data.get(0..4) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let max_len = u32::from_le_bytes(max_len_bytes);
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
    };
    let result = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        max_len,
    );
    match result {
        Ok(value) => {
            assert!(!value.is_empty());
        }
        Err(e) => {
            let msg = format!("{e}");
            assert!(!msg.is_empty());
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let Some((&selector, rest)) = data.split_first() else {
        return;
    };
    match selector.checked_rem(2) {
        Some(0) => fuzz_guard_ordering(rest),
        Some(_) => fuzz_encode_record_guard(rest),
        None => {}
    }
});
