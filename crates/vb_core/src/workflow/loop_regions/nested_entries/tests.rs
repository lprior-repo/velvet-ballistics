#![forbid(unsafe_code)]
//! These tests exercise the cold leaf through constructor-validated topology.
//! They do not prove the separate runtime entry gate or activation preflight.

#[path = "builders.rs"]
mod builders;
#[path = "support.rs"]
mod support;
#[path = "uncached.rs"]
mod uncached;

use builders::{install_pair, outer_parts};
use support::*;

#[test]
fn no_regions_produce_one_false_flag_per_node() -> TestResult {
    check_flags(linear_parts(4)?, &[])
}

#[test]
fn a_single_owner_does_not_block_its_own_body_or_endpoints() -> TestResult {
    check_flags(outer_parts(7, 5, 6)?, &[])
}

#[test]
fn nested_start_at_outer_body_and_all_cached_body_steps_are_flagged() -> TestResult {
    for family in FAMILIES {
        let mut parts = outer_parts(9, 7, 8)?;
        install_pair(&mut parts, family, 2, 3, 5, 6, 2)?;
        check_flags(parts, &[2, 3, 4, 5])?;
    }
    Ok(())
}

#[test]
fn nested_owner_gap_and_done_are_not_inferred_body() -> TestResult {
    let mut parts = outer_parts(10, 8, 9)?;
    install_pair(&mut parts, Family::Collect, 2, 4, 6, 7, 2)?;
    edit(&mut parts, 2)?.on_error = Some(step(3));
    let gap = edit(&mut parts, 3)?;
    gap.kind = CompiledNodeKind::Finish { result: slot(0) };
    gap.next = None;
    check_flags(parts, &[2, 4, 5, 6])
}

#[test]
fn sequential_start_at_prior_done_is_not_nested() -> TestResult {
    let mut parts = outer_parts(10, 4, 5)?;
    install_pair(&mut parts, Family::Collect, 5, 6, 8, 9, 2)?;
    check_flags(parts, &[])
}

#[test]
fn inner_done_that_is_another_nested_start_keeps_its_union_flag() -> TestResult {
    let mut parts = outer_parts(12, 10, 11)?;
    install_pair(&mut parts, Family::Collect, 2, 3, 4, 5, 2)?;
    install_pair(&mut parts, Family::Collect, 5, 6, 8, 9, 5)?;
    check_flags(parts, &[2, 3, 4, 5, 6, 7, 8])
}

#[test]
fn inner_done_that_is_enclosing_nested_control_keeps_its_union_flag() -> TestResult {
    let mut parts = outer_parts(11, 9, 10)?;
    install_pair(&mut parts, Family::Collect, 2, 3, 7, 8, 2)?;
    install_pair(&mut parts, Family::Collect, 3, 4, 6, 7, 5)?;
    check_flags(parts, &[2, 3, 4, 5, 6, 7])
}

#[test]
fn empty_unchecked_cache_does_not_invent_owned_body_flags() -> TestResult {
    let parts = outer_parts(7, 5, 6)?;
    let flags = super::derive(&parts, &[]).map_err(|error| format!("derive: {error:?}"))?;
    assert_eq!(flags.as_ref(), &[false; 7]);
    Ok(())
}
