# Workflow Model — vb-1wora

**Bead:** `vb-1wora` — Codec: reject trailing bytes after declared record payload (P1 bug)
**Skill:** `rust-contract` (State 3)
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`

This file specifies the legal states and transitions of the v1 storage record **decode pipeline**. The pipeline is a single linear chain with failure exits; it is not a true state machine, but its branches and ordering are policy-bearing and must be locked by the contract.

---

## 1. Decode pipeline (single-pass linear chain)

The decode pipeline runs in fixed order. Each step has a single input, a single output (Ok or a specific `Err` variant), and a transition rule. Failures are **terminal** for the call — no recovery, no retry. The pipeline is pure (no I/O, no time, no randomness).

### 1.1 Pipeline diagram

```
bytes: &[u8]
   |
   v
[1] decode_record_header(bytes, expected_magic, max_payload_len) --> Result<RecordHeader, JournalError>
   |                                                                      |
   |                                                  Err(BadMagic | UnsupportedSchemaVersion |
   |                                                       MigrationRequired | UnknownRecordKind |
   |                                                       RecordKindFamilyMismatch | HeaderLengthMismatch |
   |                                                       PayloadTooLarge | HeaderChecksumMismatch |
   |                                                       UnexpectedEof | InvalidEvent | ...)
   |
   | Ok(h)
   v
[2] payload_start  := h.header_len as usize
    payload_end    := payload_start + (h.payload_len as usize)
    payload        := bytes.get(payload_start..payload_end)
                       .ok_or(UnexpectedEof)?
   |                                                          \
   |                                                           \-- Err(UnexpectedEof)
   v
[3] *** TRAILING-BYTES CHECK ***  bytes.len() > payload_end  --> Err(TrailingBytes { trailing })    <-- NEW
   |
   | (bytes.len() == payload_end)
   v
[4] verify_digest_match(payload, h.payload_digest)           --> Err(PayloadDigestMismatch)
   |
   | Ok
   v
[5a] (decode_record_payload returns)  Ok((RecordEnvelope, payload))
    [or]

[5b] postcard::from_bytes(payload)   --> Err(PostcardDecodeFailed)
   |
   | Ok(value)
   v
[6] T::enforce_kind_parity(&envelope, &value)               --> Err(RecordKindPayloadMismatch | InvalidEvent)
   |
   | Ok
   v
