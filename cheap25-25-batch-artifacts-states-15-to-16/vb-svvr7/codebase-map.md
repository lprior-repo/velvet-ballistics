# Codebase Map — vb-svvr7

- bead_id: vb-svvr7
- title: IPC: reject trailing bytes in CLI postcard frame decoder (P1 bug)
- description: The CLI IPC postcard frame decoder accepts trailing bytes after a valid frame. Add a strict length check and reject trailing bytes with a typed error variant.
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7
- pwd_verified: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7
- jj_root_verified: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7 (cheap25-vb-svvr7 workspace)
- upstream_main: 2c8ea33c9
- captured_at: 2026-07-01
- state: 2 explore scout, scope-first, evidence-only

## TL;DR

The decoder under audit lives in the **vb_cli crate**, NOT the **vb_ipc crate**. The bead text says "CLI IPC postcard frame decoder"; the bug is in the CLI-emit/output frame decoder wired to the CLI postcard envelope, sitting at `crates/vb_cli/src/cli_postcard/validation.rs:decode_postcard`. The vb_ipc crate at `crates/vb_ipc/src/frame.rs` is a separate (and correct: uses `!=` length check) boundary, used during socket dispatch, and is not the site of the P1 bug.

The P1 bug is one line of arithmetic in a single function plus one missing enum variant plus its Display arm. Three files in one crate are touched; one test is added.

## Bug Site (Ground-Truth Evidence)

### Primary — `crates/vb_cli/src/cli_postcard/validation.rs`

- Function: `pub(crate) fn decode_postcard(data: &[u8]) -> Result<(&[u8], &[u8]), PostcardError>` (declared at line 71, 102 lines total).
- Current length check at **line 87-89**:
  ```rust
  if data.len() < payload_end {
      return Err(PostcardError::DecodeFailed);
  }
  ```
- `payload_end = HEADER_SIZE + payload_len` (line 80-85, uses `checked_add`, sound).
- After the `<` check, `data.get(payload_start..payload_end)` (line 94-96) returns the slice, the CRC + digest are validated, and the function returns `Ok((header_bytes, payload))` at line 101 even when `data.len() > payload_end`. Trailing bytes are silently accepted.
- The function is `pub(crate)`; the public entry point is `decode_postcard_json` in `codec.rs`.

### Error enum — `crates/vb_cli/src/cli_postcard/error.rs`

- File: 50 lines, declares `pub(crate) enum PostcardError` at line 7-30 with 11 variants: `InvalidMagic`, `InvalidHeaderLength`, `PayloadTooLarge`, `VersionTooOld`, `VersionTooNew`, `WrongKind`, `DigestMismatch`, `CrcMismatch`, `PayloadMetadataMismatch`, `JsonPayloadDecodeFailed`, `DecodeFailed`.
- Display impl at line 32-48 covers all 11 variants in the same order.
- The fix needs: a new variant `TrailingBytes { trailing_len: usize }` (or carry-arity shape consistent with existing variants — existing variants are unit-only, so prefer unit variant `TrailingBytes` to match the established display style), plus a Display arm.

### Decoder-shape invariant (existing types) — `crates/vb_cli/src/cli_postcard/types.rs`

- `PostcardHeader::from_bytes` (line 105-127) populates the header struct; `validate()` (line 85-96) checks magic/header_len/payload_len bounds.
- Constants: `HEADER_SIZE = 52`, `HEADER_SIZE_U32 = 52`, `MAX_PAYLOAD = 64*1024`, `MAX_PAYLOAD_U32`, `CLI_MAGIC = b"VCLA"`, `CLI_SCHEMA_VERSION = 1`, `CLI_POSTCARD_KIND = 2`.
- `CliPostcardPayload` (line 36-41): serde-encoded inner payload with `schema_version, kind, content_type, json_utf8`.

### Encoder — `crates/vb_cli/src/cli_postcard/codec.rs`

