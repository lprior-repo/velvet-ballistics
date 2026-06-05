#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::ids::{RunId, SlotIdx};
use vb_core::value::Taint;
use vb_ipc::IpcPayload;

fn arb_taint_option() -> impl Strategy<Value = Option<Taint>> {
    prop_oneof![
        Just(None),
        Just(Some(Taint::Clean)),
        Just(Some(Taint::DerivedFromSecret)),
        Just(Some(Taint::Secret)),
        Just(Some(Taint::Random)),
        Just(Some(Taint::TimeDependent)),
    ]
}

fn arb_slot_idx_with_boundaries() -> impl Strategy<Value = SlotIdx> {
    prop_oneof![
        Just(SlotIdx::ZERO),
        Just(SlotIdx::MAX),
        any::<u16>().prop_map(SlotIdx::new),
    ]
}

proptest! {
    #[test]
    fn vb_jpq7_21_answerask_payload_roundtrip_generated(
        run_raw in any::<u64>(),
        answer_slot in arb_slot_idx_with_boundaries(),
        answer in prop::collection::vec(any::<u8>(), 0..=65_536),
        taint in arb_taint_option(),
    ) {
        let run_id = RunId::new(run_raw);
        let payload = IpcPayload::AnswerAsk {
            run_id,
            answer_slot,
            answer: answer.clone(),
            taint,
        };

        let encoded = postcard::to_allocvec(&payload)
            .map_err(|err| TestCaseError::fail(format!("AnswerAsk postcard encode failed: {err}")))?;
        let decoded: IpcPayload = postcard::from_bytes(&encoded)
            .map_err(|err| TestCaseError::fail(format!("AnswerAsk postcard decode failed: {err}")))?;

        match decoded {
            IpcPayload::AnswerAsk {
                run_id: decoded_run_id,
                answer_slot: decoded_answer_slot,
                answer: decoded_answer,
                taint: decoded_taint,
            } => {
                prop_assert_eq!(decoded_run_id, run_id);
                prop_assert_eq!(decoded_answer_slot, answer_slot);
                prop_assert_eq!(decoded_answer, answer);
                prop_assert_eq!(decoded_taint, taint);
            }
            other => {
                prop_assert_eq!(other, payload);
            }
        }
    }
}
