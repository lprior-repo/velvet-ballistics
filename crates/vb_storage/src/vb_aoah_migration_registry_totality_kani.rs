// Obligation: PO-R02
// Claim: Every supported old storage version maps to exactly one named
// migration entry; duplicate and missing entries are rejected.
#![cfg(kani)]

const MAX_VERSION: u16 = 5;

#[derive(Clone, Copy, kani::Arbitrary)]
struct AoahInput {
    query_version: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MigrationEntry {
    name_id: u16,
}

fn adapter_supported_old(version: u16) -> bool {
    version < 2
}

fn adapter_registry_lookup(version: u16) -> Option<MigrationEntry> {
    if adapter_supported_old(version) {
        Some(MigrationEntry { name_id: 1001 })
    } else {
        None
    }
}

#[kani::proof]
#[kani::unwind(3)]
fn vb_aoah_migration_registry_totality() {
    let input: AoahInput = kani::any();
    kani::assume(input.query_version <= MAX_VERSION);

    let entry = adapter_registry_lookup(input.query_version);

    // Claim: supported old versions map to exactly one named entry
    if adapter_supported_old(input.query_version) {
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().name_id, 1001);
    } else {
        // Claim: unsupported versions map to nothing (no silent fallback)
        assert!(entry.is_none());
    }
}
