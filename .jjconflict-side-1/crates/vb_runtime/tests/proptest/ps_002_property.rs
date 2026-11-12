//! PS-002 proptest: PendingTimer matches_authority (POB-vb-fzgdn-009)
//! Production binding: crates/vb_runtime/src/shard/types.rs PendingTimer::matches_authority
//!
//! Property: matches_authority returns true iff all fields match exactly.

use proptest::prelude::*;
use std::time::Instant;
use vb_core::ids::StepIdx;
use vb_runtime::shard::{PendingTimer, PendingTimerKind};

proptest! {
    #[test]
    fn ps_002_matches_exact_authority(
        gen in 1u64..1000,
        step in 0u16..100,
        wait in proptest::bool::ANY,
    ) {
        let kind = if wait { PendingTimerKind::Wait } else { PendingTimerKind::Ask };
        let timer = PendingTimer {
            step: StepIdx::new(step),
            kind,
            generation: gen,
            deadline: Instant::now(),
        };
        prop_assert!(timer.matches_authority(gen, timer.deadline, kind));
    }

    #[test]
    fn ps_002_rejects_wrong_generation(
        gen in 1u64..1000,
        wrong_gen in 1u64..1000,
    ) {
        prop_assume!(gen != wrong_gen);
        let timer = PendingTimer {
            step: StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
            generation: gen,
            deadline: Instant::now(),
        };
        prop_assert!(!timer.matches_authority(wrong_gen, timer.deadline, PendingTimerKind::Wait));
    }

    #[test]
    fn ps_002_rejects_wrong_kind(
        gen in 1u64..1000,
    ) {
        let timer = PendingTimer {
            step: StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
            generation: gen,
            deadline: Instant::now(),
        };
        prop_assert!(!timer.matches_authority(gen, timer.deadline, PendingTimerKind::Ask));
    }

    #[test]
    fn ps_002_struct_fields_preserved(
        gen in 1u64..u64::MAX,
        step in 0u16..100,
    ) {
        let kind = PendingTimerKind::Wait;
        let deadline = Instant::now();
        let timer = PendingTimer { step: StepIdx::new(step), kind, generation: gen, deadline };
        prop_assert_eq!(timer.step, StepIdx::new(step));
        prop_assert_eq!(timer.generation, gen);
        prop_assert_eq!(timer.kind, kind);
        prop_assert_eq!(timer.deadline, deadline);
    }
}
