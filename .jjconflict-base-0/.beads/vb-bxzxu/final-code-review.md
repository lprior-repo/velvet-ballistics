# Final Code Review — scripts/check-test-integrity.rs

**Reviewer:** Local 3090 agent (independent review)
**Bead:** vb-bxzxu
**Artifact:** Current checker at blob `d2a64ab1a4d8ded0bf005dc0704a8dbe815a2721`
**Base for comparison:** Original main `da633fe0bbde9e414173d5f7aabe314a37f8752d`
**Prior review:** `.beads/vb-bxzxu/hash-binding-draft-review.md` (LOCAL5090)

## Verdict: APPROVE

## Verification Commands Run

All via official runner `python3 /tmp/vb-run-gate.py`:

### 1. Self-Test
```bash
python3 /tmp/vb-run-gate.py 180 .beads/vb-bxzxu/evidence/final-selftest \
    bash scripts/check-test-integrity.sh --self-test
```
**Result:** exit 0, 2.4s. All 58 assertions passed (40 main + 18 bypass/lifecycle).
Evidence: `.beads/vb-bxzxu/evidence/final-selftest.stdout`

Key self-test outcomes:
- All basic finding types detected (DeletedTestFile, DeletedTestDeclaration,
  WeakenedAssertion, IgnoredOrSkippedTest, NonUtf8TestFile, etc.)
- Rename scenarios: exact move passes, modified-retained passes, weakened fails
- **Bypass probes (prior review findings):**
  - `bypass-unrelated`: PASS (exit 1) — new_path pointing to unrelated file rejected
  - `bypass-absent`: PASS (exit 1) — ABSENT while old file exists rejected
  - `bypass-changed`: PASS (exit 1) — subsequent change after justification caught
- Justification lifecycle: stale base ignored, wrong base rejected, malformed rejected
- Files with spaces in names handled correctly

### 2. Real Base Scan
```bash
python3 /tmp/vb-run-gate.py 180 .beads/vb-bxzxu/evidence/final-base-scan \
    bash scripts/check-test-integrity.sh --base da633fe0bbde9e414173d5f7aabe314a37f8752d
```
**Result:** exit 1, 9.1s. 35 findings reported (down from ~373 in parent draft).
Evidence: `.beads/vb-bxzxu/evidence/final-base-scan.stderr`

Finding breakdown:
- IgnoredOrSkippedTest: 6
- WeakenedAssertion: 20
- DeletedTestFile: 2
- DeletedTestDeclaration: 7

This matches the author's handoff report exactly. Findings are honest raw
output — no justifications have been written yet, so no findings are suppressed.

## Prior Review Bypass Probes

The LOCAL5090 prior review identified two hash-binding bypasses:
1. `new_path` pointing to an unrelated file
2. `new_path=ABSENT` while old file still exists

These are now covered by the self-test's `run_justification_bypass_tests()`
function (lines 2241-2334 of the source), which uses the actual scanner on real
temp repos (not copied logic). All three bypass scenarios pass.

## Replay Formatting Changes

Verified that the three replay formatting changes (from commit 50cd13426)
preserve parsed tokens modulo rustfmt closure formatting and import ordering:

1. `crates/vb_storage/src/recovery/replay/core/full.rs` — closure formatting only
2. `crates/workspace_tests/tests/vb_ko29_5_fixture/mod.rs` — import ordering,
   closure formatting, line wrapping, trailing newline
3. `crates/workspace_tests/tests/vb_ko29_5_public_idempotency.rs` — import
   ordering, closure formatting, assert_eq! line wrapping

All changes are purely cosmetic; no semantic content altered.

## Edge Cases Verified by Self-Test

- Binary files: unrelated binary passes; deleted binary with tests fails
- Non-UTF8: relevant non-UTF8 test files fail; binary files skipped
- NUL bytes: fail-closed
- Files with spaces: handled correctly (both delete and rename)
- Renamed files: exact byte move no finding; modified move diffs real hunks
- Comments with `assert`: not skipped incorrectly (tested via weaken scenario)

## Known Limitations

- `scripts/check-test-integrity.rs` self-flags (its own self-test fixtures
  contain `assert_eq!`/`#[test]` substrings)
- Some `WeakenedAssertion` rows with `removed_exact=0 added_weak>0` are from
  new files adding weak assertions (pre-existing heuristic)
- No justifications written yet; the 35 findings are honest raw output

## Conclusion

The current checker at blob `d2a64ab1a4d8ded0bf005dc0704a8dbe815a2721` passes
all self-tests, correctly reports 35 findings against the original main base,
and fixes the two hash-binding bypasses identified in the prior LOCAL5090
review. The self-test exercises the actual scanner on real temp repos, not
copied logic. All edge cases are verified.

**APPROVE for publication.**