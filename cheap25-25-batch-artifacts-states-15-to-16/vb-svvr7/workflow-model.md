# Workflow Model — vb-svvr7

- bead_id: vb-svvr7
- date: 2026-07-01
- state: 3 rust-contract
- scope: tighten `decode_postcard` length check; add `PostcardError::TrailingBytes`

## Workflow Root

The CLI postcard decode pipeline is a **synchronous, single-shot, total pipeline** with no retries, no cancellation, no concurrency, and no temporal window. It is not a "workflow" in the multi-stage runtime sense; it is a parse function with a strict stage ladder.

## Stage Ladder

```
                          data: &[u8]
                               │
                               ▼
        ┌──────────────────────────────────────────┐
   S0   │  data.len() < HEADER_SIZE               │  → Err(DecodeFailed)
        └──────────────────────────────────────────┘
                               │
                               ▼
        ┌──────────────────────────────────────────┐
   S1   │  PostcardHeader::from_bytes(data)       │  → Err(InvalidMagic |
        │                                          │     InvalidHeaderLength)
        └──────────────────────────────────────────┘
                               │
                               ▼
        ┌──────────────────────────────────────────┐
   S2   │  header.validate()                      │  → Err(InvalidHeaderLength
        │                                          │     | PayloadTooLarge)
        └──────────────────────────────────────────┘
                               │
                               ▼
        ┌──────────────────────────────────────────┐
   S3   │  validate_version_and_kind(&header)     │  → Err(VersionTooOld |
        │                                          │     VersionTooNew |
        │                                          │     WrongKind)
        └──────────────────────────────────────────┘
                               │
                               ▼
        ┌──────────────────────────────────────────┐
   S4   │  payload_end =                          │  → Err(PayloadTooLarge
        │     HEADER_SIZE.checked_add(payload_len) │     | DecodeFailed)
        │  data.len() != payload_end              │  → Err(TrailingBytes) ★ NEW
        │                                          │     | DecodeFailed
        └──────────────────────────────────────────┘
                               │
                               ▼
        ┌──────────────────────────────────────────┐
   S5   │  validate_header_crc(header_bytes)      │  → Err(CrcMismatch |
        │                                          │     DecodeFailed)
        └──────────────────────────────────────────┘
                               │
                               ▼
        ┌──────────────────────────────────────────┐
   S6   │  payload_digest(payload) != header.     │  → Err(DigestMismatch)
        │             payload_digest               │
        └──────────────────────────────────────────┘
                               │
                               ▼
                          Ok((header_bytes, payload))
```

★ marks the stage whose outcome changes in this bead.

## Stages in Detail

### S0: Truncation check (header absent)

- **Guard**: `data.len() >= HEADER_SIZE` (52).
- **Outcome on guard fail**: `Err(PostcardError::DecodeFailed)`.
- **Outcome on guard pass**: proceed to S1.
- **Temporal**: instantaneous; no I/O.

### S1: Header decode

- **Action**: `PostcardHeader::from_bytes(data)` (read-only struct parse of the first 52 bytes).
- **Outcome on error**: `Err(PostcardError::InvalidMagic | InvalidHeaderLength)`.
- **Outcome on success**: `Ok(PostcardHeader)`; proceed to S2.

### S2: Header self-validation

- **Action**: `header.validate()` (asserts `header_len == HEADER_SIZE`, `payload_len <= MAX_PAYLOAD`, magic equality).
- **Outcome on error**: `Err(PostcardError::InvalidHeaderLength | PayloadTooLarge)`.
- **Outcome on success**: proceed to S3.

### S3: Version + kind

- **Action**: `validate_version_and_kind(&header)`.
- **Outcome on error**: `Err(PostcardError::VersionTooOld | VersionTooNew | WrongKind)`.
- **Outcome on success**: proceed to S4.

### S4: Frame-bounded length check (CHANGED)

