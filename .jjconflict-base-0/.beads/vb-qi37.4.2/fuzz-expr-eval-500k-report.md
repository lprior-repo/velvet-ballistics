# Fuzz Report: expr_eval at 500,000 runs
bead_id: vb-qi37.4.2
obligation: VB-EXPR-003
date: 2026-05-16

## Command
SCCACHE_DISABLE=1 RUSTC_WRAPPER= cargo fuzz run expr_eval --target x86_64-unknown-linux-gnu -- -runs=500000

## Result
STATUS: PASS
Exit: 0

## Evidence
Done 500000 runs in 2 second(s)
- Corpus: 101 entries, max entry 2264 bytes
- Coverage: 372 ft (fuzzing targets), 418 total
- No panics, no sanitizer errors, no timeouts
- libFuzzer exit: 0

## Coverage Summary
- New functions discovered during fuzzing
- Recommended dictionary generated
- All runs completed without crash or abort

## Conclusion
VB-EXPR-003 (expression stack f64 NaN/infinity handling across 500k arbitrary inputs) SATISFIED.
