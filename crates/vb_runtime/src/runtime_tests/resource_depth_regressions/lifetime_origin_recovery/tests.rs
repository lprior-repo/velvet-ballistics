#[test]
fn cold_recovery_rejects_missing_budget_origin_before_publication() -> Result<(), String> {
    let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or("clock unavailable")?;
    let run = vb_core::RunId::new(920);
    let (directory, journal) = missing_origin_journal_for_run(run, now)?;
    let last_seq = 1;
    let raw_before = raw_run_rows(journal.as_ref(), run, last_seq)?;
    drop(journal);
    let reopened = Arc::new(
        vb_storage::FjallJournal::open(directory.path(), None)
            .map_err(|error| error.to_string())?,
    );
    let runtime = relaxed_runtime_for_journal(Arc::clone(&reopened))?;
    let pending = runtime.pending_boundary_snapshot(8);
    assert!(matches!(
        runtime.recover_product(run),
        Err(RuntimeError::RecoveryCannotResume { reason }) if reason == "budget_origin_missing"
    ));
    assert_eq!(runtime.pending_boundary_snapshot(8), pending);
    assert!(matches!(
        runtime.snapshot_run(run, 1),
        Ok(crate::shard::InspectResponse::NotFound { run: found, .. }) if found == run
    ));
    assert_eq!(raw_run_rows(reopened.as_ref(), run, last_seq)?, raw_before);
    assert!(matches!(
        runtime.recover_and_resume(run),
        Err(RuntimeError::RecoveryCannotResume { reason }) if reason == "budget_origin_missing"
    ));
    assert_eq!(runtime.pending_boundary_snapshot(8), pending);
    assert_eq!(raw_run_rows(reopened.as_ref(), run, last_seq)?, raw_before);
    Ok(())
}

#[test]
fn cold_recovery_rejects_duplicate_budget_origin_before_publication() -> Result<(), String> {
    assert_public_origin_refusal(
        OriginMutation::Duplicate,
        expected_origin_error(OriginMutation::Duplicate),
    )
}

#[test]
fn cold_recovery_rejects_late_budget_origin_before_publication() -> Result<(), String> {
    assert_public_origin_refusal(
        OriginMutation::Late,
        expected_origin_error(OriginMutation::Late),
    )
}

#[test]
fn cold_recovery_rejects_future_budget_origin_before_publication() -> Result<(), String> {
    assert_future_origin_refusal()
}

#[test]
fn cold_recovery_rejects_expired_budget_origin_before_publication() -> Result<(), String> {
    assert_expired_origin_refusal()
}

fn assert_future_origin_refusal() -> Result<(), String> {
    let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or("clock unavailable")?;
    let run = vb_core::RunId::new(921);
    let (directory, journal, _) = owned_journal_for_run(run, now)?;
    assert_public_origin_refusal_with_clock(directory, journal, run, now, "budget_origin_future")
}

fn assert_expired_origin_refusal() -> Result<(), String> {
    let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or("clock unavailable")?;
    let run = vb_core::RunId::new(922);
    let (directory, journal, _) = owned_journal_for_run(run, now)?;
    assert_public_origin_refusal_with_clock(directory, journal, run, now, "run_lifetime_expired")
}

fn assert_public_origin_refusal_with_clock(
    directory: tempfile::TempDir,
    journal: Arc<vb_storage::FjallJournal>,
    run: vb_core::RunId,
    now: u64,
    reason: &'static str,
) -> Result<(), String> {
    let last_seq = 3;
    let raw_before = raw_run_rows(journal.as_ref(), run, last_seq)?;
    drop(journal);
    let reopened = Arc::new(
        vb_storage::FjallJournal::open(directory.path(), None)
            .map_err(|error| error.to_string())?,
    );
    let _restore_clock = RestoreOriginClock;
    let recovery_clock = if reason == "budget_origin_future" {
        now.checked_sub(1).ok_or("clock underflow")?
    } else {
        now.checked_add(31 * 24 * 60 * 60 * 1_000)
            .ok_or("expired clock overflow")?
    };
    crate::shard::helpers::timer::install_wall_clock_millis_for_test(recovery_clock);
    let runtime = strict_runtime_for_owned_journal(Arc::clone(&reopened))?;
    let pending = runtime.pending_boundary_snapshot(8);
    assert!(matches!(
        runtime.recover_product(run),
        Err(RuntimeError::RecoveryCannotResume { reason: found }) if found == reason
    ));
    assert_eq!(runtime.pending_boundary_snapshot(8), pending);
    assert!(matches!(
        runtime.snapshot_run(run, 1),
        Ok(crate::shard::InspectResponse::NotFound { run: found, .. }) if found == run
    ));
    assert_eq!(raw_run_rows(reopened.as_ref(), run, last_seq)?, raw_before);
    assert!(matches!(
        runtime.recover_and_resume(run),
        Err(RuntimeError::RecoveryCannotResume { reason: found }) if found == reason
    ));
    assert_eq!(runtime.pending_boundary_snapshot(8), pending);
    assert_eq!(raw_run_rows(reopened.as_ref(), run, last_seq)?, raw_before);
    Ok(())
}
