// Obligation: PO-R04
// Claim: Cleanup-required migration reports success only after the old
// keyspace is empty. Non-empty old keyspace returns typed cleanup error.
#![cfg(kani)]

const MAX_RECORDS: u8 = 8;

#[derive(Clone, Copy, kani::Arbitrary)]
struct AoahInput {
    old_record_count: u8,
    cleanup_required: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CleanupResult {
    Success,
    ErrorOldKeyspaceNotEmpty,
    NoCleanupNeeded,
}

fn adapter_cleanup(old_count: u8, required: bool) -> CleanupResult {
    if !required {
        return CleanupResult::NoCleanupNeeded;
    }
    if old_count == 0 {
        CleanupResult::Success
    } else {
        CleanupResult::ErrorOldKeyspaceNotEmpty
    }
}

#[kani::proof]
#[kani::unwind(3)]
fn vb_aoah_cleanup_success_requires_empty_old_keyspace() {
    let input: AoahInput = kani::any();
    kani::assume(input.old_record_count <= MAX_RECORDS);

    let result = adapter_cleanup(input.old_record_count, input.cleanup_required);

    // Claim: cleanup success is unreachable while old keyspace is non-empty
    if input.old_record_count > 0 && input.cleanup_required {
        assert_ne!(result, CleanupResult::Success);
    }

    // Claim: cleanup succeeds only when old keyspace is empty
    if result == CleanupResult::Success {
        assert!(input.cleanup_required);
        assert_eq!(input.old_record_count, 0);
    }

    // Claim: typed error returned for non-empty old keyspace
    if result == CleanupResult::ErrorOldKeyspaceNotEmpty {
        assert!(input.old_record_count > 0);
    }
}
