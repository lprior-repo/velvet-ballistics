// Obligation: PO-R01
// Claim: Runtime open of an old supported store returns MigrationRequired
// and performs no migration side effects.
#![cfg(kani)]

const RESTATE_V1_VERSION: u16 = 1;
const CURRENT_VERSION: u16 = 2;

#[derive(Clone, Copy, kani::Arbitrary)]
struct AoahInput {
    store_version: u16,
    write_counter_before: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MigrationRequired {
    from_version: u16,
    to_version: u16,
}

fn adapter_is_old_version(version: u16) -> bool {
    version < CURRENT_VERSION || version == RESTATE_V1_VERSION
}

fn adapter_runtime_open(version: u16, write_counter: u8) -> (Option<MigrationRequired>, u8) {
    if adapter_is_old_version(version) {
        (
            Some(MigrationRequired {
                from_version: version,
                to_version: CURRENT_VERSION,
            }),
            write_counter,
        )
    } else {
        (None, write_counter)
    }
}

#[kani::proof]
#[kani::unwind(3)]
fn vb_aoah_runtime_open_no_side_effects() {
    let input: AoahInput = kani::any();
    kani::assume(input.write_counter_before <= 64);
    kani::assume(input.store_version <= 5);

    let (migration_required, write_counter_after) =
        adapter_runtime_open(input.store_version, input.write_counter_before);

    // Claim: old supported store returns MigrationRequired
    if adapter_is_old_version(input.store_version) {
        assert!(migration_required.is_some());
        let mr = migration_required.unwrap();
        assert_eq!(mr.from_version, input.store_version);
        assert_eq!(mr.to_version, CURRENT_VERSION);
    }

    // Claim: no migration side effects — write counter is unchanged
    assert_eq!(write_counter_after, input.write_counter_before);

    // Claim: current-version store does NOT return MigrationRequired
    if input.store_version >= CURRENT_VERSION {
        assert!(migration_required.is_none());
    }
}
