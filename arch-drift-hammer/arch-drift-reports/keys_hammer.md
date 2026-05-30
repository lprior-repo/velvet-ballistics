# Architectural Drift Report: `vb_storage/src/keys.rs`

**File**: `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/keys.rs`  
**Status**: 🚨 CRITICAL DRIFT - REQUIRES IMMEDIATE REFACTOR  
**Line Count**: 838 (exceeds 300-line limit by 179%)  

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 838 | 300 | 🚨 +538 over |
| Production Code | 181 | 300 | ✅ |
| Test Code | 657 | N/A | 🚨 78% of file |
| Test/Code Ratio | 3.63:1 | - | Warning |

**Verdict**: The file violates the <300 line rule by 538 lines. However, 657 of those lines are tests. The actual production code (lines 1-181) is within limits. The problem is the file organization—tests are co-located with implementation.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Direct `u64` Timestamp in Index Key

**Location**: `index_status_key()` at line 61-76

```rust
pub fn index_status_key(
    state: crate::types::IndexStatusState,
    timestamp: u64,   // ← PRIMITIVE OBSESSION
    run: RunId,
) -> Result<[u8; INDEX_STATUS_KEY_BYTES], JournalError>
```

**Problem**: `u64` is used directly for `timestamp`. This should be wrapped in a `Timestamp` or `UnixEpoch` value object that:
- Guarantees the value is in a valid range
- Provides semantic meaning (`timestamp` vs `run_id` vs `seq`)
- Enables type-safe serialization

**Violation**: Scott Wlaschin "Make illegal states unrepresentable" principle. A caller could accidentally swap `timestamp` and `run` parameters.

### 2.2 Raw `[u8; 32]` Digest Arrays

**Locations**:
- `workflow_source_key(digest: [u8; 32])` line 23
- `compiled_ir_key(digest: [u8; 32])` line 30
- `blob_key(digest: [u8; 32])` line 56

**Problem**: `Digest` is a domain concept (BLAKE3 hash) but implemented as a raw byte array. No type distinguishes a workflow source digest from a blob digest at the type level.

**Should Be**:
```rust
struct Digest(Box<[u8; 32]>);  // Value object
enum DigestKind { WorkflowSource, CompiledIr, Blob }
```

### 2.3 Raw `u64` in `StorageKey::IndexStatus`

**Location**: `types.rs` lines 287-292

```rust
IndexStatus {
    state: IndexStatusState,
    timestamp: u64,  // ← PRIMITIVE OBSESSION
    run: RunId,
}
```

**Problem**: The enum variant `StorageKey::IndexStatus` contains a bare `u64` for `timestamp`. This should be a `Timestamp` type.

### 2.4 Repetitive Error Handling Pattern

**Location**: Every encoding function (16+ occurrences)

```rust
key.try_push(PREFIX_INDEX_STATUS)
    .map_err(|_| JournalError::KeyCapacity)?;
// ... repeated 16+ times
```

**Problem**: The `KeyCapacity` error is returned via `.map_err(|_| JournalError::KeyCapacity)` everywhere. This should be centralized.

---

## 3. DDD VIOLATIONS

### 3.1 No Value Objects for Key Segments

The module defines keys like `[0x30][state_u8][timestamp_u64_be][run_id_u64_be]` but doesn't create value objects for these segments:

| Segment | Current Type | Should Be |
|---------|--------------|-----------|
| `state_u8` | `IndexStatusState` | `IndexStatusState` (OK) |
| `timestamp_u64_be` | `u64` | `Timestamp` |
| `run_id_u64_be` | `RunId` | `RunId` (OK) |
| `workflow_id_u32_be` | `u32` | `WorkflowId` |

### 3.2 No Type-Level Key Encoding

**Location**: All encoding functions

The current approach uses byte-level manipulation:
```rust
key.try_extend_from_slice(&timestamp.to_be_bytes())
```

A DDD-aligned approach would use typed getters/setters:
```rust
struct StatusIndexKey { ... }
impl StatusIndexKey {
    fn timestamp(&self) -> Timestamp { ... }
    fn run_id(&self) -> RunId { ... }
}
```

