#![forbid(unsafe_code)]
//! Bounded, structurally generated replay proofs through real constructors.

use super::{ReplayAction, ReplayError, step::replay_step};
use crate::errors::CoreError;
use crate::value_store::ValueStore;

#[path = "replay_kani_generators.rs"]
mod generators;
use generators::{ReplayShape, cover_shape, frame, one_branch, plan, setup_error, two_branches};

#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    #[kani::unwind(40)]
    fn verify_replay_choose_slot_two_branches_no_panic() {
        let (a, b, fallback): (bool, bool, bool) = kani::any();
        kani::assert(
            check_two_branches(a, b, fallback).is_ok(),
            "valid two-branch setup",
        );
    }

    #[kani::proof]
    #[kani::unwind(40)]
    fn verify_choose_slot_output_in_input_set() {
        let (a, b): (bool, bool) = kani::any();
        kani::assert(
            check_target_membership(a, b).is_ok(),
            "valid target-set setup",
        );
    }

    #[kani::proof]
    #[kani::unwind(40)]
    fn verify_replay_deterministic_for_same_input() {
        let value: bool = kani::any();
        kani::assert(
            check_determinism(value).is_ok(),
            "valid deterministic setup",
        );
    }
}

fn check_two_branches(a: bool, b: bool, fallback: bool) -> Result<(), CoreError> {
    let shape: ReplayShape = kani::any();
    let branches = two_branches(&shape);
    let plan = plan(&shape, &branches, fallback.then_some(shape.fallback))?;
    let mut run = frame(&shape, a, b)?;
    let node = plan.node(plan.entry()).ok_or_else(setup_error)?;
    let result = replay_step(node, &mut run, &mut ValueStore::new(), &plan);
    if !a && !b && !fallback {
        kani::assert(
            matches!(
                result,
                Err(ReplayError::Internal {
                    reason: "choose_slot no branch matched and no otherwise"
                })
            ),
            "no branch and no fallback fails",
        );
    } else {
        kani::assert(result.is_ok(), "a branch or fallback succeeds");
    }
    cover_cube(a, b, fallback);
    cover_shape(&shape);
    Ok(())
}

fn check_target_membership(a: bool, b: bool) -> Result<(), CoreError> {
    let shape: ReplayShape = kani::any();
    let branches = two_branches(&shape);
    let plan = plan(&shape, &branches, Some(shape.fallback))?;
    let mut run = frame(&shape, a, b)?;
    let node = plan.node(plan.entry()).ok_or_else(setup_error)?;
    let result = replay_step(node, &mut run, &mut ValueStore::new(), &plan);
    kani::assert(
        matches!(result, Ok(ReplayAction::Continue(target)) if
        branches.iter().any(|branch| branch.target == target) || target == shape.fallback),
        "selected target is configured",
    );
    cover_pair(a, b);
    cover_shape(&shape);
    Ok(())
}

fn check_determinism(value: bool) -> Result<(), CoreError> {
    let shape: ReplayShape = kani::any();
    let plan = plan(&shape, &one_branch(&shape), Some(shape.fallback))?;
    let node = plan.node(plan.entry()).ok_or_else(setup_error)?;
    let mut run_a = frame(&shape, value, false)?;
    let mut run_b = frame(&shape, value, false)?;
    let result_a = replay_step(node, &mut run_a, &mut ValueStore::new(), &plan);
    let result_b = replay_step(node, &mut run_b, &mut ValueStore::new(), &plan);
    kani::assert(
        matching_results(result_a, result_b),
        "matching replay outcomes",
    );
    kani::assert(run_a == run_b, "whole resulting frames are identical");
    kani::cover(value, "true input");
    kani::cover(!value, "false input");
    cover_shape(&shape);
    Ok(())
}

fn cover_cube(a: bool, b: bool, fallback: bool) {
    kani::cover(!a && !b && !fallback, "000");
    kani::cover(!a && !b && fallback, "001");
    kani::cover(!a && b && !fallback, "010");
    kani::cover(!a && b && fallback, "011");
    kani::cover(a && !b && !fallback, "100");
    kani::cover(a && !b && fallback, "101");
    kani::cover(a && b && !fallback, "110");
    kani::cover(a && b && fallback, "111");
}

fn cover_pair(a: bool, b: bool) {
    kani::cover(!a && !b, "00");
    kani::cover(!a && b, "01");
    kani::cover(a && !b, "10");
    kani::cover(a && b, "11");
}

fn matching_results(
    a: Result<ReplayAction, ReplayError>,
    b: Result<ReplayAction, ReplayError>,
) -> bool {
    match (a, b) {
        (Ok(ReplayAction::Continue(a)), Ok(ReplayAction::Continue(b))) => a == b,
        (Err(_), Err(_)) => true,
        _ => false,
    }
}
