# Architectural Drift Report: `vb_ipc/src/frame.rs`

## Summary
| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total Lines | **1540** | <300 | **FAIL** (5.1x over limit) |
| Production Code | ~156 lines | - | - |
| Test Code | ~1382 lines | - | - |
| Test Ratio | 89.7% | - | SMELL |

---

## 1. Line Count Violation

**Status: CRITICAL FAILURE**
- File: `crates/vb_ipc/src/frame.rs`
- Total: **1540 lines** (limit: 300)
- Excess: **1240 lines over threshold**

### Breakdown by Section
| Section | Lines | Type |
|---------|-------|------|
| Module doc + imports | 1-8 | setup |
| Public API functions | 9-155 | production |
| `mod tests` block | 157-1540 | tests |
| Helper macros (`assert_ok!`) | 161-168 | test util |
| Helper fns (`assert_command_roundtrip`, etc.) | 170-228 | test util |
| Individual test functions | 230-1540 | tests |

---

## 2. DDD Cohesion Analysis

### Filename: `frame.rs`
**Domain Concept Claimed**: IPC frame encoding/decoding

**Actual Contents (5 concerns mixed)**:
1. **Frame encoding** (`encode_frame`) - lines 10-27
2. **Frame header decoding** (`decode_frame_header`) - lines 30-32
3. **Frame payload decoding** (`decode_frame_payload`) - lines 35-51
4. **Frame validation** (`validate_frame_magic`, `validate_frame_bounds`) - lines 54-85
5. **Frame I/O** (`read_frame_header`, `read_frame_payload`, `write_frame`) - lines 88-148

### DDD Smell: **YES** - Feature-ized cohesion

The file violates the Single Responsibility Principle by bundling:
- **Codec** (encoding/decoding)
- **Validation** (magic, bounds)
- **I/O** (read/write from streams)

These are three distinct domain operations that should be in separate modules:
- `frame_codec.rs` - encoding/decoding
- `frame_validate.rs` - validation only  
- `frame_io.rs` - I/O operations

---

## 3. All Violations

### 3.1 Oversized File (CRITICAL)
- **Violation**: 1540 lines vs 300 line limit
- **Location**: Entire file
- **Remediation**: Split into `frame_codec.rs`, `frame_validate.rs`, `frame_io.rs`

### 3.2 Inline Tests Module (MAJOR)
- **Violation**: 1382 lines of tests in `#[cfg(test)] mod tests`
- **Location**: Lines 157-1540
- **Problem**: Tests are 6.4x larger than production code
- **Remediation**: Move to `tests/frame_tests.rs` at crate level

### 3.3 Test Helper Bloat (MINOR)
- **Violations**:
  - `assert_ok!` macro (lines 161-168) - duplicated match logic across 100+ tests
  - `assert_command_roundtrip()` (lines 170-188) - 19-line helper with redundant unwrapping
  - `assert_payload_roundtrip()` (lines 190-216) - 27-line helper with redundant unwrapping
  - `assert_bad_magic_rejected()` (lines 218-228) - 11-line helper
- **Problem**: Each helper reinvents error assertion instead of using std/conventional patterns
- **Remediation**: Use `assert!(result.is_ok())` or `pretty_assertions` crate

### 3.4 Module Separation Failure (MAJOR)
- **Violation**: 3 domain concepts in 1 file
- **Problem**: `frame.rs` mixes codec, validation, and I/O
- **Remediation**: 
  ```
  frame/
    mod.rs       # Re-exports
    codec.rs     # encode_frame, decode_frame_header, decode_frame_payload
    validate.rs  # validate_frame_magic, validate_frame_bounds
    io.rs        # read_frame_header, read_frame_payload, write_frame
  ```

### 3.5 No Zero-Cost Abstraction Boundary (MINOR)
- **Violation**: Internal helper `payload_len_u32()` (line 150-155) is private but used only by `encode_frame`
- **Problem**: Leaky abstraction - `encode_frame` could inline this logic
- **Remediation**: Keep as-is or make `encode_frame` a thin wrapper in `frame_codec.rs`

---

## 4. Specific Line Counts

```
Production code:     156 lines  (lines 1-156)
Test code:          1382 lines  (lines 157-1540)
Test helpers:         ~70 lines  (161-228)
Individual tests:   ~1312 lines  (230 tests × ~5.7 lines avg)
```

### Public API Functions (9 functions, ~130 lines)
| Function | Lines | LOC |
|----------|-------|-----|
| `encode_frame` | 10-27 | 18 |
| `decode_frame_header` | 30-32 | 3 |
| `decode_frame_payload` | 35-51 | 17 |
| `validate_frame_magic` | 54-66 | 13 |
| `validate_frame_bounds` | 69-85 | 17 |
| `read_frame_header` | 88-94 | 7 |
| `read_frame_header_bounded` | 97-106 | 10 |
| `read_frame_payload` | 109-123 | 15 |
| `read_frame_payload_bounded` | 126-133 | 8 |
| `write_frame` | 136-148 | 13 |
| `payload_len_u32` (private helper) | 150-155 | 6 |

---

## 5. DDD Smell Detected

**YES** - `frame.rs` exhibits **Feature-ized Cohesion** anti-pattern:

- **Symptom**: One file does "everything related to frames"
- **Root Cause**: IPC framing was implemented as a single module rather than decomposed by responsibility
- **Impact**: Harder to test individually, impossible to use validation without codec, I/O couples to std::io

---

## 6. Remediation Priority

| Priority | Violation | Effort | Impact |
|----------|-----------|--------|--------|
| **P0 - CRITICAL** | Split file into `frame_codec.rs`, `frame_validate.rs`, `frame_io.rs` | High | Enables parallel work, clears 300L gate |
| **P0 - CRITICAL** | Move tests to `tests/frame_tests.rs` | Medium | Removes 1382 lines from production crate |
| **P1 - HIGH** | Extract test helpers to `tests/frame_tests/helpers.rs` or use `pretty_assertions` | Low | Cleaner test code |
| **P2 - MEDIUM** | Update `mod.rs` to re-export from new modules | Medium | Maintain API compatibility |

---

## 7. Recommended Module Structure

```
crates/vb_ipc/src/
├── frame/
│   ├── mod.rs          # Re-exports: encode_frame, decode_frame_header, etc.
│   ├── codec.rs        # ~60 lines: encode, decode, payload_len_u32
│   ├── validate.rs     # ~40 lines: magic, bounds validation
│   └── io.rs           # ~50 lines: read/write from streams
├── tests/
│   └── frame_tests.rs  # ~1380 lines: all frame tests
```

---

## 8. Next Actions

1. **Create `crates/vb_ipc/src/frame/` directory**
2. **Move codec logic to `frame/codec.rs`** (~60 lines)
3. **Move validation logic to `frame/validate.rs`** (~40 lines)  
4. **Move I/O logic to `frame/io.rs`** (~50 lines)
5. **Create `frame/mod.rs`** with re-exports
6. **Move all tests to `tests/frame_tests.rs`** (~1380 lines)
7. **Update `lib.rs`** to use new module path
8. **Verify `moon ci` passes**

---

*Report generated by architectural-drift agent*