- `decode_cli_payload` (line 8-13) - inner payload deserialization.
- `decode_postcard_json` (line 24-34) - public entry that combines header decode + payload decode + content validation + JSON parse. Calls `super::decode_postcard(data)` at line 27; this is the production consumer for the function under fix.
- `encode_postcard` (line 46-73) - emits `header_size + payload_len + payload` bytes (no trailing-bytes smuggling possible from our own encoder; the bug is on the decode side).

### Module root + re-exports — `crates/vb_cli/src/cli_postcard.rs`

- 42 lines. Re-exports the public API: `PostcardError`, `MAX_PAYLOAD_U32`, `CLI_MAGIC`, `CLI_POSTCARD_KIND`, `CLI_SCHEMA_VERSION`, `CliPostcardContentType`, `CliPostcardPayload`, `HEADER_SIZE`, `HEADER_SIZE_U32`, `MAX_PAYLOAD`, `PostcardHeader`, `decode_cli_payload`, `decode_postcard_json`, `encode_postcard`, `decode_postcard`, `payload_digest`, `validate_cli_payload`.
- The new `TrailingBytes` variant must be re-exported here.

## Consumer Surface (Call Sites)

| File:Line | Caller | Decoded as |
| --- | --- | --- |
| `crates/vb_cli/src/cli_postcard/codec.rs:24-34` | `decode_postcard_json` | Inner data path; first hostile user surface |
| `crates/vb_cli/tests/cli_integration.rs:2671-2703` (`assert_postcard_stdout`) | `decode_postcard_json(&output.stdout)` at line 2679 | CLI integration test consumer |
| `crates/vb_cli/src/output.rs:135-147` (`encode_postcard_json_frame`) | only encoder path; not a decoder caller |
| `verification/proptest/properties.rs:7-9, 38, 107, 145, 183, 212, 239, 267, 294, 320, 357` | proptest harness | Treats decoder as black box; new property must extend harness |
| `crates/vb_cli/src/lib.rs:5` | `pub mod cli_postcard;` | Module-level export |

No fuzz targets cover `vb_cli::cli_postcard`. A `vb_ipc` Kani harness module exists (`crates/vb_ipc/src/kani_ipc_*.rs`) but those target the IPC socket boundary, NOT the CLI postcard codec.

No Verus specs cover `cli_postcard`. Verified by searching `verification/verus/`.

The `vb_ipc/src/frame.rs:decode_frame_payload` function (line 35-51) uses correct `if payload.len() != expected_len` (line 44) — it does NOT have this bug, but the two boundary crates share the same naming space and may show up in same queries.

## Existing Tests (Coverage Baseline)

In `crates/vb_cli/src/cli_postcard/tests.rs` (197 lines, 11 unit tests):

| Test | Asserts |
| --- | --- |
| `test_valid_magic` | CLI_MAGIC == "VCLA" |
| `test_max_payload` | MAX_PAYLOAD == 65536 |
| `test_header_size` | HEADER_SIZE == 52 |
| `test_postcard_header_from_bytes` | Header round-trip |
| `test_decode_valid_postcard` | Happy-path decode |
| `test_decode_invalid_magic` | Bad magic -> `InvalidMagic` |
| `test_decode_payload_too_large` | Oversized payload -> `PayloadTooLarge` |
| `test_decode_invalid_header_length` | Wrong header_len -> `InvalidHeaderLength` |
| `test_decode_data_too_short` | Truncated (less than HEADER_SIZE) -> `DecodeFailed` |
| `test_encode_postcard` | Encoder shape |
| `test_roundtrip` | Encode->decode roundtrip |
| `decode_rejects_corrupted_crc_before_exposure` | CRC bit flip -> `CrcMismatch` |
| `decode_rejects_corrupted_digest_before_exposure` | Digest bit flip -> `DigestMismatch` |
| `decode_rejects_old_and_future_versions` | Version bounds |
| `decode_rejects_wrong_kind` | Wrong kind -> `WrongKind` |
| `decode_rejects_max_plus_one_payload_before_exposure` | MAX+1 payload -> `PayloadTooLarge` |
| `decode_rejects_truncated_header` | Header-1 byte -> `DecodeFailed` |

