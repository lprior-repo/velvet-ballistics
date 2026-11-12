# vb-ct31x Handoff: Durable Emergency-Deny Marker Storage

## Status: Candidate for Coordinator Review

All acceptance criteria from the scoped storage contract are implemented, tested, and passing. No Runtime dispatch logic, no policy configuration changes — storage seam only.

## Changed Files

| File | Change |
|------|--------|
| `crates/vb_storage/src/constants.rs` | Added `KEYSPACE_EMERGENCY_DENY = "emergency_deny"` constant |
| `crates/vb_storage/src/journal/core.rs` | New `EmergencyDenyState` enum; FjallJournal `emergency_deny` keyspace field; `emergency_deny_state()` and `activate_emergency_deny()` methods; `emergency_deny_keyspace()` accessor; `declared_keyspaces()` updated to 11 names; `probe_health()` includes new keyspace |
| `crates/vb_storage/src/journal/readonly.rs` | `declared_keyspaces()` updated to 11 names |
| `crates/vb_storage/src/batch/putters.rs` | Added `JournalWriteBatch::put_emergency_deny()` for atomic batch writes |
| `crates/vb_storage/src/journal/mod.rs` | Export `EmergencyDenyState` and register `emergency_deny_tests` module |
| `crates/vb_storage/src/error/mod.rs` | Added `EmergencyDenyMarkerUnsupportedVersion` and `EmergencyDenyMarkerMalformed` variants |
| `crates/vb_storage/src/error/codes.rs` | Added diagnostic codes `EMERGENCY_DENY_UNSUPPORTED_VERSION_CODE` (0x4042) and `EMERGENCY_DENY_MALFORMED_CODE` (0x4043) plus symbolic codes |
| `crates/vb_storage/src/journal/emergency_deny_tests.rs` | 9 behavioral tests covering all required scenarios |
| `crates/vb_storage/src/tests.rs` | Updated exhaustive error match test and declared keyspaces assertions (3 locations) |
| `crates/vb_storage/src/journal/tests.rs` | Updated declared keyspaces count assertion |

## New API

```rust
// State enum
pub enum EmergencyDenyState {
    Inactive,
    Active { seq: u64 },
}

// FjallJournal methods
pub fn emergency_deny_state(&self) -> Result<EmergencyDenyState, JournalError>
pub fn activate_emergency_deny(&self) -> Result<EmergencyDenyState, JournalError>
pub fn emergency_deny_keyspace(&self) -> &fjall::Keyspace
```

## New Error Types

| Error | Diagnostic Code | Symbolic Code | Condition |
|-------|-----------------|---------------|-----------|
| `EmergencyDenyMarkerUnsupportedVersion { version }` | 0x4042 | `EMERGENCY_DENY_UNSUPPORTED_VERSION` | Stored marker has version byte other than 1 |
| `EmergencyDenyMarkerMalformed` | 0x4043 | `EMERGENCY_DENY_MALFORMED` | Wrong length, encoding error, or missing bytes |

## Storage Contract

- **New keyspace:** `emergency_deny` (Hot profile)
- **Fixed key:** `b"active"`
- **Value encoding:** 9 bytes total: 1 byte version (currently 1) + 8 bytes big-endian u64 monotonic activation sequence
- **Missing key** = `Inactive` (not an error)
- **Activation** = write fresh monotonic sequence, persist with `PersistMode::SyncAll` via atomic batch, return new `Active` state
- **Idempotent:** repeated activation always succeeds, each time writing incremented sequence
- **No clear/remove API:** marker is monotonic and permanent

## Test Results

| Test | Status |
|------|--------|
| new_store_is_inactive | PASS |
| activate_returns_active_state | PASS |
| repeated_activation_is_idempotent | PASS |
| marker_persists_across_journal_close | PASS |
| separate_stores_are_isolated | PASS |
| malformed_marker_wrong_length_errors | PASS |
| unsupported_version_errors | PASS |
| failed_persist_returns_error_not_success | PASS |
| activation_does_not_corrupt_run_events | PASS |

**Full suite:** 1626 tests passed, 0 failed (lib only)
**Clippy:** 0 warnings (zero-tolerance `-D warnings`)

## Evidence Artifacts

| Prefix | Command | Result |
|--------|---------|--------|
| `.beads/vb-ct31x/evidence/DENY_V1_01` | `cargo test -p vb_storage --lib emergency_deny` | exit 0 |
| `.beads/vb-ct31x/evidence/DENY_V1_02` | `cargo test -p vb_storage --lib` | exit 0 |
| `.beads/vb-ct31x/evidence/DENY_V1_03` | `cargo clippy -p vb_storage --lib --all-features -- -D warnings` | exit 0 |

## Unresolved Gaps

None within the scoped contract. Runtime integration (policy evaluation, operator activation, dispatch logic) belongs to vb-ny068 and sibling beads, not this storage-only seam.

## Design Notes

- Version byte prefix allows future format changes without losing existing stored markers
- Monotonic u64 sequence provides total ordering and idempotent retry support
- Atomic batch + `SyncAll` gives the exact durability guarantee: caller sees success only after physical persistence
- Corruption detection: any malformed or unsupported marker returns a typed error, never silently `Inactive`
- Fjall I/O errors propagate as `JournalError::Fjall`, never converted to `Inactive`
