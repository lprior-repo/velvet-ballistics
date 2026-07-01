# Contract — vb-svvr7

- bead_id: vb-svvr7
- date: 2026-07-01
- state: 3 rust-contract
- title: IPC: reject trailing bytes in CLI postcard frame decoder (P1 bug)

## Summary

Tighten the strict-length frame check in `vb_cli::cli_postcard::decode_postcard` so that a valid frame plus one or more trailing bytes returns a new typed error `PostcardError::TrailingBytes`, rather than being silently accepted. The existing `DecodeFailed` path for truncated input is preserved. The new variant propagates through `decode_postcard_json` via `?` with no code change in `codec.rs`. The fix aligns the CLI envelope boundary with the sibling `vb_ipc::frame::decode_frame_payload`, which already enforces `!=`.

## Ubiquitous Language (terse)

- **Frame**: `HEADER_SIZE + payload_len` bytes; nothing before, nothing after.
- **Trailing bytes**: any bytes that appear after `HEADER_SIZE + payload_len - 1` in a candidate buffer.
- **Strict frame**: a frame whose buffer length equals exactly `HEADER_SIZE + payload_len`.

## Contract Clauses

| ID | Clause |
| --- | --- |
| **CC-TB-1** | For all `data: &[u8]`, `decode_postcard(data).is_ok()` ⇒ `data.len() == HEADER_SIZE + payload_len` (where `payload_len` is the `payload_len` field of the decoded header). |
| **CC-TB-2** | For all `data: &[u8]` with `data.len() < HEADER_SIZE + payload_len`, `decode_postcard(data) = Err(PostcardError::DecodeFailed)` (provided an earlier stage did not fire a more specific error). |
| **CC-TB-3** | For all `data: &[u8]` with `data.len() > HEADER_SIZE + payload_len`, `decode_postcard(data) = Err(PostcardError::TrailingBytes)` (provided every earlier stage passed; an earlier-stage error takes precedence). |
| **CC-TB-4** | `PostcardError::TrailingBytes` is a unit variant of `PostcardError`. |
| **CC-TB-5** | `Display::fmt(PostcardError::TrailingBytes, _)` writes a non-empty, human-readable string that distinguishes this variant from `DecodeFailed`. |
| **CC-TB-6** | `decode_postcard_json(data)` returns `Err(PostcardError::TrailingBytes)` whenever `decode_postcard(data)` does. |
| **CC-TB-7** | `encode_postcard(v, k, p)` returns a buffer of length `HEADER_SIZE + p.len()` exactly. (Already true; preserved by the fix.) |
| **CC-TB-8** | `encode_postcard(v, k, p).is_ok()` and `decode_postcard(&encoded).is_ok()` together imply `decode_postcard(&encoded) = Ok((header_bytes, payload))` where `payload == p`. (Roundtrip consistency.) |
| **CC-TB-9** | The fix is purely additive on the public surface: `PostcardError` gains one variant; no signature changes; no visibility changes; no version bumps. |
| **CC-TB-10** | The fix preserves INV-005 (bounded allocation) and POST-007 (magic + header length validation before payload decode). |

## Mapping to Source Files

| Clause | File:Line | Edit kind |
| --- | --- | --- |
| CC-TB-1, CC-TB-3 | `crates/vb_cli/src/cli_postcard/validation.rs:87-89` | Replace `if data.len() < payload_end { Err(DecodeFailed) }` with `if data.len() != payload_end { Err(if data.len() < payload_end { DecodeFailed } else { TrailingBytes }) }` |
| CC-TB-4, CC-TB-5 | `crates/vb_cli/src/cli_postcard/error.rs:7-30` | Add unit variant `TrailingBytes` to `PostcardError`. |
| CC-TB-5 | `crates/vb_cli/src/cli_postcard/error.rs:32-48` | Add Display arm: `Self::TrailingBytes => write!(f, "postcard decode failed: trailing bytes after valid frame")`. |
| CC-TB-6 | `crates/vb_cli/src/cli_postcard/codec.rs:27` | NO change — `?` propagates the new variant uniformly. |
| CC-TB-7 | `crates/vb_cli/src/cli_postcard/codec.rs:46-73` | NO change — encoder already emits exact length. |
| CC-TB-1 (unit test) | `crates/vb_cli/src/cli_postcard/tests.rs` | Add `decode_rejects_trailing_bytes_after_valid_frame` and `decode_accepts_exact_length_frame` regression tests. |
| CC-TB-1, CC-TB-3 (proptest) | `verification/proptest/properties.rs` | Add `prop_strict_length_no_trailing_bytes` property: for any payload `p` with `p.len() <= MAX_PAYLOAD` and any trailing length `n` in `[1, 4096]`, `let mut buf = encode_postcard(...); buf.extend(vec![0; n]); assert_eq!(decode_postcard(&buf), Err(TrailingBytes))`. |
| CC-TB-9 | `crates/vb_cli/src/cli_postcard.rs:22-32` | NO change — re-exports pick up the new variant automatically. |
| CC-TB-9 | `crates/vb_cli/src/lib.rs:5` | NO change. |
| CC-TB-9 | `crates/vb_cli/src/output.rs:135-147` | NO change — Display consumption handles the new variant. |
| CC-TB-9 | `crates/vb_cli/Cargo.toml` | NO change — no dependency delta. |

## Pre/Post Conditions Per Function

### `decode_postcard`

- **Pre**: `data: &[u8]`.
- **Post-Ok**: returns `Ok((header_bytes, payload))` with `header_bytes.len() == HEADER_SIZE`, `payload.len() == header.payload_len`, and `data.len() == HEADER_SIZE + payload.len() == payload_end`.
- **Post-Err**: returns one of `PostcardError::{InvalidMagic, InvalidHeaderLength, PayloadTooLarge, VersionTooOld, VersionTooNew, WrongKind, CrcMismatch, DigestMismatch, DecodeFailed, TrailingBytes}` exactly once.