**Critical gap**: zero tests assert that valid frame + trailing bytes yields an error. This is the P1 bug and the missing test gap.

In `verification/proptest/properties.rs` (369 lines, 11 proptest groups), all properties test from `encode_postcard -> decode_postcard` using inputs shorter than, equal to, or exceeding MAX_PAYLOAD only via the encoder's reject path. None test post-decode trailing bytes.

In `crates/vb_cli/tests/cli_integration.rs` (4006 lines), the `assert_postcard_stdout` helper at line 2671-2703 invokes `cli_postcard::decode_postcard_json(&output.stdout)`; this consumes CLI output, which our own encoder emits without trailing bytes, so this path is not exposed to the bug. It would, however, correctly fail harder once we tighten decoding — only true to the strict frame boundary.

## Risk Tags

| Tag | Present? | Where |
| --- | --- | --- |
| parser/codec | YES | `validation.rs:decode_postcard` |
| temporal/concurrency | NO | Pure function over `&[u8]` |
| unsafe/UB | NO | `#![forbid(unsafe_code)]` at file head; the slice arithmetic uses `checked_add` and `get` |
| persistence | NO | No on-disk effect |
| auth/security | NO | Boundary hardening only |
| dependency | YES | `postcard`, `crc32fast`, `blake3`, `serde`, `serde_json`, `serde-saphyr` — all transitive workspace crates |
| performance | NO | Negligible cost (one compare) |
| public API | YES | `decode_postcard` and `decode_postcard_json` are the public-ABI of `vb_cli::cli_postcard` for outbound CLI consumers (e.g., shell-tooled scripts that pipe postcard-encoded CLI output to other Rust tools) |
| migration | NO | This is a bug fix, not a contract version change |
| user-visible behavior | YES | The CLI returns `PostcardFrame(PostcardError::TrailingBytes)` for malformed input; downstream CLI exit codes stay StorageError (per `output.rs:output_error_exit`) so the user-visible failure mode is preserved |

## Bead Scope (Touched Files)

Files that MUST change:

1. `crates/vb_cli/src/cli_postcard/error.rs` — add `TrailingBytes` variant + Display arm.
2. `crates/vb_cli/src/cli_postcard/validation.rs` — strengthen line 87-89 check from `<` to `!=` and return `PostcardError::TrailingBytes`.
3. `crates/vb_cli/src/cli_postcard/tests.rs` — add `decode_rejects_trailing_bytes_after_valid_frame` (and `decode_accepts_exact_length_frame` as a regression).

Files that SHOULD be reviewed for side effects (read-only):

4. `crates/vb_cli/src/cli_postcard.rs` — re-exports; if `PostcardError` is re-exported via `pub(crate) use error::PostcardError`, no change needed; if any new public-name needed for `TrailingBytes`, follow the existing `pub(crate) use error::PostcardError` pattern.
5. `crates/vb_cli/src/cli_postcard/codec.rs` — `decode_postcard_json` calls `decode_postcard`; the new error propagates through unchanged (the `?` operator handles all enum variants).
6. `crates/vb_cli/src/output.rs` — `OutputError::PostcardFrame(cli_postcard::PostcardError)` wraps the new variant without change (Display match is exhaustive on the inner enum only if downstream walks it — currently `write!(formatter, "postcard frame encoding failed: {error}")` consumes Display, so Display must be updated).
7. `verification/proptest/properties.rs` — extend `prop_roundtrip_bijectivity` (or add a new property `prop_trailing_bytes_rejected`) to assert: when valid encode is followed by an arbitrary trailing byte vector of length `[1..N]`, decode yields `Err(PostcardError::TrailingBytes)`.

Files explicitly EXCLUDED (verified by re-search):

- `crates/vb_ipc/src/frame.rs` — IPC socket frame; uses `!=` length check already.
- `crates/vb_ipc/src/kani_ipc_*.rs` — IPC harness cluster; unrelated boundary.
- `crates/vb_ui_model/src/emitter/binary/mod.rs` — different `decode_postcard` (line 166); unrelated envelope.
- `tests/` (workspace root) — only holds `tests/tooling/`, no Rust integration tests.

