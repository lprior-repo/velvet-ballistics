# Error Taxonomy — vb-svvr7

- bead_id: vb-svvr7
- date: 2026-07-01
- state: 3 rust-contract
- scope: enumerate every `PostcardError` variant and add `TrailingBytes`

## Error Algebra

`PostcardError` is a flat, total, closed enum with `#[derive(Debug, Clone, PartialEq, Eq)]`. It is the sum type returned by every public decode entry point. The taxonomy below lists all 12 variants (11 existing + 1 new) with their semantic, surface stage, and the response pattern callers should adopt.

## Variants

| # | Variant | Stage | Semantic | Caller response |
| --- | --- | --- | --- | --- |
| 1 | `InvalidMagic` | S1 | First 4 bytes are not `b"VCLA"`. | Reject frame; treat as untrusted buffer. |
| 2 | `InvalidHeaderLength` | S1, S2 | `header_len` field is not 52, or header parse cannot continue. | Reject frame; report framing violation. |
| 3 | `PayloadTooLarge` | S2, S4 | `payload_len > MAX_PAYLOAD` (65536), or `payload_len` does not fit in `usize`. | Reject frame; report size violation. |
| 4 | `VersionTooOld` | S3 | `schema_version == 0`. | Reject frame; require upgrade of producer. |
| 5 | `VersionTooNew` | S3 | `schema_version > CLI_SCHEMA_VERSION`. | Reject frame; require upgrade of consumer. |
| 6 | `WrongKind` | S3 | `kind != CLI_POSTCARD_KIND`. | Reject frame; route to a different codec. |
| 7 | `DigestMismatch` | S6 | `blake3(payload) != header.payload_digest`. | Reject frame; report tampering or corruption. |
| 8 | `CrcMismatch` | S5 | `crc32fast(header_bytes[0..48]) != header_bytes[48..52]`. | Reject frame; report header tampering or corruption. |
| 9 | `PayloadMetadataMismatch` | post-decode (`validate_cli_payload`) | `schema_version`, `kind`, or `content_type` of the inner `CliPostcardPayload` does not match CLI contract. | Reject payload; report metadata violation. |
| 10 | `JsonPayloadDecodeFailed` | post-decode (`serde_json::from_slice`) | `json_utf8` is not valid JSON. | Reject payload; report JSON parse failure. |
| 11 | `DecodeFailed` | S0, S4 (`<` branch), S5 fallback | Catch-all "decode failed"; specifically: buffer shorter than `HEADER_SIZE`, OR shorter than `payload_end`. | Reject frame; insufficient data. |
| **12** | **`TrailingBytes`** ★ NEW | **S4 (`>` branch)** | **Buffer longer than `HEADER_SIZE + payload_len`.** | **Reject frame; framing violation.** |

## The New Variant

### `PostcardError::TrailingBytes`

- **Shape**: unit.
- **Display arm**: `"postcard decode failed: trailing bytes after valid frame"`.
- **Distinguishing property**: it is the ONLY variant that fires on `data.len() > HEADER_SIZE + payload_len`. All other variants either fire on truncated input, malformed input, or tampered input. A downstream tool that distinguishes "valid frame + junk" from "truncated frame" can match on this variant alone.
- **Backwards compatibility**: additive. Existing `match` arms over `PostcardError` continue to compile because Rust's exhaustiveness checker will demand a new arm; this is a desired forcing function for downstream. The `output.rs:OutputError::PostcardFrame` arm wraps `PostcardError` via `Display`, not via inner-match, so it requires only the new Display arm.

## Error Flow Diagram

```
decode_postcard(data: &[u8])
  │
  ├─ data.len() < HEADER_SIZE                          → DecodeFailed
  │
  ├─ header parse failure                              → InvalidMagic | InvalidHeaderLength
  │
  ├─ header.validate() failure                         → InvalidHeaderLength | PayloadTooLarge
  │
  ├─ version/kind failure                              → VersionTooOld | VersionTooNew | WrongKind
  │
  ├─ payload_len > usize (on 16-bit usize)             → PayloadTooLarge
  │
  ├─ payload_start + payload_len overflows             → DecodeFailed
  │
  ├─ data.len() < payload_end                          → DecodeFailed                (preserved)
  │
  ├─ data.len() > payload_end                          → TrailingBytes              ★ NEW
  │
  ├─ header CRC mismatch                               → CrcMismatch
  │
  ├─ payload digest mismatch                           → DigestMismatch
  │
  └─ all checks pass                                   → Ok((header_bytes, payload))
```

## Why `TrailingBytes` Is Not `DecodeFailed`

`DecodeFailed` is the catch-all "I cannot parse this"; it conflates:

- buffer too short for the header (S0),
- payload_end arithmetic overflow (S4 fallback),
- header CRC sub-slice too short (S5 fallback, currently unreachable post-S4),
- the inner `decode_cli_payload` failure (`postcard::from_bytes` returns its own typed error which we map to `DecodeFailed`).

Adding a new variant `TrailingBytes` distinguishes the framing violation from the catch-all so that:

1. **Callers can route**: a tool piping postcard output can detect "frame + extra junk" and decide to either truncate and retry or surface a specific error.
2. **Tests can assert tightly**: existing tests use `assert_eq!(decode_postcard(&data), Err(PostcardError::DecodeFailed))` for the truncation case; the new test uses `Err(PostcardError::TrailingBytes)` for the surplus case. No collision.
3. **Forensic value**: a log line that says "trailing bytes after valid frame" lets operators distinguish "the sender sent extra noise" from "the receiver truncated".

## Error Algebra Properties

- **Closed under `?`**: every variant propagates through the `?` operator; no variant requires special-case handling at any call site (`decode_postcard_json` proves this).
- **Total**: every input yields either `Ok` or exactly one `Err`. No `Result<Result<_, _>, _>`.
- **`PartialEq`**: every variant is `PartialEq`-equal only to itself; tests can `assert_eq!` over the result.
- **`Clone`/`Debug`/`Eq`**: derive-inherited. `std::error::Error` blanket impl is unaffected.

## Forbidden Patterns in the Error Surface

- **Do not** add a payload to `TrailingBytes`. Carrying the trailing length would diverge from the unit-shaped convention of every other variant.
- **Do not** introduce a "framing error" super-variant that wraps both `DecodeFailed` and `TrailingBytes`. The two are distinguishable and conflation would erase forensic value.
- **Do not** add a new `Result<Result<...>>` shape; keep the surface flat.
- **Do not** change `Debug`/`Display`/`PartialEq`/`Eq`/`Clone` derives; the fix is additive.

## Error Taxonomy Invariants

- **ET-TB-1**: For every `data: &[u8]`, `decode_postcard(data)` returns at most one error.
- **ET-TB-2**: The variant returned is a deterministic function of `data` and the parsed header (no environment dependency, no time dependency).
- **ET-TB-3**: `TrailingBytes` is reachable only via S4 in the post-fix implementation.
- **ET-TB-4**: `TrailingBytes` and `DecodeFailed` are distinct variants; `assert_eq!(Err(TrailingBytes), Err(DecodeFailed))` is `false`.

## Cross-Reference With `OutputError`

`OutputError::PostcardFrame(cli_postcard::PostcardError)` in `crates/vb_cli/src/output.rs` wraps the inner error and surfaces its `Display` string to the user. The new variant is wrapped automatically; the only requirement is the new `Display` arm. No exhaustive inner match is required by `output.rs` because the wrapper formats via `Display`, not via inner match.