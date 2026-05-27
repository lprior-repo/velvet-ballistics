#![forbid(unsafe_code)]
// Obligation: PO-vb-om21-key-parse-miri
use vb_core::RunId;
use vb_storage::{keys::run_event_key, types::EventSeq};
#[test]
fn vb_om21_key_parse_miri() {
    let key = match run_event_key(RunId::new(7), EventSeq::new(u64::MAX)) {
        Ok(key) => key,
        Err(err) => {
            assert!(false, "key encoding returned error: {:?}", err);
            return;
        }
    };
    let seq = u64::from_be_bytes([
        key[9], key[10], key[11], key[12], key[13], key[14], key[15], key[16],
    ]);
    assert_eq!(seq, u64::MAX);
}
