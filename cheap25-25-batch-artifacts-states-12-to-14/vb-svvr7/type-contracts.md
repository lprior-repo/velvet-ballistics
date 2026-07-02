# Type Contracts — vb-svvr7

- bead_id: vb-svvr7
- date: 2026-07-01
- state: 3 rust-contract
- scope: tighten `decode_postcard` length check; add `PostcardError::TrailingBytes`

## Source of Truth

| Path | Symbol | Role |
| --- | --- | --- |
| `crates/vb_cli/src/cli_postcard/validation.rs` | `decode_postcard` | Parser under fix |
| `crates/vb_cli/src/cli_postcard/validation.rs` | `validate_cli_payload` | Downstream consumer (read-only here) |
| `crates/vb_cli/src/cli_postcard/validation.rs` | `validate_header_crc`, `validate_version_and_kind` | Stage helpers (read-only) |
| `crates/vb_cli/src/cli_postcard/error.rs` | `PostcardError` | Error algebra |
| `crates/vb_cli/src/cli_postcard/types.rs` | `PostcardHeader`, `CliPostcardPayload`, constants | Existing shape contract |
| `crates/vb_cli/src/cli_postcard/codec.rs` | `decode_postcard_json`, `decode_cli_payload`, `encode_postcard` | Public surface |

## New Type Contracts

### `PostcardError::TrailingBytes` (added)

- **Shape**: unit variant.
- **Position**: appended after `JsonPayloadDecodeFailed` (or alongside `DecodeFailed`; final placement is a Display-order detail, not a behavioural one).
- **Semantic**: input buffer contained a valid frame plus one or more bytes after the frame end.
- **Display arm**: `"postcard decode failed: trailing bytes after valid frame"` (or symmetric with `DecodeFailed` phrasing).
- **`PartialEq`/`Eq`/`Clone`/`Debug`**: derive inherits all of them because the enum already derives these traits.
- **`std::error::Error`**: continues to be implemented via the blanket impl over `Display`.

### `decode_postcard` (tightened postcondition)

- **Signature**: unchanged. `pub(crate) fn decode_postcard(data: &[u8]) -> Result<(&[u8], &[u8]), PostcardError>`.
- **Preconditions**: unchanged.
- **Postconditions** (replacing the prior `<` check):
  - `data.len() < HEADER_SIZE + payload_len` ⇒ `Err(PostcardError::DecodeFailed)` (preserved).
  - `data.len() > HEADER_SIZE + payload_len` ⇒ `Err(PostcardError::TrailingBytes)` (**NEW**).
  - `data.len() == HEADER_SIZE + payload_len` ⇒ continue to the existing digest + CRC checks.
- **Ordering invariant**: the trailing-bytes check must run BEFORE the `data.get(0..HEADER_SIZE)` and `data.get(payload_start..payload_end)` calls so the bug surfaces as the typed error, not as a silent `DecodeFailed` from a hypothetical `ok_or` on `get`.
- **No allocation**: `decode_postcard` continues to borrow from `data`; the new check is a single integer compare.

### `decode_postcard_json` (propagation contract)

- **Signature**: unchanged. `pub(crate) fn decode_postcard_json(data: &[u8]) -> Result<(PostcardHeader, serde_json::Value), PostcardError>`.
- **Propagation rule**: every `PostcardError` variant returned by `decode_postcard` is returned by `decode_postcard_json` unchanged. The new `TrailingBytes` variant follows this rule because the `?` operator covers the entire enum uniformly.
- **No re-mapping**: callers see `TrailingBytes` directly, never masked as `DecodeFailed`.

### `encode_postcard` (encoder invariant — read-only confirmation)

- **Encoder MUST emit**: `header_size + payload_len` bytes, no more, no less.
- **Encoder length claim**: `result.len() == HEADER_SIZE + payload.len()`. This makes the encoder a witness for the decoder's exact-length invariant.
- **No silent trailing bytes from our own encoder**: confirmed by reading `codec.rs:46-73`.

## Forbidden States Made Unrepresentable (Type-Level)

| Forbidden state | Mechanism |
| --- | --- |
| `decode_postcard` returning `Ok` when `data.len() > HEADER_SIZE + payload_len` | Now returns `Err(TrailingBytes)` at the only point where the post-decode length comparison happens. |
| `TrailingBytes` matching the `DecodeFailed` arm in downstream matches | The two variants are distinct enum cases; `match` is exhaustive only if every variant is handled. Adding a variant requires updating exhaustiveness in `output.rs:OutputError::PostcardFrame` only if `output.rs` matches on the inner enum (it does not — it formats via `Display`). |
| `TrailingBytes` being silently swallowed | `?` propagates uniformly; the only way to swallow would be an explicit `if Err(PostcardError::TrailingBytes) { ... }` branch, which does not exist in the current call chain. |

## Type Contract Checklist (from `references/type-contract-checklist.md`)

- [x] Replace stringly IDs and primitive domain values with newtypes. — `PostcardHeader`, `CliPostcardPayload` are already non-stringly. The fix does not regress this.
- [x] Replace boolean behavior flags with enums. — The fix adds a new enum variant, not a boolean flag.
- [x] Replace `Option` lifecycle state with explicit state variants. — `TrailingBytes` is an explicit outcome variant, not an `Option`.
- [x] Parse external input once at the boundary. — `decode_postcard` is the single parser entry; the new check is enforced at this one boundary.
- [x] Represent domain failures with semantic error variants. — `TrailingBytes` is semantic (frame-bounded length violation), distinct from `DecodeFailed` (truncation).
- [x] Keep pure core free of I/O, time, network, storage, and randomness. — `decode_postcard` remains a pure function over `&[u8]`. No `SystemTime`, no allocator-visible side effect beyond `decode_cli_payload` (which delegates to `postcard::from_bytes`, itself pure).

## Cross-Reference With Sibling Boundary

- `vb_ipc::frame::decode_frame_payload` (`crates/vb_ipc/src/frame.rs:35-51`) already enforces `if payload.len() != expected_len` at line 44. The new `TrailingBytes` semantics in `vb_cli::cli_postcard` bring the two boundaries into parity: both now require exact length for `Ok`.

## Type-Level Risks That Remain Representable

These are NOT in scope for this bead but should be tracked:

1. **No `Frame` newtype.** A future `Frame<'a> { header: &'a PostcardHeader, payload: &'a [u8] }` whose constructor asserts the length invariant would push this bug from "detected" to "unrepresentable." Out of scope.
2. **No `StrictLength` typestate.** A future typestate on `&[u8]` that tracks "frame-bounded" status would also work. Out of scope.
3. **Encoder never tested for absence of trailing bytes.** `encode_postcard` is asserted to emit `HEADER_SIZE + payload.len()` bytes by reading the source, but a property `prop_encoder_emits_exact_length` would close that gap.

## Acceptance Conditions for `type-contracts.md`

The implementation is correct iff:

1. `PostcardError` has a new variant whose name is exactly `TrailingBytes`.
2. The new variant is unit-shaped.
3. `Display` for the new variant is implemented.
4. `decode_postcard` returns `Err(PostcardError::TrailingBytes)` when `data.len() > HEADER_SIZE + payload_len` AND no earlier stage errored.
5. `decode_postcard` continues to return `Err(PostcardError::DecodeFailed)` when `data.len() < HEADER_SIZE + payload_len`.
6. `decode_postcard_json` propagates `TrailingBytes` without remapping.
7. No new public type is added beyond the new enum variant.