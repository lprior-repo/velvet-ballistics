# vb-ct31x: Durable Emergency-Deny Marker Storage Seam

## Scope

New dedicated Fjall keyspace for a monotonic emergency-deny marker. Scoped to
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

Single fixed key: `"active"`. Value encoding (9 bytes total):

```
byte[0]     : version u8 (current = 1)
byte[1..9]  : big-endian u64 monotonic activation sequence
```

- Missing key → `EmergencyDenyState::Inactive`
- Valid versioned marker → `EmergencyDenyState::Active { seq }`
- Unknown version byte → `JournalError::EmergencyDenyMarkerUnsupportedVersion { version }`
- Wrong length → `JournalError::EmergencyDenyMarkerMalformed`
- Any Fjall read/write error propagates via `JournalError::Fjall`

The one-byte version prefix allows future format changes without losing
existing stored markers.

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
    Active { seq: u64 },
}
```

### Public API (crates/vb_storage/src/journal/core.rs)

```rust
impl FjallJournal {
    /// Returns the current emergency-deny state.
    ///
    /// `Inactive` when no marker exists. `Active { seq }` when a valid
    /// versioned marker is present. Storage read errors return
    /// `JournalError::Fjall`, never `Inactive`. Malformed or
    /// unsupported-version markers return their specific typed errors,
    /// never `Inactive`.
    pub fn emergency_deny_state(&self) -> Result<EmergencyDenyState, JournalError>;

    /// Activates the emergency-deny overlay with strict durability.
    ///
    /// Writes a monotonic versioned marker and persists with
    /// `PersistMode::SyncAll` before returning. Idempotent: repeated
    /// calls write fresh monotonic sequences but the state remains
    /// `Active`. Never overwrites or clears an existing marker —
    /// corruption is detected and reported as a typed error.
    /// Returns the new state.
    pub fn activate_emergency_deny(&self) -> Result<EmergencyDenyState, JournalError>;

    /// Returns the emergency-deny keyspace handle for internal operations.
    pub fn emergency_deny_keyspace(&self) -> &fjall::Keyspace;
}
```

### Integration Points

1. `FjallJournal::open` creates the new keyspace
2. `declared_keyspaces()` returns 11 names (mechanical count update)
3. `ReadOnlyJournal::declared_keyspaces()` returns same 11
4. `probe_health()` includes the new keyspace

## Behavior Rules

| Rule | Description |
|------|-------------|
| Missing = Inactive | No marker key → `Inactive`, never an error |
| Valid marker = Active | Version 1 + 8-byte big-endian u64 seq |
| Typed errors for corruption | Wrong version, wrong length, Fjall error — never silently Inactive |
| Atomic strict persist | Write + `SyncAll` in single `JournalWriteBatch::commit()` |
| No clear/remove API | Monotonic; activation only ever adds new sequence |
| Idempotent activation | Repeated calls succeed, each increments sequence |
| Storage scope | Same Fjall store directory as all other keyspace data |

## Tests

Public integration-style tests in `crates/vb_storage/src/journal/emergency_deny_tests.rs`:

1. New store is Inactive
2. Activate → Active with seq > 0
3. Repeated activation is idempotent (state remains Active)
4. Drop journal, reopen, marker persists (cold reopen)
5. Separate stores are isolated
6. Malformed marker (wrong length) → typed error
7. Unsupported version → typed error
8. Failed persist returns error, not success (cfg(test) injection)
9. Original run event history unchanged after activation

Corruption fixtures use crate-private unit tests writing raw bytes via the
Fjall keyspace directly (no new public backdoor API).

## Files Changed

- `crates/vb_storage/src/constants.rs` — new keyspace constant
- `crates/vb_storage/src/journal/core.rs` — new methods, keyspace creation
- `crates/vb_storage/src/journal/readonly.rs` — updated `declared_keyspaces()`
- `crates/vb_storage/src/error/mod.rs` — new error variants
- `crates/vb_storage/src/types.rs` — `EmergencyDenyState` enum (or local to core)
- `crates/vb_storage/src/journal/emergency_deny_tests.rs` — new test module
