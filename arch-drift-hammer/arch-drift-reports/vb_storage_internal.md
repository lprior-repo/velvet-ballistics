# Architectural Drift Report: `vb_storage/src/journal/internal.rs`

**File**: `crates/vb_storage/src/journal/internal.rs`  
**Lines**: 75  
**Status**: PERFECT (within 300-line limit)

---

## 1. Line Count Check

| Metric | Value | Limit | Pass/Fail |
|--------|-------|-------|-----------|
| Total Lines | 75 | 300 | ✅ PASS |

---

## 2. DDD Cohesion Analysis

### Cohesion Score: **GOOD**

The file contains a single `impl FjallJournal` block with 3 tightly-related methods:

| Method | Responsibility |
|--------|----------------|
| `decode_optional` | Generic optional record decoding with magic/version check |
| `append_unpersisted` | Append event to journal with duplicate detection |
| `append_queued_unpersisted` | Idempotent append for queued events |

All methods operate on the same aggregate (`FjallJournal`) and deal with event persistence concerns.

---

## 3. Violations

### Violation 1: Primitive Obsession (LOW SEVERITY)
**Location**: `decode_optional` signature
```rust
pub(crate) fn decode_optional<T: DeserializeOwned>(
    &self,
    keyspace: &fjall::Keyspace,  // Infrastructure leak
    key: &[u8],                   // Primitive: should be domain key type
    magic: u32,                   // Primitive: should be typed magic constant
    max_bytes: u32,               // Primitive: should be bounded type
) -> Result<Option<T>, JournalError>
```

**Recommendation**: Wrap `magic` in a typed `MagicVersion<T>` newtype and `max_bytes` in a `MaxBytes` bounded integer.

### Violation 2: Infrastructure Leakage
**Location**: `keyspace: &fjall::Keyspace`
- `fjall::Keyspace` is a Fjall-specific type, leaking storage engine details into the domain
- The `impl` block should be behind a `JournalBackend` trait to enable port/adapter separation

**Recommendation**: Introduce a `JournalStorage` trait abstraction.

### Violation 3: Primitive Parameters in `append_unpersisted`
**Location**: `encode_record` calls
```rust
encode_record(
    MAGIC_JOURNAL_EVENT,          // u32 constant
    event.record_kind(),
    event.seq().get(),            // u32 extraction
    event,
    MAX_JOURNAL_EVENT_PAYLOAD_BYTES,  // u32 constant
)?
```
The magic and max_bytes are raw `u32` values passed across module boundaries.

---

## 4. DDD Smell Assessment

| Smell | Severity | Notes |
|-------|----------|-------|
| Primitive Obsession | LOW | Magic/version and size bounds should be typed |
| Infrastructure Leakage | MEDIUM | `fjall::Keyspace` is concrete, not abstracted |
| Anemic Control Flow | NONE | `append_queued_unpersisted` match is appropriate for duplicate handling |
| State Machine Modeling | OK | Duplicate detection models legitimate state |

**Overall DDD Smell**: MINIMAL — File is cohesive and focused on journal persistence.

---

## 5. Priority Assessment

| Category | Priority |
|----------|----------|
| Line Count Violation | **NONE** |
| DDD Cohesion Issue | **LOW** |
| Refactor Urgency | **OPTIONAL** — Current design is functional, but could benefit from typed abstractions |

---

## 6. Recommendations

1. **Optional (Low Priority)**: Introduce `MagicVersion<T>` and `MaxBytes<N>` types if the codebase has precedent for such wrappers
2. **Optional (Low Priority)**: Extract `encode_record`/`decode_record` to a dedicated codec module if they grow
3. **Current State**: File is acceptable — no mandatory refactoring required

---

*Report generated: 2026-05-29*  
*Drift Agent: architectural-drift*
