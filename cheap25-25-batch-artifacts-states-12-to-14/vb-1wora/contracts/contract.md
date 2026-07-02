# Contract — vb-1wora

**Bead:** `vb-1wora` — Codec: reject trailing bytes after declared record payload (P1 bug)
**Skill:** `rust-contract` (State 3)
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`

This is the master contract. It synthesizes the domain model, type contracts, workflow model, error taxonomy, boundary map, and hazard analysis into a single normative specification. Downstream agents (proof-planner, proof-writer, test-writer, implementation) MUST satisfy the obligations named here.

---

## 1. Bead identity

| Field | Value |
|---|---|
| Bead ID | `vb-1wora` |
| Title | Codec: reject trailing bytes after declared record payload (P1 bug) |
| State | 3 (rust-contract) |
| Workspace | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora` |
| JJ root | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora` |
| JJ workspace | `cheap25-vb-1wora` |
| Parent commit | `rsvywymk 1d6c017f (AGENTS.md round10 forward-port)` |
| Skill | `rust-contract` |
| Authority | `velvet-ballistics-MASTER.md` (canonical build plan) |

## 2. Problem statement

The v1 storage record codec in `vb_storage` decodes an envelope (60-byte header) plus a payload, but does **not** verify that the input slice ends exactly at the declared payload boundary. After the fix succeeds, the decode MUST fail closed with a new `TrailingBytes` variant when `bytes.len() > payload_end`.

**Pre-fix bug:**

```rust
// crates/vb_storage/src/codec/payload.rs:56-82
let payload = bytes
    .get(payload_start..payload_end)
    .ok_or(JournalError::UnexpectedEof)?;
verify_digest_match(payload, header.payload_digest)?;  // BUG: no check on bytes.len() > payload_end
Ok((RecordEnvelope { ... }, payload))
```

A Fjall keyspace value of `header || payload || junk` decodes as if it were `header || payload`, silently dropping the tail.

## 3. Goal

Insert a single shape check in the canonical decode pipeline so that any input whose slice extends past the declared payload boundary fails closed with a new `JournalError::TrailingBytes { trailing }` variant. Apply the same check to the mirror site `decode_envelope_only`. Bind the new variant to the Verus mirror and bridge contract.

## 4. Scope

### 4.1 In scope

| Item | Path | Change |
|---|---|---|
| Trailing-bytes check (canonical) | `crates/vb_storage/src/codec/payload.rs:56-82` | Add one `if bytes.len() > payload_end { return Err(JournalError::TrailingBytes { trailing: ... }); }` block between the `bytes.get` call and `verify_digest_match`. |
| Trailing-bytes check (mirror) | `crates/vb_storage/src/codec/envelope.rs:48-83` | Add the same block between the `bytes.get` call and `verify_digest_match`. |
| `JournalError::TrailingBytes` variant | `crates/vb_storage/src/error/mod.rs:97` (insertion after `UnexpectedEof`) | Add `#[error("trailing bytes after declared payload: {trailing}")] TrailingBytes { trailing: usize }`. |
| Diagnostic code `TRAILING_BYTES_CODE = 0x4042` | `crates/vb_storage/src/error/codes.rs:49` (insertion near `UNEXPECTED_EOF_CODE`) | Add `pub const TRAILING_BYTES_CODE: DiagnosticCode = DiagnosticCode::new(0x4042);`. |
| `diagnostic_code()` arm | `crates/vb_storage/src/error/codes.rs:99-176` | Add `Self::TrailingBytes { .. } => Self::TRAILING_BYTES_CODE,`. |
| `symbolic_code()` arm | `crates/vb_storage/src/error/codes.rs:180-268` | Add `Self::TrailingBytes { .. } => "JOURNAL_TRAILING_BYTES",`. |
| Test inversion | `crates/vb_storage/src/codec/tests.rs:1498-1524` | Rename `decode_ignores_trailing_bytes_beyond_payload` to `decode_rejects_trailing_bytes_after_payload` (or similar). Replace `Ok` assertion with `Err(JournalError::TrailingBytes { trailing: 3 })` assertion. Keep the 3-byte `0xFF 0xFE 0xFD` fixture. |
| Mirror test (envelope.rs) | `crates/vb_storage/src/codec/envelope.rs:153-170` (sibling of `decode_envelope_only_rejects_truncated_payload`) | Add `decode_envelope_only_rejects_trailing_payload` test asserting `Err(JournalError::TrailingBytes { trailing: 4 })` on a record with 4 appended bytes. |
| Error variant trio | `crates/vb_storage/src/error_tests.rs` (new section after `MissingRequiredProofFlag` lines 513-557) | Add `trailing_bytes_variant_and_fields`, `trailing_bytes_display_format`, `trailing_bytes_error_code`. Mirror `InvalidGateCount` pattern. |
| Diagnostic-code registration test | `crates/vb_storage/src/error_code_tests.rs` (after `payload_too_large_error_has_correct_code` lines 144-151) | Add `trailing_bytes_error_has_correct_code`. |
| Audit header | `crates/vb_storage/src/error_tests.rs:14-62` | Move `TrailingBytes` from `Untested variants:` to `Tested variants:` block. |
| Verus mirror variant | `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:335-413` | Add `SpecJournalError::TrailingBytes { trailing: u32 }` to the enum. Update enumeration comment at lines 280-327 to include the new variant. |
| Verus bridge arm | `verification/verus/vb-vzcuf-PS-003.rs:387-451` | Add `Err(SpecJournalError::TrailingBytes { trailing }) => { ... }` arm to the `decode_record` `assume_specification` `ensures` clause. |

