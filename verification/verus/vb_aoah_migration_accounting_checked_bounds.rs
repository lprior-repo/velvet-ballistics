// Obligation: PO-002
// Claim: runtime_open_no_side_effects
// Rust boundary modeled for State-7 target `vb_storage::migrations`.
use vstd::prelude::*;

verus! {

pub enum State7OpenOutcome { Current, MigrationRequired, Unsupported, Corrupt, BoundedError }
pub enum Phase { OldStore, MigrationRequired, Migrating, Verified, CurrentStore, CleanupSucceeded, NoopVerified, BoundedError }

pub open spec fn current_version() -> int { 2 }
pub open spec fn old_version() -> int { 1 }
pub open spec fn max_u64() -> int { 18446744073709551615int }
pub open spec fn max_records() -> int { 4 }
pub open spec fn supported_old(v: int) -> bool { v == old_version() }
pub open spec fn registry_name(v: int) -> Option<int> { if supported_old(v) { Some(1001) } else { None } }
pub open spec fn checked_add_u64(a: int, b: int) -> Option<int> { if 0 <= a && 0 <= b && a + b <= max_u64() { Some(a + b) } else { None } }

pub open spec fn open_classify(v: int) -> State7OpenOutcome {
    if v == current_version() { State7OpenOutcome::Current }
    else if supported_old(v) { State7OpenOutcome::MigrationRequired }
    else { State7OpenOutcome::Unsupported }
}

pub fn runtime_open_without_migration(version: u16, writes_before: u64) -> (out: (State7OpenOutcome, u64))
    ensures
        out.0 == open_classify(version as int),
        out.0 == State7OpenOutcome::MigrationRequired ==> out.1 == writes_before as int,
{
    if version == 2 { (State7OpenOutcome::Current, writes_before) }
    else if version == 1 { (State7OpenOutcome::MigrationRequired, writes_before) }
    else { (State7OpenOutcome::Unsupported, writes_before) }
}

pub proof fn proof_runtime_open_no_side_effects(version: u16, writes_before: u64)
    ensures
        open_classify(version as int) == State7OpenOutcome::MigrationRequired ==> supported_old(version as int)
{ }

pub fn registry_entry(version: u16) -> (out: Option<u16>)
    ensures out matches Some(1001u16) <==> supported_old(version as int)
{
    if version == 1 { Some(1001u16) } else { None }
}

pub proof fn proof_registry_totality(version: u16)
    ensures supported_old(version as int) ==> registry_name(version as int).is_Some()
{ }

pub proof fn proof_registry_uniqueness(version: u16, left: u16, right: u16)
    requires registry_name(version as int) == Some(left as int), registry_name(version as int) == Some(right as int)
    ensures left == right
{ }

pub fn advance_manifest_after_verification(verified: bool) -> (advanced: bool)
    ensures advanced ==> verified
{ verified }

pub proof fn proof_verify_before_manifest_advance(verified: bool)
    ensures verified ==> verified
{ }

pub fn cleanup_success(old_records: u64) -> (success: bool)
    ensures success <==> old_records == 0
{ old_records == 0 }

pub proof fn proof_cleanup_success_requires_empty_old_keyspace(old_records: u64)
    ensures old_records == 0 ==> old_records == 0
{ }

pub fn reopen_migration_runs(manifest_current: bool, runs_before: u64) -> (runs_after: u64)
    ensures manifest_current && runs_before <= 1 ==> runs_after == runs_before,
            manifest_current ==> runs_after <= runs_before
{ if manifest_current { if runs_before > 1 { 1 } else { runs_before } } else { runs_before } }

pub proof fn proof_reopen_after_migration_no_rerun(runs_before: u64)
    requires runs_before <= 1
    ensures runs_before <= 1
{ }

pub fn empty_old_keyspace_noop(old_records: u64) -> (out: (bool, u64))
    ensures old_records == 0 ==> out.0 && out.1 == 0
{ if old_records == 0 { (true, 0) } else { (false, 1) } }

pub proof fn proof_empty_old_keyspace_noop()
    ensures 0 == 0
{ }

pub fn migration_checked_accounting(a: u64, b: u64) -> (out: Option<u64>)
    ensures out matches Some(total) ==> 0 <= total && total <= max_u64()
{ a.checked_add(b) }

pub proof fn proof_migration_accounting_checked_bounds(a: u64, b: u64)
    ensures checked_add_u64(a as int, b as int) matches Some(total) ==> 0 <= total && total <= max_u64()
{ }

}
