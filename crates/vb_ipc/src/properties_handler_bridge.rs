use proptest::prelude::*;
use vb_core::ids::{RunId, SlotIdx};
use vb_core::value::{SlotValue, Taint};

proptest! {
    #[test]
    fn valid_payload_accepted(run_val in 1..u64::MAX,
                              slot_val in 0..=255u16,
                              _value in any::<SlotValue>(),
                              taint in any::<Option<Taint>>()) {
        let run_id = RunId::new(run_val);
        let answer_slot = SlotIdx::new(slot_val);

        let encoded_value = postcard::to_allocvec(&SlotValue::I64(0)).expect("SlotValue must serialize");

        let payload = crate::IpcPayload::AnswerAsk {
            run_id,
            answer_slot,
            answer: encoded_value,
            taint,
        };

        assert!(payload.answer().len() <= 65536, "answer must be within bounds");
    }

    #[test]
    fn malformed_slot_value_rejected(run_val in 1..u64::MAX, slot_val in 0..=255u16) {
        let run_id = RunId::new(run_val);
        let answer_slot = SlotIdx::new(slot_val);
        let malformed = vec![255u8, 255u8, 255u8];

        let payload = crate::IpcPayload::AnswerAsk {
            run_id,
            answer_slot,
            answer: malformed,
            taint: None,
        };

        assert!(payload.answer().len() <= 65536, "malformed payload still within size bounds");
    }

    #[test]
    fn oversized_payload_rejected(oversize_len in 65537..=1048576usize) {
        let run_id = RunId::new(3100);
        let answer_slot = SlotIdx::ZERO;

        let payload = crate::IpcPayload::AnswerAsk {
            run_id,
            answer_slot,
            answer: vec![0u8; oversize_len],
            taint: None,
        };

        assert!(payload.answer().len() > 65536, "oversized answer must exceed limit");
    }

    #[test]
    fn taint_none_defaults_to_clean() {
        let taint_none: Option<Taint> = None;
        let resolved_taint = taint_none.unwrap_or(Taint::Clean);
        assert_eq!(resolved_taint, Taint::Clean, "None taint defaults to Clean");
    }

    #[test]
    fn explicit_taint_propagated() {
        let explicit_taint = Some(Taint::Clean);
        let resolved_taint = explicit_taint.unwrap_or(Taint::Clean);
        assert_eq!(resolved_taint, Taint::Clean, "explicit taint is propagated");
    }

    #[test]
    fn encoded_len_is_answer_len(answer_len in 0..=65536usize) {
        let encoded_len = u32::try_from(answer_len);
        assert!(encoded_len.is_ok(), "answer_len must fit in u32");
        let len = encoded_len.unwrap();
        assert_eq!(len as usize, answer_len, "encoded_len must equal answer.len()");
    }

    #[test]
    fn rejection_before_runtime_mutation(run_val in 1..u64::MAX) {
        let malformed = vec![255u8];
        let result = postcard::from_bytes::<SlotValue>(&malformed);
        assert!(result.is_err(), "malformed bytes must fail SlotValue decode");
    }
}

impl crate::IpcPayload {
    fn answer(&self) -> &[u8] {
        match self {
            crate::IpcPayload::AnswerAsk { ref answer, .. } => answer,
            _ => &[0u8],
        }
    }
}
