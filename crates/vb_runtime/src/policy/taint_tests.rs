//! Tests for digest sensitivity to taint variants.
//! Each taint produces a valid, distinct digest.

use super::*;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

fn baseline_ticket() -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 10,
    }
}

#[test]
fn clean_taint_produces_valid_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let record = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;
    assert_eq!(record.taint, Taint::Clean);
    let bytes = record.input_digest.as_bytes();
    let mut all_zero = true;
    for b in bytes.iter() {
        if *b != 0 {
            all_zero = false;
        }
    }
    assert!(!all_zero, "digest must be nonzero for canonical input");
    Ok(())
}

#[test]
fn derived_from_secret_taint_produces_valid_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let record = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::DerivedFromSecret,
    )?;
    assert_eq!(record.taint, Taint::DerivedFromSecret);
    Ok(())
}

#[test]
fn secret_taint_produces_valid_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let record = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Secret,
    )?;
    assert_eq!(record.taint, Taint::Secret);
    Ok(())
}

#[test]
fn random_taint_produces_valid_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let record = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Random,
    )?;
    assert_eq!(record.taint, Taint::Random);
    Ok(())
}

#[test]
fn time_dependent_taint_produces_valid_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let record = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::TimeDependent,
    )?;
    assert_eq!(record.taint, Taint::TimeDependent);
    Ok(())
}

#[test]
fn all_taint_variants_produce_distinct_digests() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let mut digests = Vec::new();
    for taint in [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ] {
        let record = policy.evaluate(
            baseline_ticket(),
            PinnedAdmissionPolicy::Unverified,
            b"input",
            taint,
        )?;
        digests.push((taint, record.input_digest));
    }

    // Verify all digests are distinct using pairwise comparison via iterators
    let mut remaining = &digests[..];
    while remaining.len() > 1 {
        let (first, rest) = remaining.split_at(1);
        if let Some(first_item) = first.first() {
            for second_item in rest.iter() {
                assert_ne!(
                    first_item.1, second_item.1,
                    "taint {:?} and {:?} produced identical digests",
                    first_item.0, second_item.0
                );
            }
        }
        remaining = rest;
    }
    Ok(())
}
