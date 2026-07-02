# Type Contracts — vb-1wora

**Bead:** `vb-1wora` — Codec: reject trailing bytes after declared record payload (P1 bug)
**Skill:** `rust-contract` (State 3)
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`

This file specifies the new and changed **types** in the v1 storage record codec. It is implementation-agnostic where possible; it does not write Rust code or tests.

---

## 1. New variant: `JournalError::TrailingBytes`

### 1.1 Signature (Rust shape, contract-only)

```rust
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    // ... existing variants ...
    /// The input slice extends past the declared payload boundary
    /// `RECORD_HEADER_BYTES + header.payload_len` by `trailing` bytes.
    ///
    /// `trailing == bytes.len() - payload_end` and `trailing > 0`.
    /// The decoder must fail closed before any further expensive work
    /// (BLAKE3 digest, postcard decode) when this shape defect is observed.
    #[error("trailing bytes after declared payload: {trailing}")]
    TrailingBytes { trailing: usize },
    // ... existing variants ...
}
```

### 1.2 Field semantics

| Field | Type | Range | Meaning |
|---|---|---|---|
| `trailing` | `usize` | `1..=usize::MAX` (in practice bounded by `bytes.len() - payload_end`, which is bounded by the kernel page size at the storage layer; verifier models may use a smaller bound) | Number of bytes present in the input slice beyond `payload_end`. Never zero for a `TrailingBytes` arm of the decoder — zero would mean the slice is exactly bounded, which is the success case. |

### 1.3 Ordering within `JournalError`

The variant is inserted **between `UnexpectedEof` (error/mod.rs:96-97) and `MalformedKeyspaceRow` (error/mod.rs:97-105)** so all shape-defect variants stay grouped:

- `UnexpectedEof` — too few bytes (`bytes.len() < payload_end`).
- `TrailingBytes` — too many bytes (`bytes.len() > payload_end`). **NEW**
- `MalformedKeyspaceRow` — length mismatch in a stored row under a known prefix.

This order is documentation-only at the Rust source level (Rust does not enforce enum-variant ordering for behavior), but it matters for:

- The drift gate's enumeration comment at `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:280-327` (must be updated in lockstep).
- Reader clarity during code review.

### 1.4 Diagnostic and symbolic code pairing

```rust
impl JournalError {
    /// Diagnostic code for trailing bytes after declared payload.
    pub const TRAILING_BYTES_CODE: DiagnosticCode = DiagnosticCode::new(0x4042);
    // ... existing CODE constants ...
}
```

- Numeric value `0x4042` is the next free slot in the `0x40xx` journal range. Verified free in `crates/vb_storage/src/error/codes.rs` (highest used: `0x4041`) and in `crates/vb_core/src/diagnostic.rs::CODE_REGISTRY` (registry stops at `0x4032`, but symbolic registration is optional).
- Symbolic name: `JOURNAL_TRAILING_BYTES` (recommended registration in `CODE_REGISTRY`).
- Wiring sites (see `crates/vb_storage/src/error/codes.rs`):
  - `pub const TRAILING_BYTES_CODE: DiagnosticCode = DiagnosticCode::new(0x4042);` at line ~50 (next to `UNEXPECTED_EOF_CODE`).
  - `Self::TrailingBytes { .. } => Self::TRAILING_BYTES_CODE,` added to `diagnostic_code()` `match` arms.
  - `Self::TrailingBytes { .. } => "JOURNAL_TRAILING_BYTES",` added to `symbolic_code()` string `match` arm. If the symbolic name is not registered in `CODE_REGISTRY`, the existing fallback returns `SymbolicCode::INTERNAL_INVARIANT` (no production behavior change).

### 1.5 Conversion / boundary obligations

- `JournalError::From<T>` for `T = JournalError` is trivially satisfied (identity).
- The variant flows through existing `From<JournalError>` for `TrimError::Journal(inner)` — no new wiring needed; `diagnostic_code()` and `symbolic_code()` already delegate to the inner error.
- No new `Display` work beyond the `#[error(...)]` attribute.
- No new `std::error::Error::source()` wiring (unit-of-error is itself).

## 2. New shape constraint on existing decoders

### 2.1 `decode_record_payload` (canonical site)

**Path:** `crates/vb_storage/src/codec/payload.rs:56-82`.

**Pre-fix return contract:**

```text
Result<(RecordEnvelope, &[u8]), JournalError>
```

**Post-fix return contract (additional clause):**

For all `bytes`, `expected_magic`, `max_payload_len`:

```text
let payload_end = header.header_len as usize + header.payload_len as usize;
match result {
    Ok((_env, payload)) => bytes.len() == payload_end
                          && payload.len() == header.payload_len as usize,
    Err(JournalError::TrailingBytes { trailing }) => {
        trailing == bytes.len() - payload_end
        && trailing > 0
        // Also: header is well-formed enough to compute payload_end;
        // i.e. the earlier slice bounds error did not fire first.
        && payload_start <= payload_end
        && payload_end <= bytes.len()  // required for the trailing arithmetic
    },
    Err(_) => /* existing failure modes, unchanged */,
}
```

The new clause slots between `bytes.get(payload_start..payload_end)` and `verify_digest_match(payload, ...)`:

```text
let payload = bytes.get(payload_start..payload_end)
    .ok_or(JournalError::UnexpectedEof)?;
// NEW: trailing-bytes check
if bytes.len() > payload_end {
    return Err(JournalError::TrailingBytes {
        trailing: bytes.len() - payload_end,
    });
}
verify_digest_match(payload, header.payload_digest)?;
```

### 2.2 `decode_envelope_only` (mirror site)

**Path:** `crates/vb_storage/src/codec/envelope.rs:48-83`.

Identical trailing-bytes check is added between the `bytes.get` call and `verify_digest_match`:

```text
let raw_payload = bytes.get(payload_start..payload_end)
    .ok_or(JournalError::UnexpectedEof)?;
// NEW: trailing-bytes check (mirror of decode_record_payload)
if bytes.len() > payload_end {
    return Err(JournalError::TrailingBytes {
        trailing: bytes.len() - payload_end,
    });
}
verify_digest_match(raw_payload, header.payload_digest)?;
```

**Rationale:** `decode_envelope_only` is `pub(crate)` with `#[allow(dead_code, reason = "inspection-only entry point retained for doctor/filtering workflows")]`. Its docstring claims to perform envelope + payload validation; without the trailing-bytes check, the claim is false. The mirror maintains the invariant `INV-CODEC-TB-004`.

### 2.3 Callers (no signature change)

| Caller | Path | Notes |
|---|---|---|
| `decode_record<T>` | `crates/vb_storage/src/codec/mod.rs:82-95` | Inherits the new failure mode via delegation to `decode_record_payload`. |
| `decode_journal_event` | `crates/vb_storage/src/codec/mod.rs:126-151` | Inherits via `decode_record`. |
| `has_terminal_event` | `crates/vb_storage/src/trimming/logic.rs:251` | Reads Fjall keyspace values. After the fix, a corrupted row yields `Err(TrimError::Journal(TrailingBytes))` instead of silently decoding and dropping the tail. Loop continues via `?` propagation. **No signature change.** |

## 3. New test contract (test-plan input only, not a test file)

Three test functions are required to lock the regression. They live in existing test files; this contract names them and specifies their assertions. The test-writer owns the implementation.

### 3.1 `decode_rejects_trailing_bytes_after_payload`

- **Path:** `crates/vb_storage/src/codec/tests.rs:1498-1524` (replaces `decode_ignores_trailing_bytes_beyond_payload`).
- **Fixture (unchanged from pre-fix):** valid `JournalEvent::RunCancelled` record + 3 appended bytes `0xFF 0xFE 0xFD`.
- **Assertion:** `assert!(matches!(result, Err(JournalError::TrailingBytes { trailing: 3 })))`.
- **Count field locked at `trailing: 3`** so any future refactor that breaks the byte count is caught.

### 3.2 `decode_envelope_only_rejects_trailing_payload`

