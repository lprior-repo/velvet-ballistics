use super::{encoded_record, sample_event};
use crate::codec::decode_journal_event;
use crate::constants::MAGIC_JOURNAL_EVENT;
use crate::records::RecordKind;

#[test]
fn sanity_valid_record_round_trips() {
    let bytes = encoded_record();
    let (_envelope, event) = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
        .expect("valid record must decode");
    assert_eq!(event, sample_event());
}

#[test]
fn sanity_run_accepted_wire_id_is_10() {
    assert_eq!(RecordKind::RunAccepted.id(), 10);
}
