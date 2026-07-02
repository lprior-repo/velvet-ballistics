# Codebase Map — vb-1wora (Codec: reject trailing bytes after declared record payload)

Bead ID: `vb-1wora`
Title: Codec: reject trailing bytes after declared record payload (P1 bug)
Captured: 2026-07-01 (State 2 / explore dispatch from femdation)
Workspace: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`
JJ root verified: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`
Git root: workspace is JJ-initialized, no `.git/` dir present.

## Bug Location

The v1 storage record codec in `vb_storage` decodes an envelope (60-byte header)
plus a payload, but does **not** verify that the input slice ends exactly at the
declared payload boundary. After the fix succeeds the decode must fail closed
with a new `TrailingBytes` variant when `bytes.len() > payload_end`.

Primary culprit: `decode_record_payload` in
`crates/vb_storage/src/codec/payload.rs:56-82` — computes
`payload_start..payload_end` via `bytes.get(...)` and silently ignores any tail.

Secondary culprit: `decode_envelope_only` in
`crates/vb_storage/src/codec/envelope.rs:48-83` — same shape, same bug, marked
`#[allow(dead_code, reason = "inspection-only entry point retained for
doctor/filtering workflows")]`. No production callers today, but the surface is
public to the crate and must agree with `decode_record` to avoid divergent
semantics.

Public façade: `decode_record` (and `decode_journal_event`) in
`crates/vb_storage/src/codec/mod.rs:82-151`. The façade calls
`decode_record_payload` and then `postcard::from_bytes(payload)`. The trailing-
bytes check belongs **before** `postcard::from_bytes`, because after that point
the payload slice has already been borrowed for deserialization.

## Key Symbols (production)

| Symbol | Path | Role |
|---|---|---|
| `decode_record<T>` | `crates/vb_storage/src/codec/mod.rs:82-95` | Public typed envelope+payload decode |
| `decode_journal_event` | `crates/vb_storage/src/codec/mod.rs:126-151` | Journal-specific decode + parity + envelope-seq check |
| `decode_record_payload` | `crates/vb_storage/src/codec/payload.rs:56-82` | Envelope+payload header/CRC/digest gatekeeper (BUG SITE) |
| `decode_envelope_only` | `crates/vb_storage/src/codec/envelope.rs:48-83` | Header-only decode for doctor/filter (BUG SITE, mirror) |
| `decode_record_header` | `crates/vb_storage/src/codec/header.rs:26-58` | Validates the 60-byte header |
| `verify_digest_match` | `crates/vb_storage/src/codec/payload.rs:9-18` | BLAKE3 digest check |
| `payload_len_u32` | `crates/vb_storage/src/codec/payload.rs:20-32` | Length-bounds check (helper) |
| `JournalError` enum | `crates/vb_storage/src/error/mod.rs:20-188` | Where `TrailingBytes` must be added |
| `JournalError::diagnostic_code` | `crates/vb_storage/src/error/codes.rs:99-176` | Numeric mapping (`*_CODE` constants) |
| `JournalError::symbolic_code` | `crates/vb_storage/src/error/codes.rs:180-268` | Symbolic name mapping |
| `MAGIC_JOURNAL_EVENT` (and 5 siblings) | `crates/vb_storage/src/constants.rs:60-72` | Record-kind magics used by tests |
| `RECORD_HEADER_BYTES = 60` | `crates/vb_storage/src/constants.rs:84` | Header size constant |

## Existing Tests Touched by the Bug

These tests currently document the *buggy* behavior. After the fix they must
flip polarity or be replaced.

| Test | Path | Current behavior | Required change |
|---|---|---|---|
| `decode_ignores_trailing_bytes_beyond_payload` | `crates/vb_storage/src/codec/tests.rs:1498-1524` | Asserts trailing bytes (0xFF 0xFE 0xFD) appended after a valid record decode successfully. | Rename to `decode_rejects_trailing_bytes_after_payload` (or similar). Assert `Err(JournalError::TrailingBytes { trailing })` instead of `Ok`. Keep the same input fixture. |
| `decode_envelope_only_rejects_truncated_payload` | `crates/vb_storage/src/codec/envelope.rs:153-170` | Sibling test asserting `UnexpectedEof` on truncation. | Add `decode_envelope_only_rejects_trailing_payload` next to it asserting the new error variant. |

## Existing Tests That Depend on `decode_record` (no behavior change expected, but they will exercise the new variant on round-trips)

Tests that **round-trip** a freshly `encode_record`-d buffer continue to pass
unchanged — the encoder never produces trailing bytes. Tests that exercise
corruption or fuzz-driven shapes should be reviewed for any that intentionally
append junk and rely on silent ignore.

Files containing `decode_record` / `decode_journal_event` calls (test-only or
production):

- `crates/vb_storage/src/codec/tests.rs` (heavy direct use, ~50+ tests)
- `crates/vb_storage/src/tests.rs` (100+ tests; large error-taxonomy coverage)
- `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs`
- `crates/vb_storage/src/security_tests.rs`
- `crates/vb_storage/src/trimming/tests.rs` (comment-only ref)
- `crates/vb_storage/src/test_helpers.rs` (re-exports)
- `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs`
- `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs`
- `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs`
- `crates/workspace_tests/tests/restate_decode_error_taxonomy_tests.rs`
- `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs`
- `crates/workspace_tests/benches/velvet_ballistics.rs` (perf benches)

Production callers (post-fix, these will return `Err(TrailingBytes)` instead of
silently accepting malformed records — a fail-closed hardening):

- `crates/vb_storage/src/trimming/logic.rs:251` — `has_terminal_event` reads
  Fjall keyspace values via `decode_journal_event`. Today this can be silently
  fed extra bytes by a corrupted row; after the fix the loop exits with
  `TrimError::Journal(TrailingBytes)`. Existing test
  `trimming/tests.rs:478` is comment-only reference, not affected.

No other production callers of `decode_record` / `decode_journal_event` /
`decode_record_payload` were located outside the codec module and trimming.

## Verification Artifacts (WEAK binding via `production_inner` mirror)

| Artifact | Path | Role |
|---|---|---|
| Verus spec | `verification/verus/vb-vzcuf-PS-003.rs:387-451` | `assume_specification[ production::decode_record ]` enumerates every `SpecJournalError` arm reachable from the production fn. Currently lists: `BadMagic`, `RecordKindFamilyMismatch`, `UnsupportedSchemaVersion`, `MigrationRequired`, `UnknownRecordKind`, `HeaderLengthMismatch`, `HeaderChecksumMismatch`, `PayloadDigestMismatch`, `UnexpectedEof`, `PayloadTooLarge`, `PostcardDecodeFailed`, `RecordKindPayloadMismatch`, `InvalidEvent`. **Missing**: `TrailingBytes`. |
| Mirror file (drift-tracked) | `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:335-413` | `enum SpecJournalError` enumerates the reachable variants. Comments at L280-327 enumerate the production mirror rationale. **Missing**: `TrailingBytes { trailing: u32 }` (or unit, TBD by planner). |
| Extern shim | `verification/verus/extern_vb_vzcuf_PS_003.rs:83-87` | Re-exports `SpecJournalError` from the mirror. Likely no change required — re-export already covers whatever variants the mirror adds. |
| Drift gate | `scripts/check-production-inner-drift.sh`, `scripts/check-verus-production-binding.sh` | Must pass after edits; mirror must keep production parity. |
| Kani harness | `crates/vb_storage/src/kani_postcard_envelope_wire.rs:307` | H5 calls `decode_record_payload` with mismatched payload. Could optionally add a H6 harness covering the trailing-bytes path. Out of scope unless the planner asks for proof evidence. |

## Diagnostic Code Surface (must add a new numeric constant)

Numeric codes in `crates/vb_storage/src/error/codes.rs` are sequential 0x4001–
0x4041. Next free slot in the journal numeric range that does not collide with
`crates/vb_core/src/diagnostic.rs` `CODE_REGISTRY` (registry stops at 0x4032):

- `0x4042` (next free) — recommended. The numeric constants 0x4033, 0x4034,
  0x4040, 0x4041 are defined in `error/codes.rs` but are **not** registered in
  the symbolic `CODE_REGISTRY` (so they fall back to `INTERNAL_INVARIANT` for
  `symbolic_code()`). Adding a new numeric constant follows the same pattern:
  optional symbolic registration in `CODE_REGISTRY` (cleaner, recommended).

Symbolic name conventions (existing journal entries follow
`JOURNAL_<DOMAIN>_<SHORT>`):

- `JOURNAL_TRAILING_BYTES` (recommended)
- diagnostic code constant naming follows `<VARIANT_SNAKE>_CODE`; e.g.
  `TRAILING_BYTES_CODE: DiagnosticCode = DiagnosticCode::new(0x4042);`

## Reference Docs / Patterns

