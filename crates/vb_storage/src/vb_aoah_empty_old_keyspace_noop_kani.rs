// Obligation: PO-R06
// Claim: Empty old-keyspace behavior is explicit no-op and cannot silently
// claim an unverified migration.
#![cfg(kani)]

#[derive(Clone, Copy, kani::Arbitrary)]
struct AoahInput {
    old_record_count: u8,
    verified: bool,
    manifest_version: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EmptyKeyspaceOutcome {
    NoOp,
    MigrationExecuted,
    SilentAdvanceBlocked,
}

const CURRENT_VERSION: u16 = 2;

fn adapter_empty_keyspace_outcome(
    old_count: u8,
    verified: bool,
    manifest: u16,
) -> EmptyKeyspaceOutcome {
    if old_count > 0 {
        return EmptyKeyspaceOutcome::MigrationExecuted;
    }
    if manifest >= CURRENT_VERSION {
        return EmptyKeyspaceOutcome::NoOp;
    }
    if verified {
        EmptyKeyspaceOutcome::NoOp
    } else {
        EmptyKeyspaceOutcome::SilentAdvanceBlocked
    }
}

#[kani::proof]
#[kani::unwind(3)]
fn vb_aoah_empty_old_keyspace_noop() {
    let input: AoahInput = kani::any();
    kani::assume(input.old_record_count <= 8);
    kani::assume(input.manifest_version <= 5);

    let outcome = adapter_empty_keyspace_outcome(
        input.old_record_count,
        input.verified,
        input.manifest_version,
    );

    // Claim: empty old keyspace produces explicit NoOp, not silent success
    if input.old_record_count == 0 && (input.verified || input.manifest_version >= CURRENT_VERSION)
    {
        assert_eq!(outcome, EmptyKeyspaceOutcome::NoOp);
    }

    // Claim: empty old keyspace cannot silently claim unverified migration
    if outcome == EmptyKeyspaceOutcome::SilentAdvanceBlocked {
        assert!(!input.verified);
        assert!(input.manifest_version < CURRENT_VERSION);
    }

    // Claim: non-empty old keyspace is not no-op
    if input.old_record_count > 0 {
        assert_eq!(outcome, EmptyKeyspaceOutcome::MigrationExecuted);
    }
}
