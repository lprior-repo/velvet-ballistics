# Architectural Drift Report: vb_storage codec module

**Analyzed Path:** `crates/vb_storage/src/codec/`
**Report Date:** 2026-05-29
**Status:** REFACTORED (CRITICAL)

---

## 1. Line Count Analysis

| File | Lines | Limit (300) | Status |
|------|-------|-------------|--------|
| header.rs | 104 | ✓ | PASS |
| mod.rs | 94 | ✓ | PASS |
| payload.rs | 82 | ✓ | PASS |
| validation.rs | 57 | ✓ | PASS |
| tests.rs | **2557** | ✗ | **FAIL** |
| **Total** | **2894** | — | — |

**VIOLATION:** `tests.rs` at 2557 lines **exceeds the 300-line hard limit by 2257 lines (851% over)**.

---

## 2. DDD Cohesion Analysis

### Module Domain: Binary Encoding/Decoding

The codec module is responsible for **encoding and decoding journal records** with:
- A 60-byte storage envelope (header)
- Postcard serialization for payloads
- CRC32C header integrity
- BLAKE3 payload digest

### Submodule Breakdown

| Submodule | Responsibility | Cohesion |
|-----------|---------------|----------|
| `header.rs` | Record header encode/decode | ✓ High |
| `payload.rs` | Payload encoding/decoding | ✓ High |
| `validation.rs` | Schema/kind/magic validation | ✓ High |
| `mod.rs` | Public API (encode/decode) | ✓ High |

### DDD Smells Detected

#### SMELL #1: Primitive Obsession (MINOR)
Raw integers used where newtypes exist elsewhere:
- `u32` for `magic` — should be `Magic` wrapper
- `u64` for `sequence` — already has `EventSeq` but used as raw `u64` in `encode_record_header`
- `u16` for `record_kind` — already has `RecordKind` enum

**Location:** `header.rs:14-19` and throughout
```rust
pub fn encode_record_header(
    magic: u32,       // Primitive obsession
    kind: RecordKind,
    sequence: u64,    // Should use EventSeq
    payload: &[u8],
    max_payload_len: u32,
)
```

#### SMELL #2: Workflow Leakage (MINOR)
`next_seq` and `validate_replayed_event` in `mod.rs` are **workflow functions**, not codec functions:
- They belong in a replay/recovery module, not codec

**Location:** `mod.rs:66-91`
```rust
pub(crate) fn next_seq(seq: EventSeq) -> Result<EventSeq, JournalError>
pub(crate) fn validate_replayed_event(run, expected, event) -> Result<(), JournalError>
```

---

## 3. Violations Summary

| ID | Severity | Violation | Location |
|----|----------|-----------|----------|
| V1 | **CRITICAL** | File exceeds 300 lines (2557) | `tests.rs` |
| V2 | MINOR | Primitive obsession | `header.rs:14-19` |
| V3 | MINOR | Workflow function leakage | `mod.rs:66-91` |

---

## 4. Priority & Recommendation

### Priority: **P0 — CRITICAL**

`tests.rs` at 2557 lines is an architectural gravity well. It must be split.

### Required Actions

1. **Split `tests.rs`** into test modules per feature:
   - `tests_journal_events.rs` — JournalEvent roundtrip tests
   - `tests_header.rs` — Header encode/decode tests
   - `tests_validation.rs` — Validation edge case tests
   - `tests_integration.rs` — Cross-component integration tests

2. **Relocate workflow functions** (`next_seq`, `validate_replayed_event`) to `vb_storage::recovery` module.

3. **Consider newtypes for header primitives** (low priority, can be tracked separately).

---

## 5. Files Affected

```
codec/
├── mod.rs          (94 lines) — OK, minor refactor needed
├── header.rs       (104 lines) — OK
├── payload.rs      (82 lines) — OK
├── validation.rs   (57 lines) — OK
└── tests.rs        (2557 lines) — MUST SPLIT
```

---

**END REPORT**
