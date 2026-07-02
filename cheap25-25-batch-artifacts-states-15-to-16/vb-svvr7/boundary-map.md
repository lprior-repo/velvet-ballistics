# Boundary Map — vb-svvr7

- bead_id: vb-svvr7
- date: 2026-07-01
- state: 3 rust-contract
- scope: identify the boundary touched by the strict-length fix

## Boundary Topology

```
 ┌──────────────────────────────────────────────────────────────────┐
 │  EXTERNAL WORLD (hostile input)                                 │
 │  - subprocess stdout bytes from `velvet-ballistics` CLI         │
 │  - in-process bytes from another Rust crate using the envelope  │
 └───────────────────────────────┬──────────────────────────────────┘
                                 │  &[u8]
                                 ▼
 ┌──────────────────────────────────────────────────────────────────┐
 │  PARSER BOUNDARY (the fix site)                                 │
 │  crates/vb_cli/src/cli_postcard/validation.rs:71-101             │
 │    - decode_postcard(&[u8]) -> Result<(&[u8], &[u8]), _>        │
 │    - This is the ONLY boundary that hardens in this bead.       │
 └───────────────────────────────┬──────────────────────────────────┘
                                 │  (&[u8; HEADER_SIZE], &[u8])
                                 ▼
 ┌──────────────────────────────────────────────────────────────────┐
 │  INNER POSTCARD DECODER                                         │
 │  crates/vb_cli/src/cli_postcard/codec.rs:8-13                   │
 │    - decode_cli_payload(payload) -> Result<CliPostcardPayload>  │
 │  Inner check via postcard crate; result mapped to DecodeFailed. │
 └───────────────────────────────┬──────────────────────────────────┘
                                 │  CliPostcardPayload
                                 ▼
 ┌──────────────────────────────────────────────────────────────────┐
 │  CLI PAYLOAD VALIDATOR                                          │
 │  crates/vb_cli/src/cli_postcard/validation.rs:7-20               │
 │    - validate_cli_payload(&payload) -> Result<(), _>            │
 │    - Schema/kind/content_type assertion.                         │
 └───────────────────────────────┬──────────────────────────────────┘
                                 │  ()
                                 ▼
 ┌──────────────────────────────────────────────────────────────────┐
 │  JSON PARSER                                                    │
 │  crates/vb_cli/src/cli_postcard/codec.rs:31-32                   │
 │    - serde_json::from_slice(&payload.json_utf8)                  │
 │    - Failure mapped to JsonPayloadDecodeFailed.                 │
 └───────────────────────────────┬──────────────────────────────────┘
                                 │  serde_json::Value
                                 ▼
 ┌──────────────────────────────────────────────────────────────────┐
 │  CLI OUTPUT CONSUMER                                            │
 │  crates/vb_cli/src/output.rs:135-147                             │
 │  tests/cli_integration.rs:2671-2703                              │
 │    - decode_postcard_json is the public entry for outbound CLI  │
 │      consumers and the integration test helper.                 │
 └──────────────────────────────────────────────────────────────────┘
```

## Boundary Inventory

| Boundary | Direction | Hardened here? | Notes |
| --- | --- | --- | --- |
| Hostile input → parser | Inbound | YES | `decode_postcard` is the single hardening point. |
| Parser → inner postcard decoder | Internal | No | Pure delegation. |
| Inner decoder → validator | Internal | No | Pure delegation. |
| Validator → JSON parser | Internal | No | Pure delegation. |
| JSON parser → CLI consumer | Outbound | No | Read-only call site. |
| CLI consumer → `OutputError::PostcardFrame` | Wrapper | Display arm only | No code change beyond Display arm. |

## Hardening Detail (the Fix)

- **File**: `crates/vb_cli/src/cli_postcard/validation.rs`
- **Function**: `decode_postcard`
- **Lines of interest**: 87-89 (the `<` check) and 80-85 (the `payload_end` arithmetic that produces the value compared).
- **Pre-fix comparator**: `data.len() < payload_end` returns `DecodeFailed`.
- **Post-fix comparator**: `data.len() != payload_end` returns `DecodeFailed` when `<` and `TrailingBytes` when `>`.
- **Order of operations**: the length check is performed BEFORE `data.get(0..HEADER_SIZE)` and `data.get(payload_start..payload_end)` so the new variant, not a hypothetical `ok_or(DecodeFailed)` on `get`, is the surface error. This ordering is already correct in the source; the fix preserves it.

## Functional Core / Imperative Shell

This module is **pure functional core** with no imperative shell:

- **Pure core**: `decode_postcard`, `decode_postcard_json`, `encode_postcard`, `validate_cli_payload`, `payload_digest`, `validate_header_crc`, `validate_version_and_kind`. All borrow or return `Vec<u8>`; none perform I/O.
- **Imperative shell**: NONE in this module. The CLI binary that owns this module is the shell, but the fix is module-internal.

## Async Shell

- The decode functions are synchronous and total.
- No `tokio`, no `async`, no `await`, no `Future`, no `Stream` are touched.

## Storage / Network / Time Boundaries

- **Storage**: NONE. The function is in-memory only.
- **Network**: NONE.
- **Time**: NONE. No `SystemTime`, no `Instant`, no `Duration`.
- **FFI**: NONE. No `extern "C"`.
- **Unsafe**: NONE. The file starts with `#![forbid(unsafe_code)]`.

## Boundary Test Points

| Test point | Type | Status post-fix |
| --- | --- | --- |
| `data.len() < HEADER_SIZE` | truncation | rejects with `DecodeFailed` (unchanged) |
| `data.len() < HEADER_SIZE + payload_len` | truncation | rejects with `DecodeFailed` (unchanged) |
| `data.len() == HEADER_SIZE + payload_len`, valid header, valid CRC, valid digest | success | accepts (unchanged) |
| `data.len() > HEADER_SIZE + payload_len`, valid header, valid CRC, valid digest | framing violation | **rejects with `TrailingBytes`** (NEW) |
| `data.len() > HEADER_SIZE + payload_len`, invalid header | earlier error | rejects with `InvalidMagic | InvalidHeaderLength | …` (unchanged; the new check is unreachable because earlier stage failed) |
| `data.len() > HEADER_SIZE + payload_len`, valid header, invalid CRC | earlier error | rejects with `CrcMismatch` (the new check is reached but passes, S5 catches it) |
| `data.len() > HEADER_SIZE + payload_len`, valid header, valid CRC, invalid digest | earlier error | rejects with `DigestMismatch` |

The last three rows clarify that the new error variant is the SOLE error for the strict-length case **when the frame is otherwise valid**. Any earlier-stage failure takes precedence by ordering — this matches the existing pipeline behavior.

## Boundary Crossing Invariants

- **BC-TB-1**: No bytes cross the parser boundary except the borrowed `data: &[u8]`. No copy, no clone.
- **BC-TB-2**: On success, the returned slices alias the input `data`; mutating `data` mutates the returned slices. Callers must not rely on this aliasing unless they explicitly intend it.
- **BC-TB-3**: No allocation on success. `decode_postcard` returns borrowed slices only.
- **BC-TB-4**: The inner `decode_cli_payload` may allocate (delegated to `postcard::from_bytes`); this is out of scope for the strict-length fix.

## Public-API Boundary

- `decode_postcard` is `pub(crate)`. The new check does not change its visibility.
- `PostcardError::TrailingBytes` is reachable through `vb_cli::cli_postcard::PostcardError` (re-exported in `cli_postcard.rs:22`) and through `vb_cli::OutputError::PostcardFrame`.
- Adding a variant is an **additive** API change for downstream match arms in the same crate, and **non-breaking** for code outside the crate that consumes only `Display`.
- The integration test `crates/vb_cli/tests/cli_integration.rs:2671-2703` calls `decode_postcard_json(&output.stdout)`. The CLI encoder never emits trailing bytes, so this call site is unaffected by the fix.

## Module Visibility Summary

```
vb_cli/src/lib.rs           pub mod cli_postcard              (no change)
vb_cli/src/cli_postcard.rs  pub(crate) use error::PostcardError   (no change; new variant is
                                                                  auto-reachable)
vb_cli/src/cli_postcard.rs  pub(crate) use validation::{decode_postcard, ...}  (no change)
```

## Cross-Boundary Risk Map

| Risk | Boundary | Mitigation |
| --- | --- | --- |
| Misalignment with `vb_ipc::frame::decode_frame_payload` | Cross-crate boundary | The IPC sibling already uses `!=`; the CLI fix aligns the two. No cross-crate change required. |
| Regression in `assert_postcard_stdout` integration test | Test boundary | CLI encoder emits no trailing bytes; integration test continues to pass. |
| `OutputError::PostcardFrame` Display format | Wrapper boundary | New Display arm added; format consumed by `write!(... "{error}")` is consistent with siblings. |
| `cargo clippy` lint about `if x != y { if x < y { A } else { B } }` | Source lint boundary | The pattern is acceptable; if clippy objects, an `match data.len().cmp(&payload_end)` is the idiomatic alternative and is documented in `proof-seeds.jsonl` as a possible follow-up. |