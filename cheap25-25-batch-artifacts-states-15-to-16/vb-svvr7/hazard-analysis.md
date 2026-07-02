# Hazard Analysis — vb-svvr7

- bead_id: vb-svvr7
- date: 2026-07-01
- state: 3 rust-contract
- scope: enumerate every hazard lane relevant to the strict-length fix

## Hazard Matrix

| Lane | Present? | Severity | Where | Mitigation |
| --- | --- | --- | --- | --- |
| Parser/codec | YES | P1 (the bug) | `decode_postcard` line 87-89 | Tighten `<` to `!=`, add `TrailingBytes` variant |
| Temporal | No | n/a | Pure synchronous function | n/a |
| Concurrency | No | n/a | No shared state, no threads | n/a |
| Unsafe / UB | No | n/a | `#![forbid(unsafe_code)]`; arithmetic uses `checked_add` and `get` | n/a |
| Hostile input | YES | Medium | Boundary accepts arbitrary bytes | Strict-length check; failure mapped to typed error |
| Performance | No | n/a | One extra compare | Negligible |
| Public API | YES | Low | Additive enum variant | New variant name is unique; existing matches continue to compile if they add an arm |
| User-visible behavior | YES | Low | Downstream CLI tools see stricter decoder | Document via Display; exit code path unchanged |
| Persistence | No | n/a | In-memory only | n/a |
| Auth/security | No | n/a | No authentication | n/a |
| Migration | No | n/a | No contract version bump | n/a |
| Release / API stability | YES | Low | Additive change | Documented in this bead |

## Hazard 1: Parser/Codec (P1 — the bug)

- **Description**: `decode_postcard` rejects `data.len() < payload_end` but silently accepts `data.len() > payload_end`. A valid frame plus any number of trailing bytes is treated as `Ok`.
- **Impact**:
  - **Forensic loss**: a sender that emits a valid frame plus garbage (e.g., from a corrupt transport) is treated as a valid send. The receiver cannot detect the corruption.
  - **Streaming invisibility**: a multi-frame stream is mistakable for a single frame with garbage. Any future work to support multi-frame decoding must first close this bug.
  - **Asymmetry with `vb_ipc`**: `vb_ipc::frame::decode_frame_payload` already enforces `!=`. The two boundaries disagree on the framing contract.
- **Trigger**: any caller that passes a buffer containing exactly one valid frame followed by one or more bytes.
- **Mitigation**: replace `<` with `!=`; return `TrailingBytes` on the `>` branch; preserve `DecodeFailed` on the `<` branch.
- **Residual risk**: NONE after the fix. The new variant is the sole surface for the `>` case when earlier stages pass.
- **Proof lane**: Rust-local implementation. proptest over arbitrary trailing lengths; cargo test for unit-level invariant.

## Hazard 2: Hostile Input

- **Description**: the parser is on the receiving side of untrusted bytes (CLI stdout, IPC payloads, third-party producers).
- **Attack surfaces**:
  - **Truncated frames**: existing `<` check handles this.
  - **Trailing-byte smuggling**: the P1 bug — closed by this fix.
  - **Header tampering**: existing CRC + digest check handles this.
  - **Version/kind probing**: existing version + kind check handles this.
  - **Inner payload overflow**: `payload_len <= MAX_PAYLOAD` is enforced at S2.
  - **Allocation exhaustion**: bounded by `MAX_PAYLOAD = 65536`. INV-005 (bounded allocation).
- **Mitigation**: the fix reduces the attack surface by closing the trailing-byte vector.
- **Proof lane**: fuzz over `&[u8]` of length `[HEADER_SIZE, HEADER_SIZE + MAX_PAYLOAD + N]` to confirm no `Ok` is ever returned for `data.len() != HEADER_SIZE + payload_len` (when the frame parses successfully otherwise).

## Hazard 3: Public API Stability

- **Description**: `PostcardError` is re-exported at `vb_cli::cli_postcard::PostcardError`. Adding a variant is an additive change to a public type.
- **Compatibility analysis**:
  - **External crates consuming `Display`**: unaffected. Display is a string; the new arm contributes one more string.
  - **External crates pattern-matching `PostcardError`**: Rust's exhaustiveness checker will demand a new arm at every `match` site. The `output.rs:OutputError::PostcardFrame` does not match on the inner enum (it formats via Display), so this crate is internally safe.
  - **External crates pattern-matching `OutputError`**: `PostcardFrame(PostcardError)` is unchanged in shape; `Display` on `OutputError` is unaffected.
- **Compatibility verdict**: additive change is non-breaking for code that does not exhaustively match on the inner enum.
- **Proof lane**: cargo test (existing tests must pass); clippy (no exhaustive-match lint regressions).

## Hazard 4: User-Visible Behavior