### 4.2 Recommended (not mandatory)

| Item | Path | Notes |
|---|---|---|
| Symbolic-code registration | `crates/vb_core/src/diagnostic.rs` (slice ending around line 1583) | Add `("JOURNAL_TRAILING_BYTES", DiagnosticCode::new(0x4042))` to `CODE_REGISTRY`. Without this, `symbolic_code()` falls back to `INTERNAL_INVARIANT`. |
| Kani H6 harness | `crates/vb_storage/src/kani_postcard_envelope_wire.rs` (after H5) | Add `kani_harness_rejects_trailing_bytes` mirroring H5's structure. |
| Fuzz target update | `fuzz/fuzz_targets/fuzz_storage_codec_payload_corruption.rs` | Add a "append 0..=8 trailing junk bytes" loop with an `Err(TrailingBytes)` oracle. |

### 4.3 Out of scope

- Changing `encode_record_payload` or any encoder. The encoder never produces trailing bytes.
- Changing the on-disk record format. No new bytes, no new header fields.
- Refactoring `decode_record_header`. Header parsing is untouched.
- Refactoring `verify_digest_match`. The check runs *before* the digest op.
- Refactoring `JournalError` shape (e.g. splitting into sub-enums).
- Adding a new helper function. The check is inline.
- Modifying `has_terminal_event` at `crates/vb_storage/src/trimming/logic.rs:251`. The change is transparent at the call site.

## 5. Behavior contract

### 5.1 Functional pre/post

For all `bytes: &[u8]`, `expected_magic: u32`, `max_payload_len: u32`:

**Pre:** well-typed inputs; `bytes` is a borrowed slice from any source (Fjall, network, test, fuzz).

**Post (post-fix):**

| Call shape | Post-fix return |
|---|---|
| `bytes.len() < RECORD_HEADER_BYTES` | `Err(UnexpectedEof)` from step 1 (header parse) or step 2 (slice) — unchanged. |
| `bytes.len() >= RECORD_HEADER_BYTES + payload_len` AND header validates AND digest matches | `Ok((RecordEnvelope, payload))` where `payload.len() == header.payload_len as usize`. The slice is exactly `bytes[RECORD_HEADER_BYTES..RECORD_HEADER_BYTES + payload_len]`. |
| `bytes.len() > RECORD_HEADER_BYTES + payload_len` AND header validates AND slice is in-bounds | `Err(TrailingBytes { trailing })` where `trailing = bytes.len() - (RECORD_HEADER_BYTES + payload_len) > 0`. **NEW.** |
| Header invalid (magic, schema, kind, family, length, CRC) | Existing `Err` arm — unchanged. |
| Digest mismatch | `Err(PayloadDigestMismatch)` — unchanged (only reachable when bytes.len() == payload_end). |
| Postcard decode failure | `Err(PostcardDecodeFailed)` — unchanged (only reachable when bytes.len() == payload_end AND digest matched). |
| Kind-parity failure | `Err(RecordKindPayloadMismatch)` or `Err(InvalidEvent)` — unchanged. |

