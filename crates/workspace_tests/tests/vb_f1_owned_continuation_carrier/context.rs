use super::support::*;
use serde::Serialize;
use vb_storage::{JournalError, JournalEvent, RecordKind, decode_record, encode_record};

struct MustNotTraverse;
impl Serialize for MustNotTraverse {
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("payload traversed"))
    }
}

#[test]
fn direct_serde_and_generic_codec_cannot_bypass_context() -> TestResult {
    let carrier = event(2)?;
    assert_eq!(carrier.attempt(), Some(2));
    assert!(postcard::to_allocvec(&carrier).is_err());
    assert!(postcard::from_bytes::<JournalEvent>(&[43]).is_err());
    assert!(matches!(
        encode_record(
            vb_storage::MAGIC_JOURNAL_EVENT,
            RecordKind::RunContinuation,
            2,
            &MustNotTraverse,
            1_048_576
        ),
        Err(JournalError::InvalidEvent)
    ));
    // A correct header but wrong full payload digest must still return context
    // refusal, demonstrating generic43 is stopped BEFORE payload hash traversal.
    let mut raw = envelope(&[43, 1, 17, 2], 2)?;
    *raw.last_mut().ok_or("payload")? ^= 1;
    assert!(matches!(
        decode_record::<JournalEvent>(&raw, vb_storage::MAGIC_JOURNAL_EVENT, 1_048_576),
        Err(JournalError::InvalidEvent)
    ));
    // Only tag43 is read by direct Deserialize; it never needs an opaque length.
    let mut huge = vec![43];
    huge.extend_from_slice(&[255; 128]);
    assert!(postcard::from_bytes::<JournalEvent>(&huge).is_err());
    Ok(())
}

#[test]
fn owning_event_clone_shares_opaque_storage_and_keeps_attempt_two() -> TestResult {
    let carrier = event(2)?;
    let copy = carrier.clone();
    assert_eq!(copy, carrier);
    let (
        JournalEvent::RunContinuation { carrier: first },
        JournalEvent::RunContinuation { carrier: second },
    ) = (&carrier, &copy)
    else {
        return Err("wrong event".into());
    };
    assert!(std::ptr::eq(
        first.component().as_ptr(),
        second.component().as_ptr()
    ));
    assert_eq!(first.header().actual_attempt(), 2);
    assert_eq!(carrier.attempt(), Some(2));
    assert!(carrier.is_valid());
    Ok(())
}