- **Description**: downstream CLI consumers piping postcard output may now see `TrailingBytes` errors where they previously saw silent success.
- **Impact**: low — the only producers are in-process (`encode_postcard`), which never emits trailing bytes. External producers (other Rust crates, shell pipelines) may emit trailing bytes from padding or concatenation.
- **Mitigation**: the new variant carries a clear Display message ("trailing bytes after valid frame"). Operators can route this error specifically.
- **Exit code**: unchanged. `output.rs:output_error_exit` maps any `OutputError::PostcardFrame(_)` to a non-zero exit code; this path is unchanged.
- **Proof lane**: integration test in `cli_integration.rs` continues to pass because the in-process encoder emits no trailing bytes.

## Hazard 5: Performance

- **Description**: the fix adds one extra integer compare (`!=`) versus the prior single compare (`<`).
- **Impact**: sub-nanosecond. The decoder is already O(HEADER_SIZE + payload_len) on the success path; the new branch is taken once per call.
- **Mitigation**: none needed.
- **Proof lane**: criterion benchmark (out of scope; the difference is below noise floor).

## Hazard 6: Source Lint / Style

- **Description**: AGENTS.md mandates zero clippy/fmt/rustc warnings. The fix must not introduce new lints.
- **Lint concerns**:
  - **`comparison_chain`**: the `if data.len() != payload_end { if data.len() < payload_end { A } else { B } }` shape may trigger this clippy lint. The idiomatic alternative is `match data.len().cmp(&payload_end) { Less => Err(DecodeFailed), Equal => proceed, Greater => Err(TrailingBytes) }`. Either shape is acceptable; the lint is a stylistic preference, not a correctness concern.
  - **`redundant_closure`**: not applicable.
  - **`needless_return`**: not applicable; `return` keeps the early-return style consistent with the existing function.
- **Mitigation**: follow the existing `return Err(...)` style for parity. If clippy objects, switch to `match` and document in `proof-seeds.jsonl` as a follow-up.

## Hazard 7: Temporal / Concurrency / Unsafe

- **Description**: not present in this module.
- **Mitigation**: n/a.

## Hazard 8: Encoder/Decoder Asymmetry

- **Description**: the encoder emits `HEADER_SIZE + payload.len()` bytes. The decoder post-fix requires `data.len() == HEADER_SIZE + payload_len` for `Ok`. The two are symmetric: encoder output is always accepted; decoder rejects all other lengths.
- **Verification**: a property test `prop_encoder_decoder_consistency` (in `verification/proptest/properties.rs`) would assert that `encode_postcard(v, k, p).map(|b| decode_postcard(&b).is_ok())` is `true` for all valid `v, k, p`. This property is implicit in the existing `prop_roundtrip_bijectivity`; the fix strengthens it.
- **Proof lane**: proptest extension to assert `prop_strict_length_no_trailing_bytes`.

## Hazard 9: Display Arm Exhaustiveness

- **Description**: `Display` for `PostcardError` is a manual `match` over every variant. Adding a variant without updating the `Display` impl would be a compile error (non-exhaustive match).
- **Mitigation**: the fix MUST add the `Display` arm in the same commit as the variant. Verified by reading `error.rs:32-48`.

## Hazard 10: Test Stability

- **Description**: the existing 11+ tests in `tests.rs` must continue to pass.
- **Test-by-test impact**:
  - `test_decode_valid_postcard`: encode → decode. Encoded length is `HEADER_SIZE + 100`. Decode is `Ok`. The fix does not affect this.
  - `test_roundtrip`: same as above, longer payload. Unaffected.
  - `test_decode_data_too_short`: 10-byte buffer. `data.len() < HEADER_SIZE`. S0 fires; returns `DecodeFailed`. Unaffected.
  - `test_decode_rejects_truncated_header`: header-1 bytes. Same as above. Unaffected.
  - All other tests use buffers that are exactly `HEADER_SIZE + payload_len` long. Unaffected.
- **Mitigation**: the fix is conservative; no existing test relies on the silent trailing-bytes acceptance.

## Cumulative Severity Post-Fix

After the fix, the only material hazard remaining in this module is the absence of a `Frame` newtype that would make the bug structurally impossible. That is a type-design improvement, not a behavior bug, and is out of scope for this bead.

## Open Hazard Questions

1. Should `TrailingBytes` carry the trailing length for diagnostic logging? Currently no (unit shape for parity). If a future need arises, a new variant `TrailingBytes { count: usize }` could be added without breaking the existing `TrailingBytes` arm because `match` would now need two arms or a wildcard. Out of scope today.
2. Should the encoder be re-tested with a property that asserts no trailing bytes are emitted? This would close hazard 8 explicitly. Recommended as a follow-up bead.

## Hazard Closure Checklist

- [x] Bug identified (parser/codec P1).
- [x] Bug fix specified (replace `<` with `!=`; add `TrailingBytes`; add Display arm).
- [x] No new concurrency / temporal / unsafe hazards introduced.
- [x] Public-API impact limited to additive variant.
- [x] User-visible behavior preserved for in-process encoder output.
- [x] Source lint exposure documented.
- [x] Test stability verified by reading each existing test.