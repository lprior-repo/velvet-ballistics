#![forbid(unsafe_code)]

#[path = "shipping_budget_origin_recovery/support.rs"]
mod support;

use support::{OriginMutation, cold_recover, fixture, rewrite_origin};
use vb_core::RunId;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn public_cold_recovery_accepts_pristine_budget_origin() -> TestResult {
    let run = RunId::new(8_900);
    let case = fixture(run)?;
    let (runtime, journal) = cold_recover(&case)?;
    let before = case.raw_rows(&journal)?;
    let product = runtime.recover_product(run)?;
    assert!(
        matches!(
            product,
            vb_runtime::recovery::RuntimeRecoveryProduct::Resumable(_)
        ),
        "pristine budget origin should produce resumable recovery: {product:?}"
    );
    assert!(matches!(
        runtime.snapshot_run(run, 1),
        Ok(vb_runtime::shard::InspectResponse::NotFound { run: found, .. }) if found == run
    ));
    assert_eq!(case.raw_rows(&journal)?, before);
    Ok(())
}

#[test]
fn public_cold_recovery_rejects_well_encoded_foreign_origin_without_mutation() -> TestResult {
    assert_public_origin_rejection(OriginMutation::Foreign)
}

#[test]
fn public_cold_recovery_rejects_well_encoded_out_of_order_origin_without_mutation() -> TestResult {
    assert_public_origin_rejection(OriginMutation::OutOfOrder)
}

fn assert_public_origin_rejection(mutation: OriginMutation) -> TestResult {
    let run = RunId::new(8_901);
    let case = fixture(run)?;
    let changed = rewrite_origin(&case, mutation)?;
    assert_eq!(
        vb_storage::journal::parse_event(&changed)?,
        mutation.event(run, case.origin_seq, case.started_millis)?
    );
    let before_journal = vb_storage::FjallJournal::open(case.directory.path(), None)?;
    let before = case.raw_rows(&before_journal)?;
    drop(before_journal);
    let (runtime, journal) = cold_recover(&case)?;
    let pending = runtime.pending_boundary_snapshot(8);
    let expected = mutation.expected_error(run, case.origin_seq);

    assert!(matches!(
        runtime.recover_product(run),
        Err(vb_runtime::RuntimeError::Recovery { error }) if error == expected
    ));
    assert_eq!(runtime.pending_boundary_snapshot(8), pending);
    assert!(matches!(
        runtime.snapshot_run(run, 1),
        Ok(vb_runtime::shard::InspectResponse::NotFound { run: found, .. }) if found == run
    ));
    assert_eq!(case.raw_rows(&journal)?, before);

    assert!(matches!(
        runtime.recover_and_resume(run),
        Err(vb_runtime::RuntimeError::Recovery { error }) if error == expected
    ));
    assert_eq!(runtime.pending_boundary_snapshot(8), pending);
    assert!(matches!(
        runtime.snapshot_run(run, 1),
        Ok(vb_runtime::shard::InspectResponse::NotFound { run: found, .. }) if found == run
    ));
    assert_eq!(case.raw_rows(&journal)?, before);
    Ok(())
}
