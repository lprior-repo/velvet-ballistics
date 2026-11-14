#[derive(Clone, Copy)]
enum InvalidOrigin {
    Missing,
    ForeignRun,
    OutOfOrder,
}

fn alter_origin(
    events: &mut Vec<vb_storage::JournalEvent>,
    invalid: InvalidOrigin,
) -> Result<(), String> {
    match invalid {
        InvalidOrigin::Missing => {
            events.pop().ok_or("fixture lacks its final origin event")?;
        }
        InvalidOrigin::ForeignRun | InvalidOrigin::OutOfOrder => {
            let event = events
                .last_mut()
                .ok_or("fixture lacks its final origin event")?;
            match (invalid, event) {
                (
                    InvalidOrigin::ForeignRun,
                    vb_storage::JournalEvent::RunBudgetOriginEvent { run, .. },
                ) => *run = vb_core::RunId::new(999),
                (
                    InvalidOrigin::OutOfOrder,
                    vb_storage::JournalEvent::RunBudgetOriginEvent { seq, .. },
                ) => *seq = vb_storage::EventSeq::new(4),
                _ => return Err("fixture lacks its final budget-origin event".into()),
            }
        }
    }
    Ok(())
}

fn assert_invalid_error(result: Result<(), RuntimeError>, invalid: InvalidOrigin) {
    match invalid {
        InvalidOrigin::Missing => assert!(matches!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "owned_activation_budget_origin_missing"
            })
        )),
        InvalidOrigin::ForeignRun | InvalidOrigin::OutOfOrder => assert!(matches!(
            result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::OwnedInputs(
                    vb_storage::OwnedInputError::IdentityMismatch
                ))
        )),
    }
}

fn assert_invalid_prefix(run: vb_core::RunId, invalid: InvalidOrigin) -> Result<(), String> {
    let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or("clock unavailable")?;
    let (_directory, journal, _) = owned_journal_for_run(run, now)?;
    let stored = journal
        .events_for_run_full(run)
        .map_err(|error| error.to_string())?;
    let mut altered = stored.clone();
    alter_origin(&mut altered, invalid)?;
    let result =
        crate::shard::lifecycle::owned_activation::validate_pristine_prefix_events(run, &altered);
    assert_invalid_error(result, invalid);
    assert_eq!(
        journal
            .events_for_run_full(run)
            .map_err(|error| error.to_string())?,
        stored
    );
    Ok(())
}

#[test]
fn owned_origin_prefix_rejects_missing_origin() -> Result<(), String> {
    assert_invalid_prefix(vb_core::RunId::new(911), InvalidOrigin::Missing)
}

#[test]
fn owned_origin_prefix_rejects_foreign_run() -> Result<(), String> {
    assert_invalid_prefix(vb_core::RunId::new(912), InvalidOrigin::ForeignRun)
}

#[test]
fn owned_origin_prefix_rejects_out_of_order_origin() -> Result<(), String> {
    assert_invalid_prefix(vb_core::RunId::new(914), InvalidOrigin::OutOfOrder)
}

struct RestoreOriginClock;

impl Drop for RestoreOriginClock {
    fn drop(&mut self) {
        crate::shard::helpers::timer::restore_wall_clock_for_test();
    }
}

#[test]
fn recovery_lifetime_boundary_rejects_future_origin() -> Result<(), String> {
    let now = 1_000;
    let run = vb_core::RunId::new(916);
    let (_directory, journal, _) = owned_journal_for_run(run, now)?;
    let events = journal
        .events_for_run_full(run)
        .map_err(|error| error.to_string())?;
    let admission = crate::recovery::hydrate_run_admission_from_events(&events)
        .ok_or("fixture lacks budgeted admission")?;
    let _restore = RestoreOriginClock;
    crate::shard::helpers::timer::install_wall_clock_millis_for_test(now);
    assert!(matches!(
        crate::recovery::validate_recovered_lifetime_at_wall_clock(
            Some(&admission),
            now.checked_add(1),
        ),
        Err(RuntimeError::RecoveryCannotResume { reason })
            if reason == "budget_origin_future"
    ));
    Ok(())
}

#[test]
fn recovery_lifetime_boundary_rejects_missing_origin_for_budgeted_admission() -> Result<(), String>
{
    let now = 1_000;
    let run = vb_core::RunId::new(917);
    let (_directory, journal, _) = owned_journal_for_run(run, now)?;
    let events = journal
        .events_for_run_full(run)
        .map_err(|error| error.to_string())?;
    let admission = crate::recovery::hydrate_run_admission_from_events(&events)
        .ok_or("fixture lacks budgeted admission")?;
    assert!(matches!(
        crate::recovery::validate_recovered_lifetime(Some(&admission), None, now),
        Err(RuntimeError::RecoveryCannotResume { reason })
            if reason == "budget_origin_missing"
    ));
    Ok(())
}