### 3.3 `encode_key` Dispatches to Typed Encoders

**Location**: Lines 112-131

```rust
pub fn encode_key(key: StorageKey) -> Result<Vec<u8>, JournalError> {
    let encoded = match key {
        StorageKey::WorkflowSource { digest } => workflow_source_key(digest)?.to_vec(),
        // ... 8 more arms
    };
    Ok(encoded)
}
```

**Problem**: This violates DDD because `StorageKey` is a domain type but the encoding is externalized. The encoding logic should be methods on `StorageKey` itself (or a dedicated `KeyEncoder` service), not a standalone function.

---

## 4. ARCHITECTURAL CONCERNS

### 4.1 Test-In-Code Smell

**Location**: Lines 182-838

657 lines of tests are embedded in the source file. Per repository architecture:
- Tests should be in `crates/vb_storage/src/tests.rs` or `tests/` directory
- Source files should only contain production code + doc tests

### 4.2 Public `run_prefix` Exposes Internal

**Location**: Lines 152-154, 178-180

```rust
pub(crate) fn run_prefix_key(run: RunId) -> Result<[u8; 9], JournalError>
```

This re-exports an internal helper for use by `FjallJournal`. This is a code smell indicating the journal is tightly coupled to key encoding details.

### 4.3 No Decoder Functions

The module provides encoding (`workflow_source_key`, `run_event_key`, etc.) but no corresponding decoding functions. This breaks the symmetry required for DDD storage aggregates.

---

## 5. REFACTORING PRESCRIPTION

### 5.1 Immediate: Extract Tests

Move lines 182-838 to `crates/vb_storage/src/keys_tests.rs`. This alone brings the file to 181 lines.

### 5.2 Short-term: Create Value Objects

```rust
// In types.rs
pub struct Timestamp(u64);

pub struct Digest([u8; 32]);
pub enum DigestKind { WorkflowSource, CompiledIr, Blob }
```

### 5.3 Medium-term: Centralize Error Handling

Create a `KeyBuilder` struct that encapsulates the ArrayVec and error handling:
```rust
struct KeyBuilder<const N: usize> {
    buf: ArrayVec<u8, N>,
}
impl<const N: usize> KeyBuilder<N> {
    fn push(&mut self, byte: u8) -> Result<(), JournalError> { ... }
    fn extend(&mut self, bytes: &[u8]) -> Result<(), JournalError> { ... }
    fn finish(self) -> Result<[u8; N], JournalError> { ... }
}
```

### 5.4 Long-term: Move Encoding to Domain Types

```rust
impl StorageKey {
    pub fn encode(&self) -> Result<Vec<u8>, JournalError> {
        match self {
            StorageKey::WorkflowSource { digest } => workflow_source_key(digest).map(|k| k.to_vec()),
            // ...
        }
    }
}
```

---

## 6. RISK ASSESSMENT

| Risk | Severity | Likelihood | Notes |
|------|----------|------------|-------|
| Accidental parameter swap (timestamp/run) | High | Medium | No type safety on `u64` params |
| Key collision from encoding bugs | High | Low | Tests verify injectivity |
| Maintenance burden from test co-location | Medium | High | 657 lines of dead weight in source |
| Encoding inconsistency | Medium | Low | Pattern is uniform |

---

## 7. SUMMARY

| Category | Status |
|----------|--------|
| Line Count | 🚨 FAIL (838 > 300) |
| Primitive Obsession | 🚨 4 violations |
| DDD Cohesion | 🚨 Weak - no value objects for key segments |
| Test Location | 🚨 Tests embedded in source |
| Encoding/Decoding Symmetry | 🚨 Encode only, no decode |

**Recommendation**: **BLOCK** - This file requires refactoring before approval. Priority order:
1. Extract tests to separate file (reduces to 181 lines)
2. Add `Timestamp` value object
3. Add `Digest` value object with kind tracking
4. Centralize key building error handling
