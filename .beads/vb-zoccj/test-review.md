# LOCAL5090 Independent Test Review: vb-zoccj

**Reviewer:** LOCAL5090 (independent, not author)
**Date:** 2026-10-21
**Bead:** vb-zoccj
**Author:** 3090

## Execution Evidence

Command: `cargo test -p velvet-ballistics-workspace-tests --test legacy_wire_decoder -- --nocapture`
Evidence: `.beads/vb-zoccj/evidence/legacy-independent.{json,stdout,stderr}`
Exit code: 0
Duration: 14.55s

```
running 5 tests
test legacy_golden_provenance_and_limitations ... ok
test legacy_golden_fixtures_have_valid_envelopes ... ok
test legacy_small_canonical_rejected_by_current_decoder ... ok
test legacy_high_values_rejected_by_current_decoder ... ok
test legacy_max_attempt_capacity_rejected_by_current_decoder ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Suite Review Gates (test-reviewer skill)

### Gate 1: Tests compile and execute deterministically
PASS. All 5 tests compile, execute, and pass on first run.

### Gate 2: Integration tests use public API only
PASS. Tests use `vb_storage::journal::parse_event` — the public decoding API.

### Gate 3: Tests assert behavior, not implementation details
PASS. Tests assert the exact error variant returned by the public API when fed old bytes, not internal struct field layouts.

### Gate 4: No ignored tests, sleeps, or hidden shared state
PASS. No `#[ignore]`, no sleeps, no shared mutable state between tests.

### Gate 5: Mutation resistance
PASS. If the current decoder were modified to accept old bytes (or return a different error variant), each of the 3 rejection tests would fail. The envelope verification test would fail if fixtures were corrupted.

### Gate 6: Snapshot tests are checked and intentional
PASS. BLAKE3 digests and lengths are verified against the manifest. The manifest itself is a checked artifact with provenance.

### Gate 7: Resource-heavy commands are bounded
PASS. These are lightweight unit-style integration tests with no timeouts needed. Execution time is 14.5s total including compile.

### Gate 8: No commented-out or dormant tests
PASS. All 5 declared tests run and pass. No filtered or ignored tests.

## Golden Fixture Authenticity

- Byte-identical comparison: current workspace fixtures match old branch via git hash
- Old producer source (99f36ba1a) confirms full envelope serialization with proper CRC/BLAKE3
- Roundtrip verification in old producer: encode → decode → exact equality before export
- Manifest provenance: old SHA 423d8ae8 explicitly declared

## Compatibility Break Statement

The test correctly establishes: **old 7-field ActionTicket layouts are rejected by the current 8-field decoder with PostcardDecodeFailed(DeserializeBadVarint)**. This is the expected fail-closed behavior, not a bug. The test does not claim universal legacy compatibility.

## Limitations

- Covers only ActionScheduledTicket event kind (record_kind 34)
- Does not cover pre-423d8ae8 layouts
- Does not cover migration/backward-decode paths
- These limitations are explicitly documented in the provenance test

## Conclusion

Test suite is sound, deterministic, and correctly scoped. Golden fixtures are authentic with verified provenance. All suite review gates pass.

**STATUS: APPROVED**
