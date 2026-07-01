# Error Taxonomy — vb-1wora

**Bead:** `vb-1wora` — Codec: reject trailing bytes after declared record payload (P1 bug)
**Skill:** `rust-contract` (State 3)
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`

This file updates the storage error taxonomy to include the new `TrailingBytes` variant and describes its relationship to existing variants.

---

## 1. Update at a glance

| Field | Value |
|---|---|
| Variant | `JournalError::TrailingBytes { trailing: usize }` |
| Diagnostic code | `0x4042` (`TRAILING_BYTES_CODE`) |
| Symbolic code | `JOURNAL_TRAILING_BYTES` (recommended registration in `CODE_REGISTRY`) |
| Display | `"trailing bytes after declared payload: {trailing}"` |
| Insertion order | Between `UnexpectedEof` (line 96) and `MalformedKeyspaceRow` (line 101) in `crates/vb_storage/src/error/mod.rs` |
| Reachability | `decode_record_payload`, `decode_record`, `decode_journal_event`, `decode_envelope_only` |
| Mutually exclusive with | `UnexpectedEof` (post-step-2), `PayloadDigestMismatch` (because the check runs first) |

## 2. Detailed variant spec

### 2.1 Source code path

```rust
// crates/vb_storage/src/error/mod.rs (insertion between line 97 and 101)
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    // ... existing variants ...
    #[error("unexpected end of record")]
    UnexpectedEof,
    // === NEW VARIANT BELOW ===
    /// Input slice extends past the declared payload boundary
    /// `RECORD_HEADER_BYTES + header.payload_len` by `trailing` bytes.
    ///
    /// Returned by `decode_record_payload` and `decode_envelope_only`
    /// after the slice `[header.header_len..header.header_len + header.payload_len]`
    /// was successfully extracted, but before any BLAKE3 digest or
    /// postcard deserialization work runs. The decoder fails closed
    /// to reject wire records that have been silently truncated,
    /// appended to, or composed with extra padding.
    #[error("trailing bytes after declared payload: {trailing}")]
    TrailingBytes { trailing: usize },
    // ... existing variants ...
    #[error(
        "malformed keyspace row under prefix {prefix:#04x}: actual_len={actual_len} expected_len={expected_len}"
    )]
    MalformedKeyspaceRow {
        prefix: u8,
        expected_len: usize,
        actual_len: usize,
    },
    // ...
}
```

### 2.2 Field semantics

| Field | Type | Range | Constraint |
|---|---|---|---|
| `trailing` | `usize` | `1..=usize::MAX` | Always `> 0` (post-check); equals `bytes.len() - payload_end` exactly. |

### 2.3 Diagnostic code

```rust
// crates/vb_storage/src/error/codes.rs (insertion near line 49, alongside UNEXPECTED_EOF_CODE)
impl JournalError {
    // ... existing codes ...
    pub const UNEXPECTED_EOF_CODE: DiagnosticCode = DiagnosticCode::new(0x4014);
    /// Diagnostic code for trailing bytes after declared payload
    /// (`JournalError::TrailingBytes`). Distinct from `UNEXPECTED_EOF_CODE`
    /// so callers can tell truncation from extension.
    pub const TRAILING_BYTES_CODE: DiagnosticCode = DiagnosticCode::new(0x4042);
    // ... existing codes ...
}
```

**Numeric choice rationale:** `0x4042` is the next free slot in the `0x40xx` journal range after `0x4041` (`REPLAY_ENVELOPE_SEQUENCE_MISMATCH_CODE`). Verified free against:

- `crates/vb_storage/src/error/codes.rs` (highest used: `0x4041`).
- `crates/vb_core/src/diagnostic.rs::CODE_REGISTRY` (registry stops at `0x4032` per codebase-map.md note; `0x4040`/`0x4041` are journal codes defined but not yet registered symbolically).

### 2.4 Diagnostic-code wiring

```rust
// crates/vb_storage/src/error/codes.rs (insertion in diagnostic_code() match)
impl JournalError {
    pub const fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            // ... existing arms ...
            Self::UnexpectedEof => Self::UNEXPECTED_EOF_CODE,
            // === NEW ARM ===
            Self::TrailingBytes { .. } => Self::TRAILING_BYTES_CODE,
            // ... existing arms ...
            Self::MalformedKeyspaceRow { .. } => Self::MALFORMED_KEYSPACE_ROW_CODE,
            // ...
        }
    }
}
```

### 2.5 Symbolic-code wiring

```rust
// crates/vb_storage/src/error/codes.rs (insertion in symbolic_code() string match)
impl JournalError {
    pub fn symbolic_code(&self) -> SymbolicCode {
        // ... existing prefix ...
        let s: &'static str = match self {
            // ... existing arms ...
            Self::UnexpectedEof => "UNEXPECTED_EOF",
            // === NEW ARM ===
            Self::TrailingBytes { .. } => "JOURNAL_TRAILING_BYTES",
            // ... existing arms ...
        };
        if let Some(code) = SymbolicCode::from_static(s) {
            return code;
        }
        SymbolicCode::INTERNAL_INVARIANT
    }
}
```

If `JOURNAL_TRAILING_BYTES` is **not** registered in `crates/vb_core/src/diagnostic.rs::CODE_REGISTRY`, the existing fallback returns `SymbolicCode::INTERNAL_INVARIANT`. This is acceptable but degrades observability; registration is recommended.

### 2.6 Recommended symbolic-name registration (out-of-Codec-only, vb_core change)

```rust
// crates/vb_core/src/diagnostic.rs (append to CODE_REGISTRY slice, near line 1583)
("JOURNAL_TRAILING_BYTES", DiagnosticCode::new(0x4042)),
```

This is a one-line addition that promotes the symbolic name from "in-string only" to "registered". Failure to register is non-blocking but produces less informative error messages.

## 3. Taxonomy position: where does `TrailingBytes` belong?

### 3.1 Shape-defect bucket

The codec decode pipeline already groups three shape defects:

| Variant | Wire shape defect | Direction |
|---|---|---|
| `UnexpectedEof` | bytes < payload boundary | too few |
| **`TrailingBytes` (NEW)** | **bytes > payload boundary** | **too many** |
| `MalformedKeyspaceRow` | row under prefix has wrong length | mis-shaped row |

The three variants together cover the three directions of "the bytes I have don't match the bytes I expected." They are mutually exclusive for any given `(bytes, parsed_header)` triple.

### 3.2 Relationship to integrity defects

| Variant | Class | Cause |
|---|---|---|
| `PayloadDigestMismatch` | integrity (BLAKE3) | digest ≠ hash(payload) |
| `HeaderChecksumMismatch` | integrity (CRC32C) | crc ≠ crc32c(header_prefix) |

`TrailingBytes` is **not** an integrity defect: the bytes inside `[0..payload_end]` may be perfectly intact. It is a shape defect about the bytes *outside* the declared boundary. The taxonomy cleanly separates "is the data right?" (integrity) from "did the boundary agree?" (shape).

### 3.3 Relationship to deserialization defects

| Variant | Cause |
|---|---|
| `PostcardDecodeFailed` | postcard failed to deserialize the bounded payload. |
| `RecordKindPayloadMismatch` | envelope kind ≠ payload kind. |
| `InvalidEvent` | payload decoded but `JournalEvent::is_valid()` returned false. |

`TrailingBytes` is unreachable together with any of these: it fires earlier in the pipeline.

## 4. Conversion paths

### 4.1 `From<TrimError>` (existing)

`TrimError::Journal(JournalError)` already exists (see `crates/vb_storage/src/error/codes.rs:159-167`). The new variant flows through this path automatically — no `From` impl change needed.

### 4.2 `From<fjall::Error>` (existing)

Unaffected. `TrailingBytes` is a codec-layer error; it does not arise from a Fjall operation.

### 4.3 `From<postcard::Error>` (existing)

Unaffected. The variant fires *before* `postcard::from_bytes`.

## 5. Operator-facing messages

| Surface | Output |
|---|---|
| `format!("{}", err)` | `"trailing bytes after declared payload: 3"` |
| `err.diagnostic_code()` | `DiagnosticCode(0x4042)` |
| `err.diagnostic_code().0` | `0x4042` |
| `err.symbolic_code()` | `SymbolicCode::JOURNAL_TRAILING_BYTES` (or `INTERNAL_INVARIANT` if not registered) |
| JSON serialization (if added) | `{"kind": "TrailingBytes", "trailing": 3}` — depends on existing pattern; not in scope for this bead |

Operators triaging a `TrailingBytes` error learn:

1. The wire record was longer than the header declared (fail-closed).
2. Exactly how many bytes were extra (`trailing`).
3. The diagnostic code (`0x4042`) and (if registered) symbolic name (`JOURNAL_TRAILING_BYTES`) for log correlation.

## 6. Test-audit update

**Path:** `crates/vb_storage/src/error_tests.rs:14-62` (audit comment header).

Pre-fix:
```text
// Untested variants:
// - UnexpectedEof: no direct test
```

Post-fix:
```text
// Tested variants:
// - UnexpectedEof: (still untested for direct variant; covered indirectly via decode tests)
// - TrailingBytes: trailing_bytes_variant_and_fields, trailing_bytes_display_format, trailing_bytes_error_code
```

This is a documentation update; no Rust code change. The pre-fix `Untested variants` list remains accurate for the other variants it listed; only the `TrailingBytes` line is added to `Tested variants:`.

## 7. Test surface for the new variant

The trio pattern from `InvalidGateCount` (lines 478-511 of `error_tests.rs`) is mirrored for `TrailingBytes`:

| Test function | Asserts |
|---|---|
| `trailing_bytes_variant_and_fields` | `matches!(err, TrailingBytes { trailing: 5 })` and field round-trip. |
| `trailing_bytes_display_format` | Display contains "trailing" and `5`. |
| `trailing_bytes_error_code` | `err.diagnostic_code() == TRAILING_BYTES_CODE` and `TRAILING_BYTES_CODE == DiagnosticCode::new(0x4042)`. |

Plus a sibling in `error_code_tests.rs`:

| Test function | Asserts |
|---|---|
| `trailing_bytes_error_has_correct_code` | `TrailingBytes { trailing: 100 }.diagnostic_code() == TRAILING_BYTES_CODE`. |

## 8. Mirror surface (Verus)

### 8.1 `SpecJournalError::TrailingBytes` (mirror)

**Path:** `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:335-413`.

```rust
pub enum SpecJournalError {
    // ... existing variants ...
    UnexpectedEof,
    /// Mirror of `JournalError::TrailingBytes { trailing }` at
    /// error/mod.rs:NN. Returned by `decode_record_payload`
    /// at codec/payload.rs:NN when `bytes.len() > payload_end`.
    /// `trailing_u32 == (bytes.len() - payload_end) as u32` and
    /// `trailing_u32 > 0`.
    TrailingBytes { trailing: u32 },
    // ... existing variants ...
}
```

**Shape note:** `u32` (not `usize`) because Verus byte-count models are bounded by `u32::MAX`. The production `usize` is cast/derived at the bridge boundary.

### 8.2 Comment update

**Path:** `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:280-327`.

Add under "Variants retained from the production enum":

```text
//   * TrailingBytes             -> decode_record_payload shape check
//                                  (codec/payload.rs:NN-NN) when
//                                  bytes.len() > payload_end. The
//                                  check is BEFORE verify_digest_match
//                                  and BEFORE postcard::from_bytes.
//                                  Also reachable from decode_envelope_only
//                                  (codec/envelope.rs:NN-NN) via the same
//                                  mirror check.
```

### 8.3 Bridge `ensures` update

**Path:** `verification/verus/vb-vzcuf-PS-003.rs:387-451`.

Add one arm to the `decode_record` `assume_specification[ ... ]` `match`:

```rust
Err(SpecJournalError::TrailingBytes { trailing }) => {
    &&& (bytes.len() as u32) > expected_payload_end
    &&& trailing == (bytes.len() as u32) - expected_payload_end
    &&& trailing > 0
},
```

The `expected_payload_end` parameter is a new bridge argument (or can be derived from `decoded_envelope` if `SpecRecordEnvelope` is extended to carry `payload_len`; see type-contracts.md §4.3).

## 9. Drift gate expectations

| Gate | Pass condition |
|---|---|
| `scripts/check-production-inner-drift.sh` | `SpecJournalError::TrailingBytes` exists in mirror with shape `u32`. |
| `scripts/check-verus-production-binding.sh` | Bridge `ensures` enumerates `Err(SpecJournalError::TrailingBytes { .. })`. |
| `bash scripts/verify-verus.sh` | Verus spec compiles with the new variant. |
| `cargo fmt --check` | No fmt drift. |
| `cargo clippy -- -D warnings` (source lint) | No new lints. |
| `cargo test -p vb_storage` | All tests pass with `TrailingBytes` variant. |

## 10. Forbidden patterns in error-taxonomy work

- Two distinct `JournalError` variants that both fire on `bytes.len() > payload_end`. Only `TrailingBytes` does.
- Re-using `0x4042` for a different variant.
- Re-using the symbolic name `JOURNAL_TRAILING_BYTES` for a different code.
- Wrapping `TrailingBytes` inside `Box<JournalError>` at the producer site (production uses it inline; only `CompiledIrReadback` boxes, and that's a different concern).

---

## Summary

One new variant, one new diagnostic-code pair, three wiring sites (the `#[error]` attribute, `diagnostic_code()`, `symbolic_code()`), one Verus mirror variant, one Verus bridge arm, one Verus comment update. The new variant slots cleanly into the existing shape-defect bucket alongside `UnexpectedEof` and `MalformedKeyspaceRow`.