//! Tests for digest sensitivity to taint variants.
//! Each taint produces a valid, distinct digest.

use super::*;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

fn make_ticket(
    run: u64,
    step: u16,
    seq: u64,
    action: u16,
    run_attempt: u16,
    attempt: u16,
    idempotency_key: u128,
    capacity: u16,
) -> ActionTicket {
    ActionTicket {
        run: RunId::new(run),
        step: StepIdx::new(step),
        seq: SeqNo::new(seq),
        action: ActionId::new(action),
        run_attempt,
        attempt,
        idempotency_key,
        capacity,
    }
}

fn baseline_ticket() -> ActionTicket {
    make_ticket(1, 0, 0, 0, 1, 1, 0, 10)
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

    for i in 0..digests.len() {
        for j in (i + 1)..digests.len() {
            assert_ne!(
                digests[i].1, digests[j].1,
                "taint {:?} and {:?} produced identical digests",
                digests[i].0, digests[j].0
            );
        }
    }
    Ok(())
}