[7] (decode_record returns)         Ok((RecordEnvelope, value))
```

For `decode_journal_event` an additional step 8 runs after step 7:

```
[8a] validate_journal_event_record_kind(&envelope, &event)  --> Err(RecordKindPayloadMismatch)
[8b] event.is_valid() == true                                --> Err(InvalidEvent)
[8c] envelope.sequence == event.seq().get()                  --> Err(ReplayEnvelopeSequenceMismatch)
```

### 1.2 Step-by-step state table

Each row is one step's contract. "Pre" = precondition the prior step guarantees. "Post" = postcondition this step guarantees. "Err" = the specific `JournalError` arm(s) this step can emit; the others are unreachable here.

| Step | Site | Pre | Ok Post | Err Arm(s) |
|---|---|---|---|---|
| 1 | `codec/header.rs:decode_record_header` | `bytes: &[u8]` well-typed. | `RecordHeader` parsed; magic/schema/kind/length/CRC validated. | `BadMagic { found }`, `UnsupportedSchemaVersion { version }`, `MigrationRequired { from, to }`, `UnknownRecordKind { kind }`, `RecordKindFamilyMismatch { magic, kind }`, `HeaderLengthMismatch { found }`, `PayloadTooLarge { len, max }`, `HeaderChecksumMismatch`, `UnexpectedEof` (if bytes < 60), `InvalidConfig { ... }` |
| 2 | `codec/payload.rs:62-71` | `RecordHeader` parsed. | `payload: &[u8]` of length `header.payload_len as usize`. | `UnexpectedEof` (if slice is shorter than `payload_start + payload_len`). |
| **3** | `codec/payload.rs:NN-NN` **NEW** | `payload` slice obtained. | Either `bytes.len() == payload_end`, or **fail closed**. | **`TrailingBytes { trailing }` where `trailing = bytes.len() - payload_end`, `trailing > 0`.** |
| 4 | `codec/payload.rs:verify_digest_match` | `payload.len() == header.payload_len as usize`, `bytes.len() == payload_end`. | BLAKE3 digest matches. | `PayloadDigestMismatch`. |
| 5a | `codec/payload.rs:73-81` | Digest match. | `Ok((RecordEnvelope, payload))` returned by `decode_record_payload`. | (none — terminal Ok for `decode_record_payload`) |
| 5b | `codec/mod.rs:92` | `payload.len() == header.payload_len`. | `value: T` deserialized. | `PostcardDecodeFailed(#[source] postcard::Error)`. |
| 6 | `codec/kind_parity.rs` (trait `EnforceKindParity`) | `envelope: RecordEnvelope`, `value: T`. | Parity between envelope kind and payload holds. | `RecordKindPayloadMismatch { envelope_kind, payload_kind }` or `InvalidEvent` (for `T = JournalEvent`). |
| 7 | `codec/mod.rs:82-95` | All prior steps Ok. | `Ok((RecordEnvelope, value))`. | (none — terminal Ok for `decode_record`) |
| 8 | `codec/mod.rs:135-149` | `decode_journal_event` only. | Sequence number parity and validity hold. | `RecordKindPayloadMismatch`, `InvalidEvent`, `ReplayEnvelopeSequenceMismatch { ... }`. |

### 1.3 Variant exclusivity

The decode pipeline guarantees that for any given `(bytes, expected_magic, max_payload_len)` triple, at most **one** `JournalError` arm fires. The new invariant is:

> `TrailingBytes` is mutually exclusive with `UnexpectedEof` for the same call:
>
> - `UnexpectedEof` fires iff `bytes.len() < payload_end` (input too short).
> - `TrailingBytes` fires iff `bytes.len() > payload_end` (input too long, after the slice-bounds check passed).
> - Neither fires iff `bytes.len() == payload_end`.

This is a direct consequence of the check ordering: step 2's `bytes.get(...)` either succeeds (slice is in bounds, payload extracted) or fails with `UnexpectedEof`. Step 3 only runs after step 2 succeeded, so by the time step 3 runs we know `payload_end <= bytes.len()`. The new branch only fires when `payload_end < bytes.len()` (strict).

## 2. Caller-side workflows

### 2.1 Replay / doctor workflow

```
snapshot.prefix(prefix)  -->  for each item:
                                bytes = item.value()
                                match decode_journal_event(bytes, magic, max):
                                    Ok((env, event)) => process(env, event),
                                    Err(_)            => continue  (or surface depending on policy)
```

**Pre-fix behavior:** corrupted rows with trailing bytes silently decoded the prefix, dropping the tail. Doctor scans could mis-report "valid" events that were actually malformed.

**Post-fix behavior:** corrupted rows with trailing bytes return `Err(TrailingBytes)`. The caller (e.g. `crates/vb_storage/src/trimming/logic.rs:251` `has_terminal_event`) propagates via `?` and the loop continues to the next item. Doctor surfaces the trailing-bytes count via the diagnostic code (`0x4042`) and symbolic name (`JOURNAL_TRAILING_BYTES`).

### 2.2 Admission / replay-during-recovery workflow

Identical pattern. The fix is transparent at the call site — no new error handling is required, but operators get fail-closed semantics for free.

### 2.3 Inspection-only workflow (`decode_envelope_only`)

```
doctor / filtering tool:
   let (env, raw_payload) = decode_envelope_only(bytes)?;
   // inspect env without deserializing payload
```

**Pre-fix behavior:** silent acceptance of trailing bytes (the docstring's claim of "envelope + payload validation" was incomplete).

**Post-fix behavior:** trailing bytes yield `Err(TrailingBytes)`. The inspection tool can decide whether to surface, log, or skip the row.

## 3. State invariants (the decode pipeline as a typed state)

Treating the pipeline as a small state machine for documentation purposes:

| State | Reached when | Allowed next |
|---|---|---|
| `Input(bytes)` | initial | `HeaderParse` |
| `HeaderParse(Ok)` | step 1 Ok | `SliceExtract` |
| `HeaderParse(Err)` | step 1 Err | terminal `Err(...)` |
| `SliceExtract(Ok)` | step 2 Ok | `TrailingCheck` |
| `SliceExtract(Err)` | step 2 Err (`UnexpectedEof`) | terminal `Err(UnexpectedEof)` |
| **`TrailingCheck(Ok)`** | **step 3 Ok (`bytes.len() == payload_end`)** | **`DigestVerify`** (NEW state, NEW transition) |
| **`TrailingCheck(Err)`** | **step 3 Err (`bytes.len() > payload_end`)** | **terminal `Err(TrailingBytes { trailing })`** (NEW terminal state) |
| `DigestVerify(Ok)` | step 4 Ok | `Ok((env, payload))` (terminal) |
| `DigestVerify(Err)` | step 4 Err | terminal `Err(PayloadDigestMismatch)` |

In the post-fix pipeline, `TrailingCheck` is a new state that gates `DigestVerify`. The new invariant is: **`DigestVerify` is unreachable from inputs with `bytes.len() > payload_end`.**

## 4. Failure-mode taxonomy (decode pipeline)

| Class | Arm(s) | Cause | Recovery |
|---|---|---|---|
| **Header structural** | `BadMagic`, `UnsupportedSchemaVersion`, `MigrationRequired`, `UnknownRecordKind`, `RecordKindFamilyMismatch`, `HeaderLengthMismatch`, `PayloadTooLarge`, `HeaderChecksumMismatch`, `UnexpectedEof` (in step 1, when bytes < 60) | Wire bytes do not describe a valid header. | None at the call site; surface to operator. |
| **Slice structural** | `UnexpectedEof` (in step 2, when bytes are 60..payload_end) | Slice is shorter than declared payload. | None; surface. |
| **Tail structural** (NEW) | `TrailingBytes { trailing }` | Slice is longer than declared payload. | None; surface. |
| **Integrity** | `PayloadDigestMismatch` | BLAKE3 hash does not match the stored digest. | None; surface. |
| **Deserialization** | `PostcardDecodeFailed`, `RecordKindPayloadMismatch`, `InvalidEvent` | Payload bytes are well-bounded but the inner format is broken. | None; surface. |
| **Replay** | `ReplayEnvelopeSequenceMismatch` | Envelope seq ≠ payload seq. | None; surface (fail-closed on forged records). |

## 5. Temporal / ordering hazards

### 5.1 Cheap-before-expensive

The pipeline orders steps by cost:

- Step 1 (header parse): integer compares + CRC32C; cheap.
- Step 2 (slice): pointer arithmetic + bounds check; cheap.
- **Step 3 (trailing-bytes check, NEW): integer compare + subtraction; cheap.**
- Step 4 (BLAKE3 digest): cryptographic hash; expensive (especially for large payloads).
- Step 5b (postcard decode): variable cost; bounded by payload size.

The trailing-bytes check is correctly slotted between steps 2 and 4 so a shape defect never triggers a BLAKE3 op. This is the "cheap-before-expensive" convention codified in `crates/vb_storage/src/kani_postcard_envelope_wire.rs:1-11`.

### 5.2 Pre-fix ordering bug

**Pre-fix ordering:** step 1 → step 2 → step 4 (BLAKE3) → step 5b (postcard). Step 3 was missing entirely. Inputs with `bytes.len() > payload_end` would proceed to step 4 with the valid `[0..payload_end]` slice and a length-correct digest; BLAKE3 would match (because the digest was computed over the valid prefix only), and step 5b would deserialize the prefix. The trailing bytes were silently dropped.

**Post-fix ordering:** step 1 → step 2 → step 3 (NEW) → step 4 → step 5b. Inputs with `bytes.len() > payload_end` fail closed at step 3 without paying for BLAKE3 or postcard.

## 6. Idempotence

- The trailing-bytes check is **idempotent**: applying it twice yields the same result.
- It is **pure**: no side effects, no allocation beyond the `usize` subtraction.
- It is **deterministic**: given the same `bytes.len()` and `payload_end`, the same `TrailingBytes { trailing }` is returned.

## 7. Cancellation / interruption

Not applicable. The decode pipeline is synchronous, single-threaded, and does not block on I/O. There is no cancellation point.

## 8. Concurrency

Not applicable. `decode_record_payload` and `decode_envelope_only` take `&[u8]` (shared borrow), have no interior mutability, and do not spawn tasks. Multiple concurrent calls are safe by Rust's `&`-aliasing rules; the Loom lane is not required.

## 9. End-to-end terminal states

For a single `decode_record` call:

- `Ok((env, value))` — success; caller proceeds.
- `Err(journal_err)` — failure; one specific `JournalError` arm (see §1.2).

The contract guarantees: **exactly one of `Ok` or `Err` is returned, and the `Err` arm carries enough information to triage** (either a count, an enum discriminant, or both).

## 10. Workflow policy summary

| ID | Policy | Enforced by |
|---|---|---|
| `POL-WF-001` | Trailing-bytes check runs **before** `verify_digest_match`. | structural review; the new `if` block is positioned between step 2 and step 4 in `codec/payload.rs` and `codec/envelope.rs`. |
| `POL-WF-002` | Trailing-bytes check is **before** `postcard::from_bytes`. | same; the check is in `decode_record_payload`, which is called before `postcard::from_bytes` in `codec/mod.rs:91-92`. |
| `POL-WF-003` | `TrailingBytes` and `UnexpectedEof` are mutually exclusive for the same call. | step ordering: `TrailingCheck` only runs after `SliceExtract` succeeded, which already implies `payload_end <= bytes.len()`. |
| `POL-WF-004` | The fix is symmetric across `decode_record_payload` and `decode_envelope_only`. | INV-CODEC-TB-004 (type-contracts.md). |
| `POL-WF-005` | The fix preserves round-trip behavior for `encode_record`-produced inputs. | the encoder (`encode_record_payload` at `codec/payload.rs:34-54`) always produces `bytes.len() == RECORD_HEADER_BYTES + payload_len`; no test breakage for round-trip. |

---

## Summary

The decode pipeline gains one new state (`TrailingCheck`) between the existing `SliceExtract` and `DigestVerify` states. The new state has two outcomes: `Ok` (proceed to digest) or `Err(TrailingBytes { trailing })` (terminal). The change is minimal and ordering-preserving: cheap shape checks remain before expensive cryptographic work.