- Variant registration pattern:
  `crates/vb_storage/src/error/mod.rs:96-97` (`UnexpectedEof`) is the
  closest sibling (unit variant, "excess/length" semantic). Its pattern is the
  minimum-fuss template for a unit-style `TrailingBytes { trailing: usize }`.
- Diagnostic-code + symbolic-code pair pattern:
  `crates/vb_storage/src/error/codes.rs:49,127,218` for `UNEXPECTED_EOF_CODE`,
  `diagnostic_code`, and `symbolic_code` arms.
- Test pattern for variant / display / code trio:
  `crates/vb_storage/src/error_tests.rs:454-512` (`InvalidGateCount`) and
  `crates/vb_storage/src/error_tests.rs:513-557` (`MissingRequiredProofFlag`).
- Numeric-code `is_correct` test pattern:
  `crates/vb_storage/src/error_code_tests.rs:144-160` (`payload_too_large`).

## Open Questions for the Planner

1. **Variant shape.** `TrailingBytes` (unit), `TrailingBytes { trailing: u32 }`
   (count only), or `TrailingBytes { trailing: usize, declared_payload_len: u32 }`
   (fuller diagnostics)? The diagnostic-code registry / symbolic-name registry
   path is the same regardless. `trailing: usize` mirrors the
   `MalformedKeyspaceRow` precedent at `error/mod.rs:97-105`.

2. **Numeric code value.** Recommended `0x4042` (next sequential free slot in
   the `0x40xx` journal range). If symbolic registration in `CODE_REGISTRY` is
   desired, that goes in `crates/vb_core/src/diagnostic.rs` (and
   `diagnostic.rs:1583` is the end of the registry slice).

3. **Should `decode_envelope_only` get the same fix?** Recommended yes, for
   consistency. It's not on a hot production path (only test callers) but the
   function is `pub(crate)` and its docstring claims to perform "envelope +
   payload" decode. The bug is identical.

4. **Should the check happen in `decode_record_payload` (one canonical site)
   or be repeated in `decode_record` / `decode_journal_event`?** Recommended
   canonical site: `decode_record_payload`. `decode_record` already delegates
   to it; `decode_envelope_only` can either reimplement the check inline or
   also delegate to a new `decode_payload_only_with_trailing_check` helper.

5. **Fuzz target update?** Optional. `fuzz/fuzz_targets/fuzz_storage_codec_payload_corruption.rs`
   and `fuzz_storage_codec_roundtrip.rs` will already exercise the new path
   (they feed random bytes to `decode_record`). A targeted "append N junk bytes
   after a valid record" oracle would be additive, not mandatory.

6. **Verus PS-003 spec binding.** Mandatory update. The spec must add
   `Err(SpecJournalError::TrailingBytes { .. })` to the `decode_record`
   `ensures` postcondition so the bridge contract matches production.

7. **Kani harness H6?** Optional. Could add a `kani_harness_rejects_trailing_bytes`
   to `kani_postcard_envelope_wire.rs` mirroring H5, but no existing obligation
   requires it; recommend leaving to the proof-planner.

8. **Should the check fire *before* or *after* the BLAKE3 digest check?**
   Recommended: **before** digest check. Digest is the expensive op; rejecting
   shape defects first is the cheap path and matches the
   "decode order = cheap → expensive" convention codified in
   `kani_postcard_envelope_wire.rs:1-11`. The current order is
   `header → slice → digest → postcard`; new order is
   `header → slice → trailing-bytes → digest → postcard`.

## Risks

- **R1 (HIGH) — silent acceptance of corrupted records is a security-relevant
  P1 bug.** Any production path that fed a malformed record (e.g. an
  attacker-crafted byte appended to a Fjall keyspace value) currently decodes
  the prefix and silently drops the tail. Fixing this is fail-closed and
  mandatory.
- **R2 (MED) — Verus PS-003 binding drift.** Adding a new variant without
  updating the Verus spec and mirror breaks the production-binding gate. The
  `extern_vb_vzcuf_PS_003.rs` re-export pattern will pick up the new variant
  automatically, but the spec contract must add the new arm.
- **R3 (LOW) — Test `decode_ignores_trailing_bytes_beyond_payload` will start
  failing immediately after the fix.** It must be inverted (renamed + re-
  asserted) in the same change.
- **R4 (LOW) — Diagnostic code numeric collision.** A free slot (`0x4042`)
  exists; reviewer should confirm no other branch / WIP bead is using it.
- **R5 (LOW) — `decode_envelope_only` diverges from `decode_record` if the
  fix is applied to only one site.** Recommend applying to both for symmetry.
