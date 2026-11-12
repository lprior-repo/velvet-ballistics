# vb-ct31x: Durable Emergency-Deny Marker Storage Seam

## Scope

New dedicated Fjall keyspace for an emergency-deny marker. Scoped to
the Fjall journal store directory, NOT run-scoped, NOT RunId0. Stable scope
means the marker persists across Runtime/process recreation using the same
journal directory. This is storage only: no Runtime dispatch logic, no policy
configuration changes. Child bead to vb-ny068; does not authorize Runtime
effects or close vb-ny068.

## Source Seams and Types

### New Keyspace

| Constant | Value | Profile |
|----------|-------|---------|
| `KEYSPACE_EMERGENCY_DENY` | `"emergency_deny"` | `Hot` (low latency, single key, frequent check) |

Added to `crates/vb_storage/src/constants.rs`.

### Marker Encoding

Single fixed key: `"active"`. Value encoding: exact one byte `[1]` where `1` is version 1.

- Missing key → `EmergencyDenyState::Inactive`
- Valid exact `[1]` → `EmergencyDenyState::Active`
- Zero-length or extra bytes → `JournalError::EmergencyDenyMarkerMalformed`
- Wrong single version byte → `JournalError::EmergencyDenyMarkerUnsupportedVersion { version }`
- Any Fjall read/write error propagates via `JournalError::Fjall`

No sequence counter. Constant marker repeated writes are genuinely idempotent
with full equality. No old 9-byte migration needed; this is an unaccepted
unreleased prototype.

### Error Variants (crates/vb_storage/src/error/mod.rs)

```rust
#[error("emergency deny marker has unsupported version: {version}")]
EmergencyDenyMarkerUnsupportedVersion { version: u8 },

#[error("emergency deny marker is malformed (wrong length or encoding)")]
EmergencyDenyMarkerMalformed,
```

### State Enum

```rust
pub enum EmergencyDenyState {
    Inactive,
    Active,
}
```

### Public API (crates/vb_storage/src/journal/emergency_deny.rs)

```rust
impl FjallJournal {
    /// Returns the current emergency-deny state.
    ///
    /// `Inactive` when no marker exists. `Active` when a valid
    /// versioned marker is present. Storage read errors return
    /// `JournalError::Fjall`, never `Inactive`. Malformed or
    /// unsupported-version markers return their specific typed errors,
    /// never `Inactive`.
    pub fn emergency_deny_state(&self) -> Result<EmergencyDenyState, JournalError>;

    /// Activates the emergency-deny overlay with strict durability.
    ///
    /// Reads and validates any existing marker first, rejecting corrupt
    /// or unsupported state without overwriting it. Writes the constant
    /// active marker and persists with `PersistMode::SyncAll` before
    /// returning success. Always performs a strict-batch commit+SyncAll
    /// even if already Active, because a prior failed persistence can
    /// leave visible but not durably confirmed data.
    ///
    /// Idempotent: repeated calls write the same constant marker,
    /// producing full equality with no overflow or race counter.
    /// Returns the new state.
    pub fn activate_emergency_deny(&self) -> Result<EmergencyDenyState, JournalError>;
}
```

No public `emergency_deny_keyspace` accessor. No public `put_emergency_deny`
with arbitrary key/value. Batch marker putter is pub(crate), bound to fixed
key and constant active marker.

### Integration Points

1. `FjallJournal::open` creates the new keyspace
2. `declared_keyspaces()` returns 11 names (mechanical count update)
3. `ReadOnlyJournal::declared_keyspaces()` returns same 11
4. `probe_health()` includes the new keyspace

## Behavior Rules

| Rule | Description |
|------|-------------|
| Missing = Inactive | No marker key → `Inactive`, never an error |
| Valid marker = Active | Exact one byte `[1]` version 1 |
| Typed errors for corruption | Wrong version, wrong length, Fjall error — never silently Inactive |
| Atomic strict persist | Write + `SyncAll` in single `JournalWriteBatch::commit()` |
| No clear/remove API | Activation only ever writes the constant marker |
| Idempotent activation | Repeated calls succeed, produce exact equality, no counter |
| Storage scope | Same Fjall store directory as all other keyspace data |
| Validate before write | Activation reads/validates existing marker first |
| Always persist on activate | No shortcut for already-Active; always strict-batch commit+SyncAll |

## Tests

Public integration-style tests in `crates/vb_storage/src/journal/emergency_deny_tests.rs`:

1. New store is Inactive
2. Activate → Active
3. Repeated activation is idempotent (exact equality)
4. Drop journal, reopen, marker persists (cold reopen)
5. Separate stores are isolated
6. Malformed marker (wrong length) → typed error on read AND activation
7. Unsupported version → typed error on read AND activation
8. Failed persist returns error, not success (PRECOMMIT failure injection)
9. Retry after injected failure succeeds
10. Already Active retry still performs strict persistence
11. Original run event history unchanged after activation

All tests use Result return and ?/let patterns, zero expect/unwrap/panic,
retain TempDir for fixture lifetime, prove exact state equality, and for
corruption cases prove both state read and activation reject with exact
errors while preserving raw bad bytes.

## Files Changed

- `crates/vb_storage/src/constants.rs` — new keyspace constant
- `crates/vb_storage/src/journal/core.rs` — new module import, removed inline emergency deny methods
- `crates/vb_storage/src/journal/emergency_deny.rs` — NEW: dedicated module with enum, constants, state/activate methods
- `crates/vb_storage/src/journal/batch/putters.rs` — pub(crate) put_emergency_deny with fixed key
- `crates/vb_storage/src/journal/readonly.rs` — updated `declared_keyspaces()`
- `crates/vb_storage/src/error/mod.rs` — new error variants
- `crates/vb_storage/src/journal/emergency_deny_tests.rs` — NEW: test module
