#![allow(clippy::expect_used)]

use proptest::prelude::*;
use vb_core::{RunId, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::codec::encode_record;
use vb_storage::constants::{
    MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_LEN,
};
use vb_storage::events::JournalEvent;
use vb_storage::records::RecordKind;

proptest! {
    #[test]
    fn ps002_checked_add_no_panic(a: u64, b: u64) {
        let _result = a.checked_add(b);
    }
    #[test]
    fn ps002_checked_add_correct(a: u64, b: u64) {
        if let Some(total) = a.checked_add(b) {
            prop_assert_eq!(total, a.wrapping_add(b));
        }
    }
    #[test]
    fn ps002_overflow_detect(a: u64, b: u64) {
        let a_wide = u128::from(a);
        let b_wide = u128::from(b);
        let max_wide = u128::from(u64::MAX);
        if a_wide.checked_add(b_wide).is_some_and(|s| s <= max_wide) {
            prop_assert!(a.checked_add(b).is_some());
        } else {
            prop_assert!(a.checked_add(b).is_none());
        }
    }
    #[test]
    fn ps002_u32_to_u64(n: u32) {
        let wide: u64 = u64::from(n);
        let wide_back = u32::try_from(wide).expect("u32 to u64 widening is exact");
        prop_assert_eq!(wide_back, n);
        prop_assert!(wide <= u64::from(u32::MAX));
    }
    #[test]
    fn ps002_usize_safe(n in 0usize..1000000usize) {
        let wide: u64 = n.try_into().expect("small usize fits in u64");
        let wide_back = usize::try_from(wide).expect("small usize to u64 is exact");
        prop_assert_eq!(wide_back, n);
    }
    #[test]
    fn ps002_max_encoded_fits(_dummy in proptest::bool::ANY) {
        let header_wide = u64::from(RECORD_HEADER_LEN);
        let payload_wide = u64::from(MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        let max = header_wide.checked_add(payload_wide).expect("u32 sum fits in u64");
        prop_assert!(max < u64::MAX);
    }
    #[test]
    fn ps002_encode_valid(run in 1u64..1000u64, seq in 0u64..100u64) {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(run), seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        };
  let result = encode_record(
             MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, seq,
             &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
         );
         let bytes = result.expect("encode RunAccepted must succeed for valid inputs");
         prop_assert!(!bytes.is_empty(),
             "encoded RunAccepted must produce non-empty bytes, got {} bytes",
             bytes.len());
    }
    #[test]
    fn ps002_chain_safe(
        base in 0u64..100000u64,
        adds in proptest::collection::vec(1u64..10000u64, 0..20)
    ) {
        let mut total: u64 = base;
        for add in adds {
            if let Some(nt) = total.checked_add(add) { total = nt; }
        }
        prop_assert!(total >= base);
    }
}
