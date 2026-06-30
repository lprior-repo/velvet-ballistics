# Fuzz Report: decode_record at 1,000,000 runs
bead_id: vb-qi37.4.2
obligation: VB-STORAGE-DECODE-006
date: 2026-05-16

## Command
SCCACHE_DISABLE=1 RUSTC_WRAPPER= cargo fuzz run decode_record --target x86_64-unknown-linux-gnu -- -runs=1000000

## Result
STATUS: PASS
Exit: 0

## Evidence
Done 1000000 runs in 3 second(s)
- Corpus: 37 entries, max entry 2166 bytes
- Coverage: 149 ft (fuzzing targets), 204 total
- No panics, no sanitizer errors, no timeouts
- libFuzzer exit: 0

## Coverage Summary
- Multiple magic types tested: MAGIC_JOURNAL_EVENT, MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT, MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE, MAGIC_INDEX_RECORD
- Both valid and invalid magic values tested
- All decode paths exercised without crash

## Conclusion
VB-STORAGE-DECODE-006 (record decode full pipeline adversarial input across 1M runs) SATISFIED.
