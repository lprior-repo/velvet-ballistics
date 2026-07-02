//! PS-003 proptest: Invalid authority cannot mutate state (POB-vb-fzgdn-014)
//! Production binding: crates/vb_runtime/src/shard/types.rs PendingTimer::matches_authority
//!
//! Property: matches_authority rejects various combinations of wrong inputs.

use proptest::prelude::*;
use vb_runtime::shard::{PendingTimer, PendingTimerKind};

proptest! {
    #[test]
    fn ps_003_wrong_generation_always_rejected(
        gen in 1u64..u64::MAX,
        auth_gen in 1u64..u64::MAX,
    ) {
        prop_assume!(gen != auth_gen);
        let timer = PendingTimer {
            step: vb_core::ids::StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
            generation: gen,
            deadline: std::time::Instant::now(), ..Default::default()
        };
        prop_assert!(!timer.matches_authority(auth_gen, timer.deadline, PendingTimerKind::Wait),
            "generation {} should not match authority {}", gen, auth_gen);
    }

    #[test]
    fn ps_003_kind_must_match(
        gen in 1u64..u64::MAX,
    ) {
        let timer = PendingTimer {
            step: vb_core::ids::StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
            generation: gen,
            deadline: std::time::Instant::now(), ..Default::default()
        };
        prop_assert!(!timer.matches_authority(gen, timer.deadline, PendingTimerKind::Ask));
        // Symmetric: Ask timer won't match Wait authority
        let ask_timer = PendingTimer {
            step: vb_core::ids::StepIdx::ZERO,
            kind: PendingTimerKind::Ask,
            generation: gen,
            deadline: std::time::Instant::now(), ..Default::default()
        };
        prop_assert!(!ask_timer.matches_authority(gen, ask_timer.deadline, PendingTimerKind::Wait));
    }

    #[test]
    fn ps_003_exact_match_succeeds(
        gen in 1u64..1000,
        step in 0u16..50,
    ) {
        let timer = PendingTimer {
            step: vb_core::ids::StepIdx::new(step),
            kind: PendingTimerKind::Ask,
            generation: gen,
            deadline: std::time::Instant::now(), ..Default::default()
        };
        prop_assert!(timer.matches_authority(gen, timer.deadline, PendingTimerKind::Ask));
    }
}