### 5.2 Invariants

The contract locks the following invariants:

| ID | Invariant | Lane |
|---|---|---|
| `INV-CODEC-TB-001` | `decode_record_payload` returns `Err(TrailingBytes { trailing })` iff `bytes.len() > payload_end`. | Verus + Kani + proptest |
| `INV-CODEC-TB-002` | `decode_record_payload` returns `Ok((env, payload))` only if `bytes.len() == payload_end`. | Verus + Kani + proptest |
| `INV-CODEC-TB-003` | The trailing-bytes check precedes `verify_digest_match`. | structural review |
| `INV-CODEC-TB-004` | For `decode_envelope_only(bytes)`, the same `TrailingBytes` invariant holds. | Verus (extern reuse) + proptest |
| `INV-CODEC-TB-005` | `TrailingBytes { trailing: usize }` is reachable only when `trailing > 0`. | type system + Kani |
| `INV-CODEC-TB-006` | `TRAILING_BYTES_CODE == DiagnosticCode::new(0x4042)`. | unit test + Verus |
| `INV-CODEC-TB-007` | Verus PS-003 bridge enumerates `Err(SpecJournalError::TrailingBytes { trailing: u32 })` as a reachable arm with the triggering precondition. | Verus + drift gate |

### 5.3 Ordering

The new check is positioned **between** the `bytes.get(payload_start..payload_end)` call and the `verify_digest_match` call. This is the **cheap-before-expensive** ordering required by `crates/vb_storage/src/kani_postcard_envelope_wire.rs:1-11`.

```
header parse  ->  slice  ->  trailing-bytes check  ->  BLAKE3 digest  ->  postcard decode
                       (cheap)    (cheap, NEW)           (expensive)        (variable)
```

## 6. Error contract

### 6.1 New variant

```rust
#[error("trailing bytes after declared payload: {trailing}")]
TrailingBytes { trailing: usize },
```

### 6.2 Diagnostic code

```rust
pub const TRAILING_BYTES_CODE: DiagnosticCode = DiagnosticCode::new(0x4042);
```

### 6.3 Symbolic code (recommended registration)

```rust
// In crates/vb_storage/src/error/codes.rs::symbolic_code() match arm:
Self::TrailingBytes { .. } => "JOURNAL_TRAILING_BYTES",

// In crates/vb_core/src/diagnostic.rs::CODE_REGISTRY (recommended):
("JOURNAL_TRAILING_BYTES", DiagnosticCode::new(0x4042)),
```

### 6.4 Mutually-exclusive invariant

`TrailingBytes` is mutually exclusive with `UnexpectedEof` for the same call site:

- `UnexpectedEof` fires iff `bytes.len() < payload_end` (input too short).
- `TrailingBytes` fires iff `bytes.len() > payload_end` (input too long).
- Neither fires iff `bytes.len() == payload_end`.

This is guaranteed by step ordering: step 3 (trailing) only runs after step 2 (slice) succeeded, which already implies `payload_end <= bytes.len()`.

## 7. Bridge contract (Verus mirror)

### 7.1 Mirror variant

**Path:** `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:335-413`.

```rust
pub enum SpecJournalError {
    // ... existing variants ...
    /// Mirror of `JournalError::TrailingBytes { trailing }` at
    /// error/mod.rs:NN. Returned by `decode_record_payload`
    /// at codec/payload.rs:NN when `bytes.len() > payload_end`.
    /// `trailing_u32 == (bytes.len() - payload_end) as u32`,
    /// bounded by `bytes.len() <= u32::MAX as usize` for the
    /// verifier model.
    TrailingBytes { trailing: u32 },
    // ... existing variants ...
}
```

### 7.2 Bridge `ensures` arm

