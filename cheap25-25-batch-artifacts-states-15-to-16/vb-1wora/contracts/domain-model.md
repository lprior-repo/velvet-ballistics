# Domain Model — vb-1wora

**Bead:** `vb-1wora` — Codec: reject trailing bytes after declared record payload (P1 bug)
**Skill:** `rust-contract` (State 3)
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`
**Captured:** 2026-07-01
**JJ root:** verified at workspace root (`cheap25-vb-1wora` workspace)
**Source inputs:** `.beads/vb-1wora/STATE.md`, `.beads/vb-1wora/codebase-map.md`, `.beads/vb-1wora/delivery-scope.jsonl`

---

## 1. Ubiquitous Language

The domain is the v1 storage record codec in `vb_storage` and its interaction with the `JournalError` railway. Vocabulary is constrained to terms already present in production code so the contract is checkable.

| Term | Meaning | Source |
|---|---|---|
| **Envelope** | The first `RECORD_HEADER_BYTES = 60` bytes of a stored record. Carries `magic`, `schema_version`, `record_kind`, `sequence`, `payload_len`, `payload_digest`, plus a CRC32C. | `crates/vb_storage/src/constants.rs:84`, `crates/vb_storage/src/types.rs::RecordEnvelope` |
| **Payload** | The bytes `bytes[60..60+header.payload_len]` after a valid header. Subject to BLAKE3 digest match. | `crates/vb_storage/src/codec/payload.rs:56-82` |
| **Record** | `Envelope ++ Payload` as a contiguous `&[u8]`. The on-disk / over-the-wire shape. | `crates/vb_storage/src/codec/mod.rs:60-71` |
| **Declared payload boundary** | The half-open range `[RECORD_HEADER_BYTES, RECORD_HEADER_BYTES + header.payload_len)`. Authoritative because `header.payload_len` is itself CRC32C-protected. | derived from `crates/vb_storage/src/codec/payload.rs:62-71` |
| **Trailing bytes** | Any byte at offset `>= payload_end` in the input slice, when `payload_end < bytes.len()`. Pre-fix these bytes were silently dropped by `bytes.get(payload_start..payload_end)`; post-fix they MUST yield `JournalError::TrailingBytes { trailing }`. | this contract |
| **Record envelope decode** | The full pipeline: header parse → slice → digest verify → postcard decode → parity check. | `crates/vb_storage/src/codec/payload.rs:56-82`, `crates/vb_storage/src/codec/mod.rs:82-95` |
| **Inspection-only decode** | A decoder that returns `(RecordEnvelope, &[u8])` *without* calling `postcard::from_bytes`, used by `doctor`/`filtering` workflows. | `crates/vb_storage/src/codec/envelope.rs:18-83` |
| **Fail-closed** | An invariant: if any step of the decode pipeline produces an error variant, the function MUST return `Err` and MUST NOT yield `Ok` for malformed input. | existing doctrine; this contract enforces it for the trailing-bytes case |
| **Cheap-before-expensive** | The decode order convention: shape checks (slice length, range bounds) run before expensive cryptographic checks (BLAKE3 digest) and deserialization (postcard). | `crates/vb_storage/src/kani_postcard_envelope_wire.rs:1-11` and this contract |

## 2. Entities, Value Objects, Aggregates

The codec surface is already highly typed; this contract introduces one new variant and re-asserts the shape of the existing aggregate.

### 2.1 Aggregate: `Record` (production-borrowed, not new)

- Lifetime: as long as the borrowed `&[u8]` input.
- Identity: none (records are content-addressed downstream by digest).
- Composition: `Record = Envelope (60 bytes) ++ Payload (header.payload_len bytes)`. Anything beyond `payload_end` is **outside the aggregate** and a structural defect.
- Invariant (NEW, this contract): `bytes.len() == RECORD_HEADER_BYTES + header.payload_len` is required for any `Ok` return from `decode_record_payload` and `decode_envelope_only`.

### 2.2 Value object: `TrailingBytes { trailing: usize }`

- Type: `JournalError::TrailingBytes { trailing: usize }`.
- Domain meaning: the input slice declared a payload boundary at `payload_end` but continued past it by exactly `trailing` bytes (`trailing == bytes.len() - payload_end`).
- Validation: `trailing > 0`. The variant is unreachable for any input where `bytes.len() <= payload_end` (those cases are not "trailing" — they are either truncated (`UnexpectedEof`) or perfectly bounded).
- Precedent: `JournalError::MalformedKeyspaceRow { actual_len: usize, expected_len: usize }` at `crates/vb_storage/src/error/mod.rs:97-105` already uses `usize` for byte-length deltas. The `TrailingBytes` variant follows that precedent.

### 2.3 Re-asserted value objects (unchanged)

| Value object | Where | Invariant |
|---|---|---|
| `RecordEnvelope` | `crates/vb_storage/src/types.rs` | Captures envelope metadata only; not affected by the fix. |
| `RecordHeader` | `crates/vb_storage/src/types.rs` | The 60-byte header, already validated by `decode_record_header`. |
| `JournalError` (enum) | `crates/vb_storage/src/error/mod.rs:21-188` | Railway enum; gains one new variant. |
| `DiagnosticCode` | `crates/vb_core/src/diagnostic.rs` | Stable numeric identifier; `0x4042` is the next free slot in the `0x40xx` journal range. |
| `SymbolicCode` | `crates/vb_core/src/diagnostic.rs` | Symbolic identifier; recommended registration is `JOURNAL_TRAILING_BYTES`. |

## 3. Commands, Events, Policies

This bead is a codec-layer hardening; it introduces no new commands or events, but it does sharpen one policy.

### 3.1 Commands (existing, sharpened)

| Command | Site | Policy |
|---|---|---|
| `decode_record<T>(bytes, expected_magic, max_payload_len)` | `crates/vb_storage/src/codec/mod.rs:82-95` | MUST fail closed if `bytes.len() > payload_end`. |
| `decode_journal_event(bytes, expected_magic, max_payload_len)` | `crates/vb_storage/src/codec/mod.rs:126-151` | Inherits the policy from `decode_record`. No direct change. |
| `decode_record_payload(bytes, expected_magic, max_payload_len)` | `crates/vb_storage/src/codec/payload.rs:56-82` | MUST fail closed if `bytes.len() > payload_end`. CANONICAL SITE for the check. |
| `decode_envelope_only(bytes)` | `crates/vb_storage/src/codec/envelope.rs:48-83` | MUST fail closed if `bytes.len() > payload_end`. MIRROR of the canonical site. |

### 3.2 Events (none new)

The codec surface has no observable events; failures are returned as `JournalError` variants and are propagated by callers (e.g. `crates/vb_storage/src/trimming/logic.rs:251`).

### 3.3 Policies (sharpened)

- **P1 (new): Fail-closed on trailing bytes.** Any decoder in the canonical pipeline (`decode_record_payload` and `decode_envelope_only`) MUST return `Err(JournalError::TrailingBytes { trailing })` when the input slice extends past the declared payload boundary. The check MUST run **before** `verify_digest_match` and **before** `postcard::from_bytes`.
- **P2 (existing): Cheap-before-expensive decode order.** Header parse → slice → trailing-bytes check → BLAKE3 digest → postcard decode. The new check slots in between slice bounds and digest to satisfy this.
- **P3 (existing): Boundary-bytes-equals-payload-end for Ok.** `Ok` requires `bytes.len() == payload_end`. This is the converse of P1 and was implicit before the fix; the contract makes it explicit.

## 4. Invariants

| ID | Invariant | Lane |
|---|---|---|
| `INV-CODEC-TB-001` | `decode_record_payload(bytes, magic, max)` returns `Err(TrailingBytes { trailing })` iff `bytes.len() > payload_end`. | Verus + Kani + proptest |
| `INV-CODEC-TB-002` | `decode_record_payload(bytes, magic, max)` returns `Ok((env, payload))` only if `bytes.len() == payload_end`. | Verus + Kani + proptest |
| `INV-CODEC-TB-003` | The `TrailingBytes` check precedes `verify_digest_match`. The pre-fix call ordering (`bytes.get` → `verify_digest_match`) is updated to `bytes.get` → trailing check → `verify_digest_match`. | structural (lint-able via call-site review) |
| `INV-CODEC-TB-004` | For `decode_envelope_only(bytes)`, the same `TrailingBytes` invariant holds, mirror of `INV-CODEC-TB-001`/`002`. | Verus (extern reuse) + proptest |
| `INV-CODEC-TB-005` | `JournalError::TrailingBytes { trailing }` is the **only** error variant for inputs where `bytes.len() > payload_end` but the slice `[payload_start..payload_end]` is otherwise well-formed. Pre-fix this case returned `Ok` silently. | Kani + proptest + manual review |
| `INV-CODEC-TB-006` | `TRAILING_BYTES_CODE == DiagnosticCode::new(0x4042)`. `symbolic_code()` for the variant returns either `JOURNAL_TRAILING_BYTES` (if registered in `CODE_REGISTRY`) or `INTERNAL_INVARIANT` (existing fallback for unregistered codes). | proptest + error-code unit test |
| `INV-CODEC-TB-007` | The Verus PS-003 bridge `assume_specification[ production::decode_record ]` enumerates `Err(SpecJournalError::TrailingBytes { trailing: u32 })` as a reachable arm, with `bytes.len() > payload_end` as the triggering precondition. | Verus + production-binding gate |

## 5. Forbidden / Illegal States (now unrepresentable in the well-typed domain)

The fix removes one "silently accepted" illegal state and makes it impossible for the new variant to be mis-coded.

| Illegal state | Now representable? |
|---|---|
| `decode_record_payload` returns `Ok` for `bytes.len() > payload_end` (pre-fix bug) | **NO** — must return `Err(TrailingBytes { trailing })`. |
| `decode_envelope_only` returns `Ok` for `bytes.len() > payload_end` (pre-fix bug, mirror) | **NO** — must return `Err(TrailingBytes { trailing })`. |
| `TrailingBytes { trailing: 0 }` | **NO** — the check fires only when `bytes.len() > payload_end`, so `trailing >= 1`. |
| Variant encoded as a unit (no count) when the count matters for telemetry/diagnostics | **NO** — `trailing: usize` is required so the operator knows how many bytes were ignored. |
| Two `JournalError` variants both reachable from the same `(bytes, magic, max)` shape that contradict each other | **NO** — `TrailingBytes` is mutually exclusive with `UnexpectedEof` (one fires on `bytes.len() < payload_end`, the other on `bytes.len() > payload_end`). |
| `TRAILING_BYTES_CODE` colliding with an existing numeric code | **NO** — `0x4042` is verified free in `crates/vb_storage/src/error/codes.rs` and in `crates/vb_core/src/diagnostic.rs::CODE_REGISTRY` slice up to line 1583. |

## 6. Open Domain Questions (carried forward to planner)

The contract deliberately *does not* decide these — the proof-planner and proof-writer may need to weigh in:

1. **Variant shape final.** This contract standardizes on `TrailingBytes { trailing: usize }` (count only), mirroring `MalformedKeyspaceRow { actual_len: usize, expected_len: usize }`. The Verus mirror uses `u32` because Verus spec values for byte counts are bounded by `u32::MAX` (the `header.payload_len` field is `u32`). Both shapes must agree modulo the cast. If the planner wants `TrailingBytes { trailing: usize, declared_payload_len: u32 }`, the contract must be re-issued.
2. **Symbolic-code registration.** Recommended: register `JOURNAL_TRAILING_BYTES` in `crates/vb_core/src/diagnostic.rs::CODE_REGISTRY` so the symbolic name surfaces rather than falling back to `INTERNAL_INVARIANT`. The contract treats this as a *recommended* but not mandatory piece of the change.
3. **Coverage of `decode_envelope_only`.** Recommended yes (mirror). The contract lists it as `INV-CODEC-TB-004`. If the planner chooses to defer it to a follow-up bead, the bridge contract for `decode_record` still binds, but the inspection-only surface diverges temporarily.
4. **Kani H6 harness.** The contract lists the trailing-bytes path as a Kani-eligible claim (see `proof-seeds.jsonl`) but does not require a dedicated harness. The planner may add a H6 mirror of H5.
5. **Fuzz target update.** The contract lists a fuzz surface (see `proof-seeds.jsonl`) but does not require it. The existing `fuzz_storage_codec_payload_corruption.rs` already feeds random bytes; the targeted "append N junk bytes" oracle is additive.

## 7. Out of Scope

- Changing the encoder (`encode_record` / `encode_record_payload`). The encoder never produces trailing bytes, so no change is needed and any change here would risk breaking round-trip tests.
- Changing the on-disk record format. The fix is purely on the decode path; no new bytes, no new header fields.
- Changing `decode_record_header`. Header parsing is untouched.
- Changing `verify_digest_match`. The trailing-bytes check runs *before* the digest op.
- Refactoring `JournalError` shape (e.g. splitting into sub-enums). Single-variant addition is the minimum-fuss template.
- Adding a new helper function (e.g. `decode_payload_only_with_trailing_check`). The check is inline; if the planner later wants to extract a helper, that is a separate bead.

---

## Summary

The fix introduces a single new variant, a single shape invariant on the decode pipeline, and a single Verus mirror arm. The domain is small because the bug is local: `decode_record_payload` and its mirror `decode_envelope_only` must reject, not silently accept, inputs whose slice extends past `RECORD_HEADER_BYTES + header.payload_len`. The cheap-before-expensive order is preserved by inserting the check between slice bounds and the BLAKE3 op.