- **Action (pre-fix)**: `if data.len() < payload_end { return Err(DecodeFailed) }`. **Bug**: accepts `data.len() > payload_end`.
- **Action (post-fix)**: `if data.len() != payload_end { return Err(if data.len() < payload_end { DecodeFailed } else { TrailingBytes }) }`. Bug closed.
- **Outcome on success**: proceed to S5.
- **Outcome ordering**: this stage MUST execute before any `data.get(...)` call so the new variant, not a hypothetical `get().ok_or(DecodeFailed)`, is the surface error.

### S5: Header CRC

- **Action**: `validate_header_crc(header_bytes)` (recomputes CRC32 over bytes 0..48, compares with bytes 48..52).
- **Outcome on error**: `Err(PostcardError::CrcMismatch)` (or `DecodeFailed` if the slice is too short, which is impossible after S4).
- **Outcome on success**: proceed to S6.

### S6: Payload digest

- **Action**: `payload_digest(payload)` and equality with `header.payload_digest`.
- **Outcome on error**: `Err(PostcardError::DigestMismatch)`.
- **Outcome on success**: `Ok((header_bytes, payload))`.

## `decode_postcard_json` Super-Stage

```
data: &[u8]
    │
    ▼
[ S0..S6 from decode_postcard ]  → may Err(TrailingBytes) here ★
    │
    ▼
PostcardHeader::from_bytes(header_bytes)   → Err(InvalidMagic | InvalidHeaderLength)
    │
    ▼
decode_cli_payload(payload_bytes)          → Err(DecodeFailed)
    │
    ▼
validate_cli_payload(&payload)             → Err(PayloadMetadataMismatch)
    │
    ▼
serde_json::from_slice(&payload.json_utf8) → Err(JsonPayloadDecodeFailed)
    │
    ▼
Ok((header, value))
```

- The new `TrailingBytes` variant appears in the first stage and propagates unchanged through every subsequent stage.
- No stage remaps the error to `DecodeFailed`.

## Terminal States

- **Ok((header_bytes, payload))**: the canonical success outcome. By INV-TB-1, this implies `data.len() == HEADER_SIZE + payload_len`.
- **Err(PostcardError::TrailingBytes)**: new terminal; frames with extra bytes are rejected here.
- All other `Err` variants are unchanged terminals.

## Idempotence

The function is pure and idempotent: `decode_postcard(data) == decode_postcard(data)` for all `data`. No mutation, no cache, no I/O.

## Cancellation

None. There is no async context, no long-running operation, no opportunity for cancellation.

## Retries

None. The function has no retry policy; callers may retry the entire call.

## Hazard Lanes Touched

| Hazard lane | Touched? |
| --- | --- |
| Temporal | No — synchronous, total function. |
| Concurrency | No — pure, no shared state. |
| Persistence | No — no on-disk effect. |
| Unsafe/UB | No — `#![forbid(unsafe_code)]` and `checked_add`/`get` are safe. |
| Parser/codec | YES — strict-length parser invariant. |
| Public API | YES — additive enum variant. |
| Performance | No — single integer compare. |
| User-visible behavior | YES — downstream CLI consumers piping postcard output now reject extra bytes. |

## Workflow Invariants

- **WF-TB-1**: Every `Err(PostcardError)` returned by `decode_postcard` is exactly one of the 12 enum variants.
- **WF-TB-2**: The variant returned by S4 is deterministic given `data.len()` and `payload_end`: `<` ⇒ `DecodeFailed`; `>` ⇒ `TrailingBytes`; `==` ⇒ proceed.
- **WF-TB-3**: Stages S1–S3 short-circuit before S4 is reached; S5–S6 are skipped if S4 fails.
- **WF-TB-4**: `decode_postcard_json` returns the same variant set as `decode_postcard` plus three additional variants (`DecodeFailed` again, `PayloadMetadataMismatch`, `JsonPayloadDecodeFailed`).

## What This Workflow Is NOT

- Not a multi-step stateful process.
- Not a runtime supervisor.
- Not a worker pool.
- Not a streaming protocol. Concatenated frames in a single buffer are explicitly rejected as `TrailingBytes` (this is the bug fix).