**Path:** `verification/verus/vb-vzcuf-PS-003.rs:387-451`.

Add to the `decode_record` `assume_specification[ ... ]` `ensures` `match`:

```rust
Err(SpecJournalError::TrailingBytes { trailing }) => {
    &&& (bytes.len() as u32) > expected_payload_end
    &&& trailing == (bytes.len() as u32) - expected_payload_end
    &&& trailing > 0
},
```

The `expected_payload_end` parameter is a new bridge argument (or derived from `decoded_envelope.payload_len` if `SpecRecordEnvelope` is extended).

### 7.3 Drift expectations

| Gate | Pass condition |
|---|---|
| `scripts/check-production-inner-drift.sh` | `SpecJournalError::TrailingBytes { trailing: u32 }` exists with the correct shape. |
| `scripts/check-verus-production-binding.sh` | The `decode_record` bridge `ensures` enumerates the new arm. |
| `bash scripts/verify-verus.sh` | Verus spec compiles with the new variant. |

## 8. Test contract

The contract specifies **what tests must exist** to lock the regression. The test-writer owns implementation.

### 8.1 Required tests

| Test name | Path | Assertion |
|---|---|---|
| `decode_rejects_trailing_bytes_after_payload` | `crates/vb_storage/src/codec/tests.rs` (replaces `decode_ignores_trailing_bytes_beyond_payload` at 1498-1524) | `matches!(result, Err(JournalError::TrailingBytes { trailing: 3 }))` on a valid record + `0xFF 0xFE 0xFD` tail. |
| `decode_envelope_only_rejects_trailing_payload` | `crates/vb_storage/src/codec/envelope.rs` (sibling of `decode_envelope_only_rejects_truncated_payload`) | `matches!(result, Err(JournalError::TrailingBytes { trailing: 4 }))` on a valid record + 4 appended bytes. |
| `trailing_bytes_variant_and_fields` | `crates/vb_storage/src/error_tests.rs` | Field round-trip and pattern-match on the variant. |
| `trailing_bytes_display_format` | `crates/vb_storage/src/error_tests.rs` | Display contains "trailing" and the byte count. |
| `trailing_bytes_error_code` | `crates/vb_storage/src/error_tests.rs` | `err.diagnostic_code() == TRAILING_BYTES_CODE` and `TRAILING_BYTES_CODE == DiagnosticCode::new(0x4042)`. |
| `trailing_bytes_error_has_correct_code` | `crates/vb_storage/src/error_code_tests.rs` | `TrailingBytes { trailing: N }.diagnostic_code() == TRAILING_BYTES_CODE`. |

### 8.2 Round-trip preservation

All existing round-trip tests (encode → decode) continue to pass unchanged because the encoder never produces trailing bytes. The contract does not introduce any round-trip regression.

## 9. Forbidden patterns

| Pattern | Why forbidden |
|---|---|
| `bytes.get(...)` followed directly by `verify_digest_match(...)` without an intervening `if bytes.len() > payload_end` check. | Violates `INV-CODEC-TB-003`. |
| Two `JournalError` variants both reachable on `bytes.len() > payload_end`. | Violates the mutually-exclusive invariant. |
| `TrailingBytes { trailing: 0 }`. | Violates `INV-CODEC-TB-005`; the variant is unreachable when `trailing == 0`. |
| Numeric codes outside the `0x40xx` journal range for storage-layer errors. | Existing convention. |
| Re-using `0x4042` for a different variant. | Violates the diagnostic-code uniqueness invariant. |
| Re-using the symbolic name `JOURNAL_TRAILING_BYTES` for a different code. | Violates the symbolic-code uniqueness invariant. |
| `unwrap()`, `expect()`, `panic!()`, `todo!()`, `dbg!()` in the post-fix decode path. | Existing AGENTS.md doctrine. |
| `unsafe` in the post-fix decode path. | Existing AGENTS.md doctrine. |
| Wrapping `TrailingBytes` inside `Box<JournalError>` at the producer site. | Existing convention (only `CompiledIrReadback` boxes). |
| Hand-written shadow types without `#[path = "..."]` binding in the Verus mirror. | GOD RULE 2 (vacuum-proof prohibition). |
| Modifying the encoder to "balance" the new check. | The encoder is correct; modifying it risks round-trip breakage. |

