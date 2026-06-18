use proptest::prelude::*;
use vb_core::ids::{RunId, SlotIdx};
use vb_core::value::Taint;

proptest! {
    #[test]
    fn answerask_payload_roundtrip(run_val in 1..u64::MAX,
                                   slot_val in 0..=255u16,
                                   _value in prop::collection::vec(any::<i64>(), 0..=1024),
                                   taint in any::<Option<Taint>>()) {
        let run_id = RunId::new(run_val);
        let answer_slot = SlotIdx::new(slot_val);
        let answer_bytes: Vec<u8> = vec![0u8; 8];

        let payload = crate::IpcPayload::AnswerAsk {
            run_id,
            answer_slot,
            answer: answer_bytes,
            taint,
        };

        let encoded = postcard::to_allocvec(&payload).expect("AnswerAsk must serialize");
        let decoded: crate::IpcPayload = postcard::from_bytes(&encoded).expect("AnswerAsk must deserialize");

        match decoded {
            crate::IpcPayload::AnswerAsk { run_id: d_run, answer_slot: d_slot, ref d_answer, ref d_taint } => {
                assert_eq!(run_id, d_run, "run_id must survive round-trip");
                assert_eq!(answer_slot, d_slot, "answer_slot must survive round-trip");
                assert_eq!(payload.answer(), d_answer.as_slice(), "answer bytes must survive round-trip");
                assert_eq!(payload.taint(), d_taint, "taint must survive round-trip");
            }
            _ => panic!("decoded payload must be AnswerAsk variant"),
        }
    }

    #[test]
    fn no_legacy_ticket_field(run_val in 1..u64::MAX, slot_val in 0..=255u16, answer in prop::collection::vec(any::<u8>(), 0..=65536)) {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: RunId::new(run_val),
            answer_slot: SlotIdx::new(slot_val),
            answer,
            taint: None,
        };

        let encoded = postcard::to_allocvec(&payload).expect("must serialize");
        let decoded: crate::IpcPayload = postcard::from_bytes(&encoded).expect("must deserialize");

        match decoded {
            crate::IpcPayload::AnswerAsk { ref answer: a, .. } => {
                assert!(a.len() <= 65536, "answer bytes within limit");
            }
            _ => panic!("expected AnswerAsk"),
        }
    }

    #[test]
    fn taint_none_propagates(run_val in 1..u64::MAX, slot_val in 0..=255u16, answer in prop::collection::vec(any::<u8>(), 0..=65536)) {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: RunId::new(run_val),
            answer_slot: SlotIdx::new(slot_val),
            answer,
            taint: None,
        };

        assert_eq!(payload.taint(), None);

        let encoded = postcard::to_allocvec(&payload).expect("must serialize");
        let decoded: crate::IpcPayload = postcard::from_bytes(&encoded).expect("must deserialize");

        match decoded {
            crate::IpcPayload::AnswerAsk { ref taint, .. } => {
                assert_eq!(*taint, None, "None taint must survive round-trip");
            }
            _ => panic!("expected AnswerAsk"),
        }
    }

    #[test]
    fn taint_some_propagates(run_val in 1..u64::MAX, slot_val in 0..=255u16, answer in prop::collection::vec(any::<u8>(), 0..=65536)) {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: RunId::new(run_val),
            answer_slot: SlotIdx::new(slot_val),
            answer,
            taint: Some(Taint::Clean),
        };

        let encoded = postcard::to_allocvec(&payload).expect("must serialize");
        let decoded: crate::IpcPayload = postcard::from_bytes(&encoded).expect("must deserialize");

        match decoded {
            crate::IpcPayload::AnswerAsk { ref taint, .. } => {
                assert_eq!(*taint, Some(Taint::Clean), "Some(taint) must survive round-trip");
            }
            _ => panic!("expected AnswerAsk"),
        }
    }

    #[test]
    fn empty_answer_is_valid(run_val in 1..u64::MAX, slot_val in 0..=255u16) {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: RunId::new(run_val),
            answer_slot: SlotIdx::new(slot_val),
            answer: vec![],
            taint: None,
        };

        let encoded = postcard::to_allocvec(&payload).expect("empty answer must serialize");
        let decoded: crate::IpcPayload = postcard::from_bytes(&encoded).expect("empty answer must deserialize");

        match decoded {
            crate::IpcPayload::AnswerAsk { ref answer, .. } => {
                assert!(answer.is_empty(), "empty answer must survive round-trip");
            }
            _ => panic!("expected AnswerAsk"),
        }
    }

    #[test]
    fn max_answer_bytes_valid(run_val in 1..u64::MAX, slot_val in 0..=255u16) {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: RunId::new(run_val),
            answer_slot: SlotIdx::new(slot_val),
            answer: vec![0u8; 65536],
            taint: None,
        };

        let encoded = postcard::to_allocvec(&payload).expect("max-sized answer must serialize");
        let decoded: crate::IpcPayload = postcard::from_bytes(&encoded).expect("max-sized answer must deserialize");

        match decoded {
            crate::IpcPayload::AnswerAsk { ref answer, .. } => {
                assert_eq!(answer.len(), 65536, "max-sized answer must preserve length");
            }
            _ => panic!("expected AnswerAsk"),
        }
    }
}

impl crate::IpcPayload {
    fn answer(&self) -> &[u8] {
        match self {
            crate::IpcPayload::AnswerAsk { ref answer, .. } => answer,
            _ => &[0u8],
        }
    }

    fn taint(&self) -> &Option<Taint> {
        match self {
            crate::IpcPayload::AnswerAsk { ref taint, .. } => taint,
            _ => &None,
        }
    }
}
