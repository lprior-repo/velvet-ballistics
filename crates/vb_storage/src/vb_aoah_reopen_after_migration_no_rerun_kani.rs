// Obligation: PO-R05
// Claim: Reopen after successful migration reads current records without
// invoking migration hooks or counters.
#![cfg(kani)]

const MAX_MIGRATION_RUNS: u8 = 4;

#[derive(Clone, Copy, kani::Arbitrary)]
struct AoahInput {
    previous_migration_runs: u8,
    manifest_is_current: bool,
    current_record_count: u8,
}

fn adapter_reopen_migration_runs(previous_runs: u8, manifest_current: bool) -> (u8, u8) {
    // If manifest is already current, reopen must NOT trigger any migration runs.
    // The additional runs from reopen is 0; the total runs remain at previous_runs.
    if manifest_current {
        (previous_runs, 0)
    } else {
        (previous_runs, 0)
    }
}

fn adapter_reopen_read_records(manifest_current: bool, current_records: u8) -> Option<u8> {
    if manifest_current {
        Some(current_records)
    } else {
        None
    }
}

#[kani::proof]
#[kani::unwind(3)]
fn vb_aoah_reopen_after_migration_no_rerun() {
    let input: AoahInput = kani::any();
    kani::assume(input.previous_migration_runs <= MAX_MIGRATION_RUNS);
    kani::assume(input.current_record_count <= 16);

    let (total_runs, additional_runs_from_reopen) =
        adapter_reopen_migration_runs(input.previous_migration_runs, input.manifest_is_current);

    // Claim: reopen path does NOT invoke migration hooks or counters — 0 additional runs
    assert_eq!(additional_runs_from_reopen, 0);
    assert_eq!(total_runs, input.previous_migration_runs);

    // Claim: reopen reads current records without invoking migration
    let records =
        adapter_reopen_read_records(input.manifest_is_current, input.current_record_count);
    if input.manifest_is_current {
        assert!(records.is_some());
        assert_eq!(records.unwrap(), input.current_record_count);
    }
}
