# LOCAL5090 Independent Code Review: vb-zoccj

**Reviewer:** LOCAL5090 (independent, not author)
**Date:** 2026-10-21
**Bead:** vb-zoccj
**Author:** 3090

## Review Target

- Test file: `crates/workspace_tests/tests/legacy_wire_decoder.rs`
- Golden fixtures: `.beads/vb-79ffw/old-golden/` (3 binary files + manifest)
- Old producer source (readonly): `99f36ba1a0f6ff6a2029061f88f9c7f5045b1da2:crates/vb_storage/tests/old_ticket_golden_export.rs`

## Provenance Verification

- Old production commit: `423d8ae8b667d82b4d36e57d29d7ef2aede08bdf`
- Old producer source commit: `99f36ba1a0f6ff6a2029061f88f9c7f5045b1da2`
- Current fixtures byte-identical to old branch via git hash comparison (c988af63, e19e406b, a7d66eda)
- Old producer serializes complete `JournalEvent::ActionScheduledTicket` via `encode_record`, not bare payload
- Old producer verified roundtrip: `encode_record` → `parse_event` → exact event equality before export
- Manifest declares exact lengths (122, 135, 128) and BLAKE3 digests for all 3 cases

## Test Architecture Assessment

The test feeds real old golden bytes through the current public `parse_event` API and asserts the exact typed rejection variant. This correctly establishes fail-closed behavior for the schema break.

## Findings

### Finding 1: No unwrap/expect/panic in test logic
All test functions use `Result`-based error propagation. No assertions that could silently pass on wrong behavior.

### Finding 2: Exact error variant assertion
Each of the 3 rejection tests asserts both:
- `JournalError::PostcardDecodeFailed` outer variant
- `postcard::Error::DeserializeBadVarint` inner variant
This is the correct behavior for the old 7-field → new 8-field layout change.

### Finding 3: Portable fixture path resolution
`repo_root()` uses `CARGO_MANIFEST_DIR` first, falls back to cwd, and validates via manifest existence check. No hardcoded absolute paths.

### Finding 4: Fixture integrity verification
`verify_fixture()` checks both BLAKE3 digest and length against manifest before decoding. This prevents stale or corrupted fixtures from passing.

### Finding 5: Proper scope and limitation documentation
`legacy_golden_provenance_and_limitations()` explicitly documents:
- ActionScheduledTicket event kind only
- Does not cover other JournalEvent variants
- Does not cover pre-423d8ae8 layouts
- Fail-closed behavior, not universal legacy compatibility

## Minor Style Observations (non-blocking)

- Lines 34, 36 use `unwrap_or(Path::new("."))` which technically violates the "no unwrap" engineering rule. These are defensive fallbacks, not panics, and do not affect test determinism or correctness.
- The test file comment mentions "schema-1 journal events" but the actual incompatibility is the ActionTicket field layout change, not schema version (which remained 1). Minor doc accuracy issue.

## Conclusion

Test architecture is sound, provenance is verified, assertions are strong and specific. No functional defects found.

**STATUS: APPROVED**
