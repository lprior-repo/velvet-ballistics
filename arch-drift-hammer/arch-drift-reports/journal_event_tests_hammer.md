# ARCHITECTURAL DRIFT REPORT

**File**: `crates/vb_storage/src/journal/journal_event_tests.rs`
**Total Lines**: 642
**Limit**: 300
**Violation**: YES — 214% of line budget

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 642 | 300 | **FAIL** |
| Over by | 342 | 0 | **FAIL** |

**Required action**: File MUST be split into at least 3 smaller test modules.

---

## 2. RESPONSIBILITY MAP

The file tests **two distinct public functions** across **six concern areas**:

```
journal_event_tests.rs (642 lines)
├── parse_event happy-path tests (lines 24–94)          [2 tests]
│   └── Validates correct round-trip encode→decode
├── parse_event BadMagic tests (lines 100–166)           [3 tests]
│   └── Wrong magic, zero magic, 0xFFFFFFFF magic
├── parse_event UnexpectedEof tests (lines 172–229)      [3 tests]
│   └── Empty input, short header, truncated payload
├── parse_event PayloadDigestMismatch test (lines 235–264)[1 test]
│   └── Corrupt payload detection
├── parse_event UnsupportedSchemaVersion test (lines 270–304)[1 test]
│   └── Future schema version rejection
├── parse_event UnknownRecordKind test (lines 310–343)   [1 test]
│   └── Invalid record kind = 999
├── parse_event boundary tests (lines 349–373)           [1 test]
│   └── Minimum valid record
├── JournalEvent::is_valid tests (lines 379–481)         [7 tests]
│   └── Structural validity predicates
├── parse_event + is_valid invariant (lines 487–607)    [1 test]
│   └── ALL 18 variants produce valid parsed output
└── parse_event payload-too-large test (lines 613–642)   [1 test]
    └── PayloadTooLarge error
```

**Test count**: 21 test functions total.

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### VIOLATION 1: Magic byte construction from primitives (3 occurrences)

**Location**: Lines 134–136, 155, 287, 327

```rust
// Line 134-136 — hardcoded magic 0x00000000 as raw u32
bytes[0..4].copy_from_slice(&0x0000_0000u32.to_le_bytes());

// Line 155 — raw 0xFFu8 repeated RECORD_HEADER_BYTES times
let bytes = vec![0xFFu8; RECORD_HEADER_BYTES];

// Line 327 — hardcoded magic 999u16 for record kind
bytes[6..8].copy_from_slice(&999u16.to_le_bytes());
```

**Problem**: `0x0000_0000`, `0xFFFF_FFFF`, `999` are magic primitives. There is already `MAGIC_BLOB` in scope, but no `MAGIC_JOURNAL_EVENT_DEFAULT` or `INVALID_RECORD_KIND` constant.

**Refactor**: Extract:
- `const INVALID_MAGIC: u32 = 0x0000_0000;`
- `const INVALID_RECORD_KIND: u16 = 999;`

---

### VIOLATION 2: Hardcoded magic offsets with raw numbers

**Location**: Lines 136–138, 287–288, 291–292, 327–331

```rust
bytes[0..4].copy_from_slice(&0x0000_0000u32.to_le_bytes()); // magic at 0
bytes[4..6].copy_from_slice(&1u16.to_le_bytes());            // schema version at 4
bytes[4..6].copy_from_slice(&future_version.to_le_bytes());  // schema version at 4
bytes[CRC_OFFSET..CRC_OFFSET + 4]                             // CRC at CRC_OFFSET
bytes[6..8].copy_from_slice(&999u16.to_le_bytes());          // kind at 6
```

**Problem**: Offsets `0`, `4`, `6`, `CRC_OFFSET` are magic numbers. There is already `RECORD_HEADER_BYTES` and `CRC_OFFSET` as named constants — but the offset values `4` and `6` are used directly without referencing named constants that describe what lives at those offsets.

**Refactor**: Define named constants in `journal/header.rs`:
```rust
pub const SCHEMA_VERSION_OFFSET: usize = 4;
pub const RECORD_KIND_OFFSET: usize = 6;
```

---

### VIOLATION 3: Raw attempt values

**Location**: Lines 440, 473, 510, etc.

```rust
attempt: 0,  // Lines 440, 473
attempt: 1,  // Lines 388, 510, 512, 518, 520, ...
```

**Problem**: `attempt: 0` and `attempt: 1` are used as magic values throughout. There is no `Attempt::INVALID` or `Attempt::FIRST` newtype.

**Refactor**: These already use `u32` raw types. Should be `Attempt(u32)` newtype with constants:
```rust
pub const Attempt::ZERO = Attempt(0);
pub const Attempt::FIRST = Attempt(1);
```

---

### VIOLATION 4: Schema version arithmetic with raw u16

**Location**: Lines 287–288

```rust
let future_version: u16 = CURRENT_SCHEMA_VERSION + 1;
bytes[4..6].copy_from_slice(&future_version.to_le_bytes());
```

**Problem**: `CURRENT_SCHEMA_VERSION + 1` is a magic operation. The schema version field is raw `u16` with no `SchemaVersion` newtype.

**Refactor**: Define `SchemaVersion(u16)` with `next_version()` method.

---

## 4. DDD PRINCIPLE VIOLATIONS

### Parse, Don't Validate — PARTIAL ADHERENCE

`parse_event` does return a fully-constructed `JournalEvent` which is then checked via `is_valid()`. This is **validate** not **parse**. The proper DDD approach:

```rust
// CURRENT (validate after parse):
let parsed = parse_event(&bytes)?;
if !parsed.is_valid() { return Err(...); }

// PREFERRED (parse = validate):
// parse_event should only succeed for structurally valid events.
// No is_valid() call needed after successful parse.
```

The `parse_event_result_is_valid_is_true_for_all_variants` test (lines 488–607) confirms the current design smell — it proves that **every successful parse is valid**, meaning `is_valid()` is redundant for the success path.

### State Transition Modeling — NOT APPLICABLE

These are pure parsing tests. No workflow/state machine tests present. This is acceptable for a test file.

### Exhaustive Variant Coverage — GOOD

The `parse_event_result_is_valid_is_true_for_all_variants` test (lines 488–607) does enumerate all 18 `JournalEvent` variants. This is excellent coverage discipline.

---

## 5. REQUIRED REFACTORING

### Step 1: Split into 3 modules

```
src/journal/
├── mod.rs
├── parse_event/
│   ├── mod.rs          # re-exports
│   ├── happy_tests.rs  # 2 tests  (~70 lines)
│   ├── error_tests.rs  # 9 tests  (~250 lines)
│   └── invariant_tests.rs  # 2 tests  (~140 lines)
├── is_valid/
│   ├── mod.rs
│   └── validity_tests.rs   # 7 tests  (~110 lines)
└── integration_tests.rs    # 1 test   (~60 lines)
```

**Target**: Each file ≤ 150 lines.

### Step 2: Extract magic constant newtypes

Create `crates/vb_storage/src/journal/constants.rs` or extend existing:

```rust
/// Magic for an invalid/zero journal header
pub const INVALID_JOURNAL_MAGIC: u32 = 0x0000_0000;
/// Magic for an explicitly invalid all-ones magic
pub const INVALID_JOURNAL_MAGIC_FF: u32 = 0xFFFF_FFFF;
/// Sentinel record kind for unknown-kind tests
pub const UNKNOWN_RECORD_KIND: u16 = 999;
```

### Step 3: Introduce Attempt newtype

In `vb_core` or shared types:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attempt(u32);

impl Attempt {
    pub const ZERO: Attempt = Attempt(0);
    pub const FIRST: Attempt = Attempt(1);
    pub fn new(v: u32) -> Option<Attempt> { (v > 0).then_some(Attempt(v)) }
}
```

---

## 6. VERDICT

| Check | Result |
|-------|--------|
| Line count ≤ 300 | **FAIL** (642 lines) |
| Primitive obsession | **FAIL** (5 violations) |
| Parse don't validate | **PARTIAL** (redundant `is_valid()`) |
| Named constants for magic numbers | **FAIL** |
| Exhaustive coverage | **PASS** (18 variants) |

**STATUS: REFACTOR REQUIRED**

File must be split into ≤300 line chunks and primitive obsession violations resolved before this file passes architectural review.