- **Path:** `crates/vb_storage/src/codec/envelope.rs:153-170` (sibling of `decode_envelope_only_rejects_truncated_payload`).
- **Fixture:** valid record + 4 appended bytes (different count than 3.1 to lock the field's dynamic value).
- **Assertion:** `assert!(matches!(result, Err(JournalError::TrailingBytes { trailing: 4 })))`.

### 3.3 Error variant trio (`trailing_bytes_variant_and_display`, `trailing_bytes_error_code`, …)

- **Path:** `crates/vb_storage/src/error_tests.rs` (new section, mirrors `InvalidGateCount` at lines 478-511).
- **Three-test pattern:** variant+fields / display_format / error_code.
- **Display assertion:** contains "trailing" and the byte count.
- **Code assertion:** `TRAILING_BYTES_CODE == DiagnosticCode::new(0x4042)` and `TrailingBytes { trailing: 5 }.diagnostic_code() == TRAILING_BYTES_CODE`.

### 3.4 Diagnostic-code registration test (`trailing_bytes_code_is_correct`)

- **Path:** `crates/vb_storage/src/error_code_tests.rs` (mirrors `payload_too_large_error_has_correct_code` at lines 144-151).

### 3.5 Audit header update

- **Path:** `crates/vb_storage/src/error_tests.rs:14-62`.
- **Change:** move `TrailingBytes` from the `Untested variants:` block to the `Tested variants:` block in the comment header.

## 4. Verus mirror (production-binding contract)

### 4.1 New variant on `SpecJournalError`

**Path:** `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:335-413`.

```rust
pub enum SpecJournalError {
    // ... existing variants ...
    /// Mirror of `JournalError::TrailingBytes { trailing }` at
    /// error/mod.rs:NN-NN (NEW). Returned by `decode_record_payload`
    /// at codec/payload.rs:NN when `bytes.len() > payload_end`.
    /// `trailing_u32 == (bytes.len() - payload_end) as u32`,
    /// bounded by `bytes.len() <= u32::MAX as usize` for the
    /// verifier model.
    TrailingBytes { trailing: u32 },
    // ... existing variants ...
}
```

**Shape rationale:** `u32` not `usize` because Verus spec values for byte counts are bounded by `u32::MAX` (matching `header.payload_len`'s concrete type). The production `usize` field is cast/derived at the boundary.

### 4.2 Updated enumeration comment

**Path:** `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:280-327`.

Add one bullet under "Variants retained from the production enum":

```text
//   * TrailingBytes             -> decode_record_payload shape check
//                                  (codec/payload.rs:NN-NN) when
//                                  bytes.len() > payload_end. The
//                                  check is BEFORE verify_digest_match
//                                  and BEFORE postcard::from_bytes.
```

### 4.3 Updated bridge `ensures`

**Path:** `verification/verus/vb-vzcuf-PS-003.rs:387-451`.

Add one arm to the `decode_record` `assume_specification[ ... ]` `match`:

```rust
Err(SpecJournalError::TrailingBytes { trailing }) => {
    &&& !header_ok || (trailing > 0 && bytes.len() as u32 > expected_payload_end)
},
```

The `expected_payload_end` is a new bridge parameter (or a derivation from `decoded_envelope.payload_len` if that field is added to `SpecRecordEnvelope`; if not, the planner can hoist it as a top-level bridge parameter mirroring `header_ok`).

### 4.4 Extern shim

**Path:** `verification/verus/extern_vb_vzcuf_PS_003.rs:83-87`.

No code change required — the shim re-exports `SpecJournalError` and picks up new variants automatically. The drift gate (`scripts/check-production-inner-drift.sh`) re-checks parity.

## 5. Type-state / typestate considerations

None. The codec functions are not state machines; they are pure parsers over `&[u8]`. There is no "decoding" vs "decoded" state to encode typestatically. The new invariant is purely shape-based and is enforced at the function level.

## 6. Newtype discipline

Already followed. The `JournalError` enum, `DiagnosticCode`, `SymbolicCode`, `RecordEnvelope`, and `RecordHeader` are all newtypes/wrappers. No new newtype is needed; the `trailing: usize` field is a primitive on the variant because it is byte-count metadata.

## 7. Idempotence and parse-once-at-boundary

- The trailing-bytes check is **idempotent**: running the check twice on the same `bytes` yields the same result.
- The check is **pure**: no I/O, no allocation beyond the `usize` subtraction.
- The check is **boundary-local**: it does not run deeper in the pipeline. After the check passes (no trailing), the rest of the decode proceeds unchanged.

## 8. Forbidden patterns (linter-enforceable)

The following patterns must NOT appear in the post-fix codec surface:

- `bytes.get(payload_start..payload_end)?;` followed directly by `verify_digest_match(...)` without an intervening `if bytes.len() > payload_end { ... }` check. (Lint-able via call-site review or a clippy lint if the workspace adopts one; for now this is structural review.)
- New `JournalError` variants in `error/mod.rs` that lack a matching `diagnostic_code()` arm.
- Numeric codes outside the `0x40xx` journal range for storage-layer errors.
- Use of `unreachable!()`, `panic!()`, `unwrap()`, `expect()`, `todo!()`, or `dbg!()` in the post-fix path. (Existing AGENTS.md doctrine.)

---

## Summary

Two function signatures gain a behavioral clause (no signature change), one enum gains one variant, one diagnostic-code pair is added, three Verus artifacts gain a single arm each, and three test functions are specified. The surface is intentionally minimal: this is a fix, not a refactor.