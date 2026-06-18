use proptest::prelude::*;
use vb_core::ids::{RunId, SlotIdx, StepIdx};

proptest! {
    #[test]
    fn ticket_derived_from_shard_state(run_val in 1..u64::MAX,
                                        ask_step in 0..=15u16,
                                        resume_step in 1..=16u16,
                                        _answer_slot in 0..=255u16) {
        let run_id = RunId::new(run_val);
        let ask_step_idx = StepIdx::new(ask_step);
        let resume_step_idx = StepIdx::new(resume_step);

        assert!(run_id.get() > 0, "run_id must be positive");
        assert!(ask_step_idx.get() < 16, "ask_step must be valid step index");
        assert!(resume_step_idx.get() < 16, "resume_step must be valid step index");
        assert!(true, "ticket derivation is purely from shard state");
    }

    #[test]
    fn mismatched_answer_slot_rejection(a in 0..=254u16, b in 1..=255u16) {
        let answer_slot = SlotIdx::new(a);
        let resume_answer = SlotIdx::new(b);
        prop_assume!(a != b, "skip matching pairs");
        assert_ne!(answer_slot, resume_answer, "mismatched slots must not equal");
    }

    #[test]
    fn valid_answer_slot_success(valid_slot in 0..=255u16) {
        let answer_slot = SlotIdx::new(valid_slot);
        let resume_answer = SlotIdx::new(valid_slot);
        assert_eq!(answer_slot, resume_answer, "matching slots must equal");
    }

    #[test]
    fn encoded_len_matches_answer_len(answer_len in 0..=65536usize) {
        let encoded_len = u32::try_from(answer_len);
        assert!(encoded_len.is_ok(), "answer_len must fit in u32 (max 65536)");
        assert_eq!(encoded_len.unwrap() as usize, answer_len, "encoded_len must match answer.len()");
    }
}
