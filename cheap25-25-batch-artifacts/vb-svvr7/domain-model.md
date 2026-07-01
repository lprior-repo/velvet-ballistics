# Domain Model — vb-svvr7

- bead_id: vb-svvr7
- title: IPC: reject trailing bytes in CLI postcard frame decoder (P1 bug)
- date: 2026-07-01
- state: 3 rust-contract
- workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7

## Ubiquitous Language

- **Frame**: a single bounded sequence of bytes `b[0..HEADER_SIZE + payload_len]` that the CLI postcard envelope defines. A frame consists of exactly one `PostcardHeader` followed by exactly one payload of length `payload_len`. A frame has no prefix, suffix, padding, or trailer.
- **Header**: a fixed `HEADER_SIZE = 52` byte structure at the start of every frame, containing the magic `b"VCLA"`, schema_version (u16 LE), kind (u16 LE), header_len (u32 LE, must equal `HEADER_SIZE`), payload_len (u32 LE, must be `<= MAX_PAYLOAD`), a blake3 digest of the payload, and a CRC32 of the first 48 bytes.
- **Payload**: the variable-length body of a frame, of length `payload_len` bytes, carrying a postcard-encoded `CliPostcardPayload` whose `json_utf8` field is the actual JSON document.
- **Trailing bytes**: any bytes appearing after `b[HEADER_SIZE + payload_len - 1]` in a candidate input. Trailing bytes are NOT part of the frame. They may be (a) zero, (b) the start of the next frame in a multi-frame stream, or (c) garbage, truncation artifact, or hostile injection. The decoder treats them as a single uniform category: invalid framing.
- **Exact-length input**: a byte slice whose length equals `HEADER_SIZE + payload_len` for the header that begins it. The post-fix contract is: success implies exact-length; `<` is impossible; `>` is impossible; only `==` yields success.
- **Strict frame**: a frame that occupies exactly its declared byte range and nothing more. The single fix in scope is to make `decode_postcard` accept only strict frames.

## Domain Decisions

| Decision | Choice | Rationale |
| --- | --- | --- |
| Is "trailing bytes after a valid frame" a valid CLI postcard frame? | No. | A frame is a fixed-length object; suffix bytes violate the contract. |
| Does a valid CLI envelope tolerate multiple concatenated frames in one buffer? | No (in this scope). | Concatenation is the caller's responsibility; the decoder MUST NOT accept `Ok` for a buffer that contains a frame plus anything else. |
| Is a frame ever accepted when `data.len() < payload_end`? | No. | Truncation is already rejected by the existing `<` check. After the fix, both directions of inequality are rejected. |
| Does the new error variant carry the trailing length? | No — keep it unit-shaped. | All 11 existing variants are unit; carrying payload would break parity with the established display style. |
| Should `decode_postcard_json` re-map `TrailingBytes` to `DecodeFailed`? | No. | Surfacing as-is lets callers distinguish framing bugs (`TrailingBytes`) from data corruption (`DecodeFailed`). |
| Does the fix change `CLI_SCHEMA_VERSION`? | No. | This is a bug fix, not a contract version change. |
| Does the fix change `MAX_PAYLOAD` or `HEADER_SIZE`? | No. | Constants are unaffected; the only change is the comparator and the new error variant. |
| Does the sibling `vb_ipc::frame::decode_frame_payload` need the same fix? | No. | It already uses `if payload.len() != expected_len` at `frame.rs:44`; the bug is asymmetric to that boundary. |

## Entities / Value Objects

| Type | Kind | Carries |
| --- | --- | --- |
| `Frame` (proposed) | Value object | `(&[u8; HEADER_SIZE], &[u8])` where `header.1.len() == header.0.payload_len` |
| `PostcardHeader` | Existing value object | magic, schema_version, kind, header_len, payload_len, payload_digest, header_crc |
| `CliPostcardPayload` | Existing value object | schema_version, kind, content_type, json_utf8 |
| `StrictFrame` (proposed typestate) | Typestate marker | Marks slices whose `len() == HEADER_SIZE + payload_len` |
| `PostcardError::TrailingBytes` | New enum variant | Unit; no payload |

The proposed `Frame` and `StrictFrame` are **not** introduced in this fix scope (the fix is arithmetic + an enum variant). They are listed as future-typed contracts that, if introduced later, would make the bug unrepresentable by construction. Today the bug is fixed by tightening the comparator.

## Aggregate / Workflow Root

- The CLI postcard module (`vb_cli::cli_postcard`) is a small aggregate over the byte buffer domain.
- Its single write-path is `encode_postcard`; its single read-path is `decode_postcard`. There is no concurrency, no shared state, no persistence.
- The aggregate invariant is: **every accepted frame has length `HEADER_SIZE + payload_len` exactly**.

## Commands / Events / Policies

| Action | Policy |
| --- | --- |
| `decode_postcard` success | Returns `(header_bytes, payload)` AND requires `data.len() == payload_end`. |
| `decode_postcard` rejection on `data.len() < payload_end` | Returns `Err(PostcardError::DecodeFailed)`. (Already correct.) |
| `decode_postcard` rejection on `data.len() > payload_end` | Returns `Err(PostcardError::TrailingBytes)`. (NEW — replaces the silent acceptance.) |
| `decode_postcard_json` propagation | Forwards every `PostcardError` variant via `?`; no masking. |
| OutputError::PostcardFrame wrapping | Consumes inner `Display`; requires the new Display arm. |

## Forbidden States (post-fix)

The following representable-but-illegal states are now structurally rejected:

1. `Ok((_, _))` from `decode_postcard` when `data.len() > HEADER_SIZE + payload_len`. (Now returns `TrailingBytes`.)
2. `Ok((_, _))` from `decode_postcard` when `data.len() < HEADER_SIZE + payload_len`. (Already returns `DecodeFailed`.)
3. `PostcardError::TrailingBytes` returning a non-zero `Display` arm that mis-classifies the failure mode. (Now arm exists.)

## Invariants

- **INV-TB-1**: For all `data: &[u8]` and `result = decode_postcard(data)`, `result.is_ok()` implies `data.len() == HEADER_SIZE + payload_len` where `payload_len` is the `payload_len` field of the decoded header.
- **INV-TB-2**: For all `data: &[u8]` with `data.len() > HEADER_SIZE + payload_len_after_header_parse`, `decode_postcard(data)` returns `Err(PostcardError::TrailingBytes)` *unless* an earlier stage of the parse pipeline already returned a different error (which takes precedence by ordering).
- **INV-TB-3**: `PostcardError::TrailingBytes` is `PartialEq`-equal only to itself; it cannot be confused with `DecodeFailed` (which means truncation) or `InvalidHeaderLength` (which means the header's own `header_len` field is wrong).
- **INV-TB-4**: `decode_postcard_json(data)` returns `Err(PostcardError::TrailingBytes)` whenever `decode_postcard(data)` would, because the propagation chain is a single `?` operator over the same enum.

## Open Domain Questions

1. Should a future revision introduce `Frame` and `StrictFrame` newtypes that *make* the bug unrepresentable rather than *detect* it? (Out of scope for this bead.)
2. Should the encoder be allowed to produce a multi-frame concatenation in one `Vec<u8>`? (Today it does not; the encoder emits exactly one frame.)
3. Should the CLI wire a length-prefixed stream protocol atop this envelope? (Out of scope.)