## Open Questions for Downstream

1. Should the `TrailingBytes` variant carry the trailing length? Existing variants are unit-shaped; carrying payload would match `IpcError::PayloadLengthMismatch { header, actual }` from `vb_ipc/src/error.rs` but break parity with the rest of `PostcardError`. Recommend unit variant unless downstream proof prefers arithmetic coupling.
2. Is `decode_postcard_json` required to surface the new error identically to existing variants, or should it mask it as `DecodeFailed`? Recommend surfacing as-is so callers can distinguish framing bugs from data corruption.
3. Should the proptest in `verification/proptest/properties.rs` add a property `prop_strict_length_no_trailing_bytes`, or is a unit test in `tests.rs` sufficient? Both are recommended — unit test locks down the variant name; proptest locks down the property over arbitrary trailing lengths.
4. Does the master plan require Miri harness since unsafe is forbidden at file head? Per AGENTS.md, Miri is used for soundness of unsafe code; this crate is `#![forbid(unsafe_code)]`, so Miri is unnecessary.
5. Is a Kani harness warranted? Per AGENTS.md GOD RULE 1, Kani harnesses must not hardcode shapes. A `#[cfg(kani)]` harness feeding `kani::any()` `Vec<u8>` would expose the decoder to bounded-model-check, but is optional given a property proptest already exists.

## Recommended Downstream Owners

| Skill | Suggested lane |
| --- | --- |
| `rust-contract` | Author a value-object/typestate spec: introduce `Frame` newtype around `&[u8; 52]` and `Payload` newtype around `&[u8; payload_len]` so illegal-length framing becomes unrepresentable. Add `ReadError` / `FrameError` totals that include `TrailingBytes`. |
| `proof-planner` | Plan bounded proptest + optional Kani harness for `decode_postcard`. Skip Miri (forbid-unsafe). Skip Verus (no production-already-bound spec exists; per AGENTS.md GOD RULE 2, vacuum Verus specs are rejected, so adding one is unjustified here). |
| `test-planner` | Add: (a) unit test `decode_rejects_trailing_bytes_after_valid_frame`; (b) unit test `decode_accepts_exact_length_frame` (no trailing bytes); (c) proptest `prop_strict_length_no_trailing_bytes`; (d) BDD scenario in `cli_postcard/tests.rs` already covers the bytes. |
| `holzman-rust` | Implementation: add 1 enum variant, 1 Display arm, replace 1 `<` with 1 `!=` check returning the new variant, add 1-2 tests. No `unwrap`/`expect`/`panic`. Use `checked_add` is already present; mirror that pattern. New `TrailingBytes` must propagate through `?` chains — verified, since all `PostcardError` variants are uniform unit shape and downstream code already uses `?`. |

## Verification Inputs (Pre-Discovery)

```bash
pwd -P                       # /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7
jj root                      # /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7
test -s .beads/vb-svvr7/STATE.md          # OK (1.2K)
test -s .beads/vb-svvr7/baseline-report.md # OK (694B)
rg -n "decode_postcard|cli_postcard" crates/vb_cli # many matches, see Consumer Surface
rg -n "TrailingBytes|trailing_bytes" .    # ZERO matches — confirms gap
```

## Anti-Hallucination Notes

- I did NOT read `velvet-ballistics-MASTER.md` end-to-end; I only confirmed no `TrailingBytes` mentions exist in it.
- I did NOT inspect `xtask/tests/bundle_tests.rs` or fuzz targets beyond the line scan.
- I did NOT verify the encode path produces no trailing bytes; if the encoder has its own latent trailing-byte bug, the unit test must encode-via-`encode_postcard` rather than fabricate `data` by hand. The existing test scaffolding at `tests.rs:179-189` (`encode_test_postcard`/`write_test_bytes`) already supports both shapes.
- Verified `vb_ipc::frame::decode_frame_payload` already uses `!=` correctly — there is no second bug to fold in.