## 10. Delivery checklist

The bead is deliverable when **all** of the following are true:

- [ ] Production: `JournalError::TrailingBytes { trailing: usize }` variant added at `crates/vb_storage/src/error/mod.rs`.
- [ ] Production: `TRAILING_BYTES_CODE = 0x4042` constant added at `crates/vb_storage/src/error/codes.rs`.
- [ ] Production: `diagnostic_code()` arm returns `TRAILING_BYTES_CODE` for `TrailingBytes`.
- [ ] Production: `symbolic_code()` arm returns `"JOURNAL_TRAILING_BYTES"` for `TrailingBytes`.
- [ ] Production: Trailing-bytes check added to `decode_record_payload` at `crates/vb_storage/src/codec/payload.rs`.
- [ ] Production: Trailing-bytes check added to `decode_envelope_only` at `crates/vb_storage/src/codec/envelope.rs`.
- [ ] Tests: `decode_rejects_trailing_bytes_after_payload` replaces `decode_ignores_trailing_bytes_beyond_payload`.
- [ ] Tests: `decode_envelope_only_rejects_trailing_payload` added.
- [ ] Tests: error variant trio (`variant_and_fields`, `display_format`, `error_code`) added.
- [ ] Tests: `trailing_bytes_error_has_correct_code` added in `error_code_tests.rs`.
- [ ] Tests: audit header in `error_tests.rs:14-62` updated.
- [ ] Verus mirror: `SpecJournalError::TrailingBytes { trailing: u32 }` variant added.
- [ ] Verus mirror: enumeration comment updated at lines 280-327.
- [ ] Verus bridge: `decode_record` `ensures` arm for `TrailingBytes` added.
- [ ] Recommended: `JOURNAL_TRAILING_BYTES` registered in `CODE_REGISTRY` (optional, non-blocking).
- [ ] Gates: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test -p vb_storage` all pass.
- [ ] Gates: `scripts/check-production-inner-drift.sh` passes.
- [ ] Gates: `scripts/check-verus-production-binding.sh` passes.
- [ ] Gates: `bash scripts/verify-verus.sh` passes (assuming Verus is installed).
- [ ] Round-trip: all existing encode/decode round-trip tests pass unchanged.

## 11. Risk register

| Risk | Severity | Mitigation |
|---|---|---|
| Verus mirror drift breaks production-binding gate. | HIGH | Mirror and bridge updates are part of this bead. |
| Test inversion forgotten; pre-fix test starts failing. | MED | Test rename + inversion is in the delivery scope. |
| Round-trip regression if encoder is accidentally modified. | MED | Encoder is out of scope; reviewer must reject encoder changes. |
| Cheap-before-expensive ordering violated. | MED | Contract pins the position; structural review. |
| Diagnostic-code collision (`0x4042` already taken). | LOW | Verified free in `codebase-map.md`. |
| Symbolic code not registered. | LOW | Recommended but optional. |

## 12. Cross-references

- `domain-model.md` — entities, value objects, invariants, forbidden states.
- `type-contracts.md` — function signatures, variant shape, Verus mirror signatures.
- `workflow-model.md` — decode pipeline states, transitions, ordering.
- `error-taxonomy.md` — variant placement, code wiring, mirror surface.
- `boundary-map.md` — functional-core / imperative-shell split, parser boundary.
- `hazard-analysis.md` — hazard register, severity roll-up, lane requirements.
- `proof-seeds.jsonl` — proof seeds for proof-planner.
- `traceability-matrix.jsonl` — invariant ↔ test ↔ artifact traceability.

---

## Summary

The fix is a single-variant addition to `JournalError`, applied at one canonical site (`decode_record_payload`) and one mirror site (`decode_envelope_only`), bound to the Verus mirror and bridge contract. The test that documented the bug must be inverted. Three new tests lock the regression. Two gates (`check-production-inner-drift.sh`, `check-verus-production-binding.sh`) enforce Verus parity. The contract is minimal because the bug is local.