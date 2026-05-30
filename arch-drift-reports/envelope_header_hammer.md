# ARCH-DRIFT REPORT: envelope_header.rs

**File**: `crates/vb_proof_kernels/src/envelope_header.rs`  
**Line Count**: 579 (LIMIT: 300) — **VIOLATION: +279 lines over limit**  
**Status**: MUST SPLIT

---

## 1. RESPONSIBILITY MAP

| Region | Lines | Responsibility |
|--------|-------|----------------|
| 1–6 | 6 | Module doc comment + `HEADER_LEN` constant |
| 8–21 | 14 | `EnvelopeHeader` struct definition (ALL pub fields) |
| 23–73 | 51 | `EnvelopeHeader` impl: constructors + validation methods |
| 75–92 | 18 | `ValidationError` + `ValidationResult` enums |
| 94–104 | 11 | Standalone public helper functions |
| 106–579 | **473** | `#[cfg(test)]` module — 81% of file |

**Verdict**: Single file doing too much. Core domain, validation, helpers, AND tests all welded together.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS (12 violations)

### 2.1 Raw Integer Fields in `EnvelopeHeader`

| Field | Raw Type | NewType Proposal | Encapsulated Behavior |
|-------|----------|-------------------|----------------------|
| `magic` | `u32` | `Magic(u32)` | `validate_magic()` becomes `self.0 == Self::MAGIC_VALUE` |
| `version` | `u8` | `Version(u8)` | Range check [1, 2] |
| `kind` | `u8` | `Kind(u8)` | Discriminant for envelope variant |
| `flags` | `u8` | `Flags(u8)` | Bitflag operations |
| `reserved` | `u8` | `Reserved(u8)` | Must be zero |
| `schema` | `u32` | `Schema(u32)` | Schema discriminant |
| `payload_len_u32` | `u32` | *(internal)* | Part of `PayloadLen` |
| `payload_len_hi` | `u32` | *(internal)* | Part of `PayloadLen` |
| `header_crc32` | `u32` | `HeaderCrc(u32)` | CRC-32 checksum |
| `payload_crc32` | `u32` | `PayloadCrc(u32)` | CRC-32 checksum |
| `blake3_digest` | `[u8; 32]` | `Blake3Digest([u8; 32])` | Digest type |

### 2.2 `payload_len_hi` + `payload_len_u32` = Primitive Obsession²

These two `u32` fields combine to form a `u64` via bit-shift:

```rust
pub fn payload_len(&self) -> u64 {
    u64::from(self.payload_len_hi) << 32 | u64::from(self.payload_len_u32)
}
```

**Scott Wlaschin Violation**: The combination logic is exposed, not encapsulated. Should be:

```rust
struct PayloadLen(u64);
impl PayloadLen {
    fn lo(&self) -> u32 { self.0 as u32 }
    fn hi(&self) -> u32 { (self.0 >> 32) as u32 }
}
```

### 2.3 `u64 max` Parameter in Validation Methods

```rust
pub fn validate_payload_len(&self, max: u64) -> bool
pub fn validate_before_alloc(&self, max_payload: u64) -> ValidationResult
```

Should be `MaxPayload(u64)` newtype to prevent units confusion.

### 2.4 `HEADER_LEN` is an Orphaned Constant

```rust
pub const HEADER_LEN: usize = 60;
```

Should be `EnvelopeHeader::HEADER_LEN` or a `HeaderLen(usize)` newtype.

---

## 3. DDD SCOTT WLASCHIN VIOLATIONS

### 3.1 `validate_header_len` is Dead Code (Line 52–54)

```rust
pub fn validate_header_len(&self) -> bool {
    true  // ALWAYS returns true — stub/wrong
}
```

This validation always passes. Either implement it properly or remove it.

### 3.2 All Struct Fields are Public

```rust
pub struct EnvelopeHeader {
    pub magic: u32,          // Encapsulation broken
    pub version: u8,        // Encapsulation broken
    pub kind: u8,            // Encapsulation broken
    pub flags: u8,           // Encapsulation broken
    ...
}
```

DDD principle: **Make illegal states unrepresentable**. Public fields allow unchecked mutation. Use builder pattern or `From`-like constructors.

### 3.3 Validation Logic is Split

- Method on struct: `header.validate_magic()`
- Standalone function: `validate_header_before_alloc(&header, max_payload)`
- Another standalone: `compute_header_crc(_header)`
- Another standalone: `validate_header_crc(_header)`

Should be unified in a `Validator` or `ValidationContext`.

### 3.4 `ValidationError` and `ValidationResult` Should Be in a `validation` Submodule

Error types belong near the validation logic, not in the main module namespace.

---

## 4. TEST BLOAT — 473/579 lines (81.7%)

### File Structure Problem

```
envelope_header.rs
├── [Core + Impl + Enums]  (106 lines actual)
└── tests module           (473 lines noise)
```

### Root Cause

Tests are inline (`#[cfg(test)] mod tests`) rather than in `envelope_header_tests.rs` or `tests/envelope_header_tests.rs`.

### Impact

- File exceeds 300-line limit solely due to tests
- Core domain logic is obscured
- Recompiles tests on every build

---

## 5. STUB FUNCTIONS — NOT IMPLEMENTED

| Function | Status | Line |
|----------|--------|------|
| `compute_header_crc` | Returns `0` always | 98–100 |
| `validate_header_crc` | Returns `true` always | 102–104 |

These are stub implementations, not actual CRC computation/validation.

---

## 6. REFACTORING PLAN

### Phase 1: Split Tests Out
```
envelope_header.rs          (~106 lines)  ← keep core
envelope_header_tests.rs    (~473 lines)  ← move tests here
```

### Phase 2: NewType Wrappers (optional, can be separate bead)
Create newtypes for each primitive field to eliminate primitive obsession.

### Phase 3: Encapsulation
Make fields private, provide constructors and accessors.

---

## 7. SUMMARY

| Issue | Severity |
|-------|----------|
| Line count 579 > 300 | **CRITICAL** |
| 12 primitive obsession violations | **HIGH** |
| Tests inline (81% of file) | **HIGH** |
| Dead code (`validate_header_len`) | **MEDIUM** |
| Stub functions (`compute_header_crc`, `validate_header_crc`) | **MEDIUM** |
| Public struct fields (encapsulation) | **MEDIUM** |

**IMMEDIATE ACTION**: Move `#[cfg(test)]` module to separate `envelope_header_tests.rs` file. This alone fixes the 300-line violation.
