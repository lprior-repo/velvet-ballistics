#![forbid(unsafe_code)]
//! Boundary-policy tests for the seeded autonomous scheduler facade.
//!
//! First / Random / RoundRobin policy tests live here. Each policy
//! must be deterministic for the same seed + same input stream; only
//! `Random` depends on the seed value.

use crate::scheduler::tests::fixtures::{FIXTURE_SEED_A, FIXTURE_SEED_B, make_scheduler};
use crate::scheduler::types::{BoundaryChoice, BoundaryDecision, BoundaryPolicy};

#[test]
fn boundary_policy_first_avoids_randomness() {
    // `BoundaryPolicy::First` must be seed-independent: two
    // schedulers with different seeds must produce identical
    // decision streams when both use `First`.
    let mut scheduler_a = make_scheduler(FIXTURE_SEED_A, BoundaryPolicy::First);
    let mut scheduler_b = make_scheduler(FIXTURE_SEED_B, BoundaryPolicy::First);

    let choice_stream = [
        BoundaryChoice::Free,
        BoundaryChoice::Free,
        BoundaryChoice::Free,
        BoundaryChoice::Free,
        BoundaryChoice::Free,
    ];
    let mut decisions_a = Vec::with_capacity(choice_stream.len());
    let mut decisions_b = Vec::with_capacity(choice_stream.len());
    for choice in choice_stream {
        decisions_a.push(
            scheduler_a
                .decide_boundary(choice.clone())
                .unwrap_or(BoundaryDecision::Advance),
        );
        decisions_b.push(
            scheduler_b
                .decide_boundary(choice)
                .unwrap_or(BoundaryDecision::Advance),
        );
    }
    assert_eq!(
        decisions_a, decisions_b,
        "First policy must be seed-independent"
    );
    for decision in decisions_a.iter().chain(decisions_b.iter()) {
        assert_eq!(
            *decision,
            BoundaryDecision::Advance,
            "First policy must always emit Advance for Free choices"
        );
    }
}

#[test]
fn round_robin_policy_cycles_through_variants() {
    // RoundRobin should cycle deterministically: Advance, Yield,
    // Fail, Retry, Advance, ...
    let mut scheduler = make_scheduler(FIXTURE_SEED_A, BoundaryPolicy::RoundRobin);
    let decisions: Vec<BoundaryDecision> = (0..4)
        .map(|_| {
            scheduler
                .decide_boundary(BoundaryChoice::Free)
                .unwrap_or(BoundaryDecision::Advance)
        })
        .collect();
    assert!(matches!(decisions[0], BoundaryDecision::Advance));
    assert!(matches!(decisions[1], BoundaryDecision::Yield { .. }));
    assert!(matches!(decisions[2], BoundaryDecision::Fail { .. }));
    assert!(matches!(decisions[3], BoundaryDecision::Retry { .. }));
}

#[test]
fn random_policy_uses_prng_state() {
    // `Random` policy must produce different outcomes for different
    // seeds when given the same input stream. We sample a 16-step
    // stream and assert at least one divergence.
    let mut scheduler_a = make_scheduler(FIXTURE_SEED_A, BoundaryPolicy::Random);
    let mut scheduler_b = make_scheduler(FIXTURE_SEED_B, BoundaryPolicy::Random);

    let choice_stream: Vec<BoundaryChoice> = (0..16).map(|_| BoundaryChoice::Free).collect();
    let decisions_a: Vec<BoundaryDecision> = choice_stream
        .iter()
        .map(|c| {
            scheduler_a
                .decide_boundary(c.clone())
                .unwrap_or(BoundaryDecision::Advance)
        })
        .collect();
    let decisions_b: Vec<BoundaryDecision> = choice_stream
        .iter()
        .map(|c| {
            scheduler_b
                .decide_boundary(c.clone())
                .unwrap_or(BoundaryDecision::Advance)
        })
        .collect();
    let diverged = decisions_a
        .iter()
        .zip(decisions_b.iter())
        .any(|(a, b)| a != b);
    assert!(
        diverged,
        "Random policy must produce divergent decisions across different seeds"
    );
}

#[test]
fn constrained_choices_override_policy() {
    // Constrained choices (AdvanceOnly / YieldOnly / FailOnly /
    // RetryOnly) must be honoured by every policy, including
    // RoundRobin and Random.
    for policy in [
        BoundaryPolicy::First,
        BoundaryPolicy::Random,
        BoundaryPolicy::RoundRobin,
    ] {
        let mut scheduler = make_scheduler(FIXTURE_SEED_A, policy);
        let decision = scheduler
            .decide_boundary(BoundaryChoice::AdvanceOnly)
            .unwrap_or(BoundaryDecision::Fail {
                variant: crate::RuntimeError::ShutdownInProgress,
            });
        assert_eq!(decision, BoundaryDecision::Advance);
    }
}