### `decode_postcard_json`

- **Pre**: `data: &[u8]`.
- **Post-Ok**: returns `Ok((header, value))` with `header: PostcardHeader` and `value: serde_json::Value`.
- **Post-Err**: returns one of the 12 `PostcardError` variants exactly once; the new `TrailingBytes` is reachable only via the first `?` chain element.

### `encode_postcard`

- **Pre**: `schema_version: u16`, `kind: u16`, `payload: &[u8]` with `payload.len() <= MAX_PAYLOAD`.
- **Post-Ok**: returns `Ok(buf)` with `buf.len() == HEADER_SIZE + payload.len()`.
- **Post-Err**: returns `Err(PostcardError::PayloadTooLarge)` if `payload.len() > MAX_PAYLOAD`.

## Error Algebra (post-fix)

```
PostcardError =
    InvalidMagic
  | InvalidHeaderLength
  | PayloadTooLarge
  | VersionTooOld
  | VersionTooNew
  | WrongKind
  | DigestMismatch
  | CrcMismatch
  | PayloadMetadataMismatch
  | JsonPayloadDecodeFailed
  | DecodeFailed
  | TrailingBytes         // NEW
```

All variants are unit-shaped. `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, and `std::error::Error` are uniformly implemented.

## Invariants (Restated)

- **INV-TB-1**: `Ok` ⇒ exact length.
- **INV-TB-2**: `data.len() > payload_end` AND earlier stages pass ⇒ `TrailingBytes`.
- **INV-TB-3**: `TrailingBytes` is distinct from `DecodeFailed` in both `PartialEq` and `Display`.
- **INV-TB-4**: `decode_postcard_json` propagates `TrailingBytes` without remapping.

## Proof Coverage Targets (Seed Hints Only — proof-planner owns final lanes)

- **Rust-local implementation**: proptest over arbitrary `payload: Vec<u8>` and arbitrary `trailing: Vec<u8>` of length 1..=4096 to assert `decode_postcard(encode_postcard(p) ++ trailing) = Err(TrailingBytes)`.
- **Rust-local implementation**: cargo unit test asserting `decode_rejects_trailing_bytes_after_valid_frame` and `decode_accepts_exact_length_frame`.
- **Refinement (optional)**: a `#[cfg(kani)]` harness with `kani::any::<Vec<u8>>()` feeding `decode_postcard` would expose the strict-length branch to bounded-model-check; per AGENTS.md GOD RULE 1 the harness must NOT hardcode shapes. Optional; proptest is the primary evidence.
- **Miri**: not applicable (`#![forbid(unsafe_code)]`).
- **Verus**: not applicable (no production-bound spec exists for this module; per GOD RULE 2 a vacuum Verus spec is rejected).
- **Flux**: not applicable (the fix is a single integer compare; Flux refinement on length is unnecessary).
- **Loom**: not applicable (no concurrency).
- **cargo-fuzz**: optional; a fuzz target `fuzz_postcard_strict_length` covering `&[u8]` of length `[HEADER_SIZE, HEADER_SIZE + MAX_PAYLOAD + 8192]` would close the hostile-input lane. Not required because proptest already covers arbitrary trailing lengths.

## Acceptance Conditions

The bead is accepted iff:

1. `PostcardError::TrailingBytes` exists in `crates/vb_cli/src/cli_postcard/error.rs`.
2. `Display::fmt` for `PostcardError::TrailingBytes` is implemented and non-empty.
3. `decode_postcard` returns `Err(PostcardError::TrailingBytes)` for any input whose length exceeds `HEADER_SIZE + payload_len` by at least one byte, AND whose header, version, kind, CRC, and digest all validate.
4. `decode_postcard` continues to return `Err(PostcardError::DecodeFailed)` for any input whose length is less than `HEADER_SIZE + payload_len`.
5. `decode_postcard_json` propagates `TrailingBytes` via `?` without remapping.
6. The unit test `decode_rejects_trailing_bytes_after_valid_frame` passes.
7. The unit test `decode_accepts_exact_length_frame` passes (regression).
8. The proptest `prop_strict_length_no_trailing_bytes` passes.
9. `cargo clippy -p vb_cli --all-targets -- -D warnings` passes.
10. `moon run :source-lint` passes (canonical CI gate per AGENTS.md).

## Out of Scope

- Introducing a `Frame<'a>` newtype that would make the bug structurally impossible.
- Adding a multi-frame streaming protocol atop the CLI envelope.
- Adding a `TrailingBytes { count: usize }` payload — kept unit for parity.
- Miri, Verus, Flux, Kani harnesses — not warranted given the module's character and AGENTS.md GOD RULES.
- Bumping `CLI_SCHEMA_VERSION` — this is a bug fix.

## References

- `.beads/vb-svvr7/STATE.md`
- `.beads/vb-svvr7/codebase-map.md`
- `.beads/vb-svvr7/delivery-scope.jsonl`
- `crates/vb_cli/src/cli_postcard/validation.rs:71-101`
- `crates/vb_cli/src/cli_postcard/error.rs:7-50`
- `crates/vb_cli/src/cli_postcard/codec.rs:24-34`
- `crates/vb_cli/src/cli_postcard.rs:22-32`
- `crates/vb_cli/src/output.rs:135-147`
- `crates/vb_cli/src/cli_postcard/tests.rs:1-197`
- `verification/proptest/properties.rs:1-369`
- `crates/vb_ipc/src/frame.rs:35-51` (correct sibling for parity reference)
- AGENTS.md (God Rules 1-5)
- `.opencode/skill/rust-contract/SKILL.md`