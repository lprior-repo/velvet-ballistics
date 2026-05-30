# Architectural Drift Report: `vb_storage/src/types.rs`

**File**: `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/types.rs`  
**Line Count**: 301 (VIOLATES <300 rule)  
**Status**: REFACTOR REQUIRED

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 301 | 300 | **VIOLATED** |

The file exceeds the architectural limit by exactly 1 line.

---

## 2. STORAGE TYPE RESPONSIBILITY MAP

| Type | Responsibility | Assessment |
|------|----------------|------------|
| `StorageLimits` | Configures write limits | **GOOD** - NewType |
| `DurabilityProfile` | Runtime durability contract | **GOOD** - State machine enum |
| `KeyspaceProfile` | Fjall keyspace tuning | **GOOD** - Discriminated config |
| `keyspace_options_for()` | Profile → Fjall options | **GOOD** - Pure function |
| `EventSeq` | Monotonic event sequence | **GOOD** - NewType wrapper |
| `JournalQueueCapacity` | Non-zero queue capacity | **GOOD** - Validated NewType |
| `JournalBatchSize` | Non-zero batch size | **GOOD** - Validated NewType |
| `JournalWriterQueueProfileCounts` | Queue profile metrics | **PRIMITIVE OBSESSION** |
| `JournalWriterFlushReport` | Flush operation metrics | **PRIMITIVE OBSESSION** |
| `FjallConfig` | Fjall cache configuration | **PRIMITIVE OBSESSION** |
| `RecordEnvelope` | Decoded record metadata | **PRIMITIVE OBSESSION** |
| `RecordHeader` | Decoded 60-byte header | **PRIMITIVE OBSESSION** |
| `IndexStatusState` | State marker for index | **GOOD** - State machine enum |
| `StorageKey` | Keyspace key variants | **GOOD** - Domain enum |

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 3.1 `JournalWriterQueueProfileCounts` (lines 156-162)

```rust
pub struct JournalWriterQueueProfileCounts {
    pub journaled: usize,  // PRIMITIVE
    pub strict: usize,     // PRIMITIVE
}
```

**Violation**: `usize` is a primitive. These represent bounded counts in a domain where negative values are invalid.

**Fix**: Create `JournaledCount(usize)` and `StrictCount(usize)` NewTypes, or a generic `NonZeroUsize` wrapper similar to `JournalQueueCapacity`.

### 3.2 `JournalWriterFlushReport` (lines 165-171)

```rust
pub struct JournalWriterFlushReport {
    pub drained: usize,  // PRIMITIVE
    pub written: usize,   // PRIMITIVE
}
```

**Violation**: Same pattern — unbounded `usize` for counts that should be non-negative by construction.

**Fix**: Apply the same `NonZeroUsize` wrapper pattern.

### 3.3 `FjallConfig` (lines 174-186)

```rust
pub struct FjallConfig {
    pub cache_size_bytes: u64,  // PRIMITIVE
}
```

**Violation**: `u64` raw bytes for cache size. No validation that it's non-zero or properly aligned.

**Fix**: Wrap in `CacheSizeBytes(u64)` NewType with `try_from_u64()` validation.

### 3.4 `RecordEnvelope` (lines 189-199)

```rust
pub struct RecordEnvelope {
    pub magic: u32,           // PRIMITIVE - should be MagicNumber(u32)
    pub schema_version: u16,   // PRIMITIVE - should be SchemaVersion(u16)
    pub record_kind: u16,      // PRIMITIVE - should be RecordKind(u16)
    pub sequence: u64,         // PRIMITIVE - should be bounded
}
```

**Violation**: All fields are raw primitives representing domain concepts.

**Fix**: Create dedicated types:
- `MagicNumber(u32)` — validated magic constant
- `SchemaVersion(u16)` — version wrapper
- `RecordKind(u16)` — kind identifier
- `RecordSequence(u64)` — already exists as `EventSeq`, reuse it

### 3.5 `RecordHeader` (lines 202-220)

```rust
pub struct RecordHeader {
    pub magic: u32,               // PRIMITIVE
    pub schema_version: u16,       // PRIMITIVE
    pub record_kind: u16,         // PRIMITIVE
    pub header_len: u32,          // PRIMITIVE - should be bounded
    pub payload_len: u32,         // PRIMITIVE - should be bounded
    pub sequence: u64,            // PRIMITIVE
    pub payload_digest: [u8; 32], // GOOD - fixed array
    pub header_checksum: u32,     // PRIMITIVE
}
```

**Violation**: Multiple primitives that should be typed.

**Fix**: Apply same `RecordEnvelope` fixes, plus:
- `HeaderLength(u32)` — with bounds validation
- `PayloadLength(u32)` — with bounds validation
- `HeaderChecksum(u32)` — typed checksum

---

## 4. DUPLICATED VALIDATION PATTERN

`JournalQueueCapacity` (lines 108-127) and `JournalBatchSize` (lines 134-153) share identical implementation patterns:

```rust
impl JournalQueueCapacity {
    pub const fn new(value: NonZeroUsize) -> Self { Self(value) }
    pub fn try_from_usize(value: usize) -> Result<Self, crate::JournalError> { ... }
    pub const fn get(self) -> usize { self.0.get() }
}

impl JournalBatchSize {
    pub const fn new(value: NonZeroUsize) -> Self { Self(value) }
    pub fn try_from_usize(value: usize) -> Result<Self, crate::JournalError> { ... }
    pub const fn get(self) -> usize { self.0.get() }
}
```

**Issue**: Copy-paste duplication. Should be consolidated into a generic `BoundedCapacity<T>(NonZeroUsize)` or similar.

---

## 5. RECOMMENDED REFACTORING

### Split Strategy

1. **New file: `storage_limits.rs`**
   - `StorageLimits`
   - `DurabilityProfile`
   - `KeyspaceProfile`
   - `keyspace_options_for()`

2. **New file: `journal_types.rs`**
   - `EventSeq`
   - `JournalQueueCapacity`
   - `JournalBatchSize`
   - `JournalWriterQueueProfileCounts` (after fixing primitives)
   - `JournalWriterFlushReport` (after fixing primitives)

3. **New file: `fjall_config.rs`**
   - `FjallConfig` (after fixing primitives)

4. **New file: `record_types.rs`**
   - `RecordEnvelope` (after fixing primitives)
   - `RecordHeader` (after fixing primitives)

5. **New file: `index_types.rs`**
   - `IndexStatusState`
   - `StorageKey`

6. **Keep: `types.rs`** as a re-export module with minimal content (< 50 lines)

---

## 6. SUMMARY

| Category | Count | Severity |
|----------|-------|----------|
| Line count violations | 1 | CRITICAL |
| Primitive obsession violations | 5 structs | HIGH |
| Duplicated validation patterns | 2 structs | MEDIUM |
| Well-typed domain types | 7 | — |

**Total violations**: 1 CRITICAL + 5 HIGH + 1 MEDIUM

---

## 7. ACTION ITEMS

- [ ] Split file into domain-specific modules
- [ ] Create NewType wrappers for all primitive fields in `RecordEnvelope`, `RecordHeader`, `JournalWriterQueueProfileCounts`, `JournalWriterFlushReport`, `FjallConfig`
- [ ] Consolidate `JournalQueueCapacity` and `JournalBatchSize` into generic type
- [ ] Verify all compilation after refactor
- [ ] Ensure <300 lines per file rule